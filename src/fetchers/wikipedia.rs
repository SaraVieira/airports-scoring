use std::sync::LazyLock;

use anyhow::{Context, Result};
use regex::Regex;
use serde::Deserialize;
use serde_json::Value;
use sqlx::PgPool;
use tracing::info;

use crate::models::{Airport, FetchResult};

pub const USER_AGENT: &str = "AirportIntelligencePlatform/1.0";

// Pre-compiled regexes used in strip_wiki_markup (called many times per article).
static RE_REF: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"<ref[^>]*>.*?</ref>|<ref[^/]*/?>").unwrap()
});
static RE_WIKILINK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\[\[(?:[^|\]]*\|)?([^\]]*)\]\]").unwrap()
});
static RE_TEMPLATE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\{\{[^}]*\}\}").unwrap()
});
static RE_HTML_TAG: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"<[^>]+>").unwrap()
});
static RE_YEAR: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(19\d{2}|20\d{2})\b").unwrap()
});
static RE_NUMBER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[\d,]+").unwrap()
});
static RE_SKYTRAX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(19\d{2}|20\d{2})\D{0,30}(\d)\s*-?\s*star|(\d)\s*-?\s*star\D{0,30}(19\d{2}|20\d{2})").unwrap()
});

fn article_title_from_url(url: &str) -> Option<&str> {
    url.rsplit_once("/wiki/").map(|(_, title)| title)
}

#[derive(Debug, Deserialize)]
struct WikiParseResponse {
    parse: Option<WikiParse>,
}

#[derive(Debug, Deserialize)]
struct WikiParse {
    wikitext: Option<WikiText>,
}

#[derive(Debug, Deserialize)]
struct WikiText {
    #[serde(rename = "*")]
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WikiQueryResponse {
    query: Option<WikiQuery>,
}

#[derive(Debug, Deserialize)]
struct WikiQuery {
    pages: Option<Value>,
}

/// Fetch Wikipedia data for an airport: infobox fields, passenger tables, and notes.
/// Stores results in `wikipedia_snapshots` and `pax_yearly` tables.
pub async fn fetch(pool: &PgPool, airport: &Airport, full_refresh: bool) -> Result<FetchResult> {
    let iata = airport
        .iata_code
        .as_deref()
        .unwrap_or("???");

    let wiki_url = match airport.wikipedia_url.as_deref() {
        Some(url) if !url.is_empty() => url,
        _ => {
            info!(airport = iata, "No wikipedia_url set, skipping");
            return Ok(FetchResult {
                records_processed: 0,
                last_record_date: None,
            });
        }
    };

    let title = article_title_from_url(wiki_url)
        .context("Could not extract article title from wikipedia_url")?;

    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .build()?;

    let revision_id = fetch_revision_id(&client, title).await?;

    // Skip if article hasn't changed since our last fetch.
    if !full_refresh {
        if let Some(rev) = revision_id {
            let last_rev: Option<(Option<i64>,)> = sqlx::query_as(
                "SELECT article_revision_id FROM wikipedia_snapshots \
                 WHERE airport_id = $1 ORDER BY fetched_at DESC LIMIT 1",
            )
            .bind(airport.id)
            .fetch_optional(pool)
            .await?;

            if let Some((Some(stored_rev),)) = last_rev {
                if stored_rev == rev {
                    info!(airport = iata, revision = rev, "Wikipedia article unchanged, skipping");
                    return Ok(FetchResult {
                        records_processed: 0,
                        last_record_date: None,
                    });
                }
            }
        }
    }

    let wikitext = fetch_wikitext(&client, title).await?;

    // Extract opened year from the RAW infobox value so that templates like
    // {{Start date|2020|10|31|df=y}} are not stripped before we can grab the year.
    let opened_year = parse_infobox_field_raw(&wikitext, "opened")
        .and_then(|s| extract_year(&s));
    let operator_raw = parse_infobox_field(&wikitext, "operator")
        .or_else(|| parse_infobox_field(&wikitext, "owner-oper"))
        .or_else(|| parse_infobox_field(&wikitext, "owner_oper"));
    let owner_raw = parse_infobox_field(&wikitext, "owner")
        .or_else(|| parse_infobox_field(&wikitext, "owner-oper"))
        .or_else(|| parse_infobox_field(&wikitext, "owner_oper"));
    let terminal_count = parse_infobox_field(&wikitext, "terminals")
        .or_else(|| parse_infobox_field(&wikitext, "terminal_count"))
        .or_else(|| parse_infobox_field(&wikitext, "num_terminals"))
        .and_then(|s| s.trim().parse::<i16>().ok());

    // Parse stat* infobox fields for current-year passenger/movement data
    let infobox_stats = parse_infobox_stats(&wikitext);
    if let Some(ref stats) = infobox_stats {
        if let Some(stat_year) = stats.year {
            // Upsert infobox stat data into pax_yearly
            sqlx::query(
                "INSERT INTO pax_yearly (airport_id, year, total_pax, domestic_pax, international_pax, aircraft_movements, source)
                 VALUES ($1, $2, $3, $4, $5, $6, 'wikipedia_infobox')
                 ON CONFLICT (airport_id, year) DO UPDATE SET
                     total_pax          = COALESCE(EXCLUDED.total_pax, pax_yearly.total_pax),
                     domestic_pax       = COALESCE(EXCLUDED.domestic_pax, pax_yearly.domestic_pax),
                     international_pax  = COALESCE(EXCLUDED.international_pax, pax_yearly.international_pax),
                     aircraft_movements = COALESCE(EXCLUDED.aircraft_movements, pax_yearly.aircraft_movements),
                     source             = CASE WHEN pax_yearly.source IS NULL THEN 'wikipedia_infobox' ELSE pax_yearly.source END",
            )
            .bind(airport.id)
            .bind(stat_year)
            .bind(stats.total_pax)
            .bind(stats.domestic_pax)
            .bind(stats.international_pax)
            .bind(stats.aircraft_movements)
            .execute(pool)
            .await
            .with_context(|| format!("Failed to upsert infobox stats for {} year {}", iata, stat_year))?;

            info!(
                airport = iata,
                year = stat_year,
                total_pax = stats.total_pax,
                aircraft_movements = stats.aircraft_movements,
                "Parsed infobox stat fields"
            );
        }
    }

    let pax_rows = parse_passenger_table(&wikitext);
    let mut pax_count: i32 = 0;

    for (year, total_pax) in &pax_rows {
        sqlx::query(
            "INSERT INTO pax_yearly (airport_id, year, total_pax, source)
             VALUES ($1, $2, $3, 'wikipedia')
             ON CONFLICT (airport_id, year) DO UPDATE SET
                 total_pax = COALESCE(EXCLUDED.total_pax, pax_yearly.total_pax)",
        )
        .bind(airport.id)
        .bind(*year)
        .bind(*total_pax)
        .execute(pool)
        .await
        .with_context(|| format!("Failed to upsert pax_yearly for {} year {}", iata, year))?;

        pax_count += 1;
    }

    let renovation_notes = extract_section_text(&wikitext, &["expansion", "renovation", "development", "construction"]);
    let ownership_notes = extract_section_text(&wikitext, &["ownership", "privatisation", "privatization", "shareholders"]);
    let milestone_notes = extract_section_text(&wikitext, &["history", "milestone", "timeline"]);
    let skytrax_history = extract_skytrax_history(&wikitext);

    // Fetch ACI ASQ awards from the dedicated Wikipedia article.
    // Search by airport name, city, and short name for better matching.
    let search_name = format!(
        "{} {} {}",
        airport.name,
        airport.city,
        airport.short_name.as_deref().unwrap_or("")
    );
    let aci_awards = fetch_aci_awards(&client, &search_name).await;
    if aci_awards.is_some() {
        info!(airport = iata, "Extracted ACI ASQ awards");
    }

    sqlx::query(
        "INSERT INTO wikipedia_snapshots
         (airport_id, opened_year, operator_raw, owner_raw, terminal_count,
          renovation_notes, ownership_notes, milestone_notes,
          skytrax_history, aci_awards, wikipedia_url, article_revision_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
    )
    .bind(airport.id)
    .bind(opened_year)
    .bind(operator_raw.as_deref())
    .bind(owner_raw.as_deref())
    .bind(terminal_count)
    .bind(renovation_notes.as_deref())
    .bind(ownership_notes.as_deref())
    .bind(milestone_notes.as_deref())
    .bind(skytrax_history.as_ref().and_then(|v| serde_json::to_value(v).ok()))
    .bind(aci_awards.as_ref().and_then(|v| serde_json::to_value(v).ok()))
    .bind(wiki_url)
    .bind(revision_id)
    .execute(pool)
    .await
    .context("Failed to insert wikipedia_snapshot")?;

    // Backfill airports table with parsed Wikipedia data
    sqlx::query(
        r#"
        UPDATE airports
        SET opened_year    = COALESCE($2, airports.opened_year),
            terminal_count = COALESCE($3, airports.terminal_count)
        WHERE id = $1
        "#,
    )
    .bind(airport.id)
    .bind(opened_year)
    .bind(terminal_count)
    .execute(pool)
    .await
    .context("Failed to update airports with Wikipedia data")?;

    // Try to match operator_raw against organisations and set operator_id
    if let Some(ref op) = operator_raw {
        let org_id: Option<(i32,)> = sqlx::query_as(
            "SELECT id FROM organisations WHERE name ILIKE $1 OR short_name ILIKE $1 LIMIT 1",
        )
        .bind(op)
        .fetch_optional(pool)
        .await?;

        if let Some((oid,)) = org_id {
            sqlx::query("UPDATE airports SET operator_id = $1 WHERE id = $2 AND operator_id IS NULL")
                .bind(oid)
                .bind(airport.id)
                .execute(pool)
                .await?;
        }
    }

    // Try to match owner_raw against organisations and set owner_id
    if let Some(ref ow) = owner_raw {
        let org_id: Option<(i32,)> = sqlx::query_as(
            "SELECT id FROM organisations WHERE name ILIKE $1 OR short_name ILIKE $1 LIMIT 1",
        )
        .bind(ow)
        .fetch_optional(pool)
        .await?;

        if let Some((oid,)) = org_id {
            sqlx::query("UPDATE airports SET owner_id = $1 WHERE id = $2 AND owner_id IS NULL")
                .bind(oid)
                .bind(airport.id)
                .execute(pool)
                .await?;
        }
    }

    let total = pax_count + 1;
    info!(
        airport = iata,
        pax_years = pax_count,
        revision = revision_id.unwrap_or(0),
        "Wikipedia fetch complete"
    );

    Ok(FetchResult {
        records_processed: total,
        last_record_date: None,
    })
}

async fn fetch_revision_id(client: &reqwest::Client, title: &str) -> Result<Option<i64>> {
    let url = format!(
        "https://en.wikipedia.org/w/api.php?action=query&titles={}&prop=revisions&rvprop=ids&format=json",
        title
    );
    let resp: WikiQueryResponse = client.get(&url).send().await?.json().await?;

    if let Some(query) = resp.query {
        if let Some(pages) = query.pages {
            if let Some(obj) = pages.as_object() {
                for (_page_id, page) in obj {
                    if let Some(revisions) = page.get("revisions") {
                        if let Some(rev) = revisions.as_array().and_then(|a| a.first()) {
                            if let Some(revid) = rev.get("revid").and_then(|v| v.as_i64()) {
                                return Ok(Some(revid));
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(None)
}

async fn fetch_wikitext(client: &reqwest::Client, title: &str) -> Result<String> {
    let url = format!(
        "https://en.wikipedia.org/w/api.php?action=parse&page={}&prop=wikitext&format=json",
        title
    );
    let resp: WikiParseResponse = client.get(&url).send().await?.json().await?;

    resp.parse
        .and_then(|p| p.wikitext)
        .and_then(|w| w.content)
        .context("No wikitext returned from Wikipedia API")
}

/// Extract the raw (unstripped) value of an infobox field.
/// This scans only inside the first `{{Infobox airport` block to avoid
/// false-positive matches from other templates or article body text.
fn parse_infobox_field_raw(wikitext: &str, field: &str) -> Option<String> {
    let infobox = extract_infobox_block(wikitext)?;
    let pattern = format!(r"(?mi)^\|\s*{}\s*=\s*(.+)$", regex::escape(field));
    let re = Regex::new(&pattern).ok()?;
    re.captures(&infobox)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().trim().to_string())
}

fn parse_infobox_field(wikitext: &str, field: &str) -> Option<String> {
    let raw = parse_infobox_field_raw(wikitext, field)?;
    let stripped = strip_wiki_markup(&raw);
    if stripped.is_empty() { None } else { Some(stripped) }
}

/// Extract the `{{Infobox airport ... }}` block from wikitext.
/// Handles nested `{{ }}` pairs so we find the correct closing `}}`.
fn extract_infobox_block(wikitext: &str) -> Option<String> {
    let lower = wikitext.to_lowercase();
    let start = lower.find("{{infobox airport")?;
    let bytes = wikitext.as_bytes();
    let mut depth = 0u32;
    let mut i = start;
    while i < bytes.len() - 1 {
        if bytes[i] == b'{' && bytes[i + 1] == b'{' {
            depth += 1;
            i += 2;
        } else if bytes[i] == b'}' && bytes[i + 1] == b'}' {
            depth -= 1;
            if depth == 0 {
                return Some(wikitext[start..i + 2].to_string());
            }
            i += 2;
        } else {
            i += 1;
        }
    }
    // If we never closed, return everything from start (best effort).
    Some(wikitext[start..].to_string())
}

fn extract_year(s: &str) -> Option<i16> {
    RE_YEAR.captures(s)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse::<i16>().ok())
}

fn strip_wiki_markup(text: &str) -> String {
    let s = RE_REF.replace_all(text, "");
    let s = RE_WIKILINK.replace_all(&s, "$1");
    let s = RE_TEMPLATE.replace_all(&s, "");
    let s = RE_HTML_TAG.replace_all(&s, "");
    s.trim().to_string()
}

/// Parsed stat* infobox fields.
#[derive(Debug, Default)]
struct InfoboxStats {
    year: Option<i16>,
    total_pax: Option<i64>,
    domestic_pax: Option<i64>,
    international_pax: Option<i64>,
    aircraft_movements: Option<i32>,
}

/// Parse `stat-year`, `stat{N}-header`, and `stat{N}-data` fields from the infobox.
fn parse_infobox_stats(wikitext: &str) -> Option<InfoboxStats> {
    let mut stats = InfoboxStats::default();

    // Extract stat-year
    let year_raw = parse_infobox_field_raw(wikitext, "stat-year")
        .or_else(|| parse_infobox_field_raw(wikitext, "stat_year"))?;
    let year_stripped = strip_wiki_markup(&year_raw);
    // Remove non-digit chars and parse
    let year_digits: String = year_stripped.chars().filter(|c| c.is_ascii_digit()).collect();
    let year = year_digits.parse::<i16>().ok()?;
    if !(1950..=2030).contains(&year) {
        return None;
    }
    stats.year = Some(year);

    // Scan stat1 through stat9
    for n in 1..=9 {
        let header_field = format!("stat{}-header", n);
        let data_field = format!("stat{}-data", n);

        let header = match parse_infobox_field_raw(wikitext, &header_field) {
            Some(h) => strip_wiki_markup(&h).to_lowercase(),
            None => continue,
        };

        let data_raw = match parse_infobox_field_raw(wikitext, &data_field) {
            Some(d) => d,
            None => continue,
        };

        // Strip wiki markup (removes {{increase}}, {{decrease}}, {{steady}}, refs, etc.)
        let data_clean = strip_wiki_markup(&data_raw);
        // Take only the first number token (before any percentage or text suffix)
        // e.g. "41,560,000 12.2%" → "41,560,000"
        let first_token = data_clean
            .split_whitespace()
            .next()
            .unwrap_or("");
        let digits: String = first_token.chars().filter(|c| c.is_ascii_digit()).collect();
        if digits.is_empty() {
            continue;
        }

        // Skip percentage/change headers — they contain "passenger" but are not counts
        if header.contains("change") || header.contains("growth") || header.contains("%") {
            continue;
        }

        if header.contains("passenger") || header.contains("total passenger") {
            if let Ok(v) = digits.parse::<i64>() {
                if v > 1000 { // sanity check: real pax counts are always > 1000
                    stats.total_pax = Some(v);
                }
            }
        } else if header.contains("domestic") {
            if let Ok(v) = digits.parse::<i64>() {
                stats.domestic_pax = Some(v);
            }
        } else if header.contains("international") {
            if let Ok(v) = digits.parse::<i64>() {
                stats.international_pax = Some(v);
            }
        } else if header.contains("aircraft movement") || header.contains("aircraft operation") {
            if let Ok(v) = digits.parse::<i32>() {
                stats.aircraft_movements = Some(v);
            }
        }
    }

    // Only return if we found at least some data
    if stats.total_pax.is_some()
        || stats.domestic_pax.is_some()
        || stats.international_pax.is_some()
        || stats.aircraft_movements.is_some()
    {
        Some(stats)
    } else {
        None
    }
}

fn parse_passenger_table(wikitext: &str) -> Vec<(i16, i64)> {
    let mut results = Vec::new();

    let table_starts: Vec<usize> = wikitext
        .match_indices("{|")
        .map(|(idx, _)| idx)
        .collect();

    for &start in &table_starts {
        let table_end = match wikitext[start..].find("|}") {
            Some(e) => start + e + 2,
            None => continue,
        };
        let table_text = &wikitext[start..table_end];
        let table_lower = table_text.to_lowercase();

        if !table_lower.contains("passenger") && !table_lower.contains("pax") && !table_lower.contains("traffic") {
            continue;
        }

        for line in table_text.lines() {
            let clean = strip_wiki_markup(line);

            if let Some(year_cap) = RE_YEAR.captures(&clean) {
                let year: i16 = year_cap[1].parse().unwrap();
                if year < 1950 || year > 2030 {
                    continue;
                }

                let numbers: Vec<i64> = RE_NUMBER
                    .find_iter(&clean)
                    .filter_map(|m| {
                        let s = m.as_str().replace(',', "");
                        s.parse::<i64>().ok()
                    })
                    .filter(|&n| n > 10_000)
                    .collect();

                if let Some(&max_pax) = numbers.iter().max() {
                    if let Some(existing) = results.iter_mut().find(|(y, _)| *y == year) {
                        if max_pax > existing.1 {
                            existing.1 = max_pax;
                        }
                    } else {
                        results.push((year, max_pax));
                    }
                }
            }
        }
    }

    if results.is_empty() {
        for line in wikitext.lines() {
            let lower_line = line.to_lowercase();
            if !lower_line.contains("passenger") && !lower_line.contains("pax") {
                continue;
            }
            let clean = strip_wiki_markup(line);
            if let Some(year_cap) = RE_YEAR.captures(&clean) {
                let year: i16 = year_cap[1].parse().unwrap();
                let numbers: Vec<i64> = RE_NUMBER
                    .find_iter(&clean)
                    .filter_map(|m| m.as_str().replace(',', "").parse::<i64>().ok())
                    .filter(|&n| n > 10_000)
                    .collect();
                if let Some(&max_pax) = numbers.iter().max() {
                    if !results.iter().any(|(y, _)| *y == year) {
                        results.push((year, max_pax));
                    }
                }
            }
        }
    }

    results.sort_by_key(|(y, _)| *y);
    results
}

fn extract_section_text(wikitext: &str, keywords: &[&str]) -> Option<String> {
    let lines: Vec<&str> = wikitext.lines().collect();
    let mut result = String::new();

    for (i, line) in lines.iter().enumerate() {
        let lower = line.to_lowercase();
        if lower.starts_with("==") && keywords.iter().any(|k| lower.contains(k)) {
            let header_level = line.chars().take_while(|c| *c == '=').count();
            for &subsequent in &lines[i + 1..] {
                let sub_level = subsequent.chars().take_while(|c| *c == '=').count();
                if sub_level > 0 && sub_level <= header_level {
                    break;
                }
                let cleaned = strip_wiki_markup(subsequent);
                if !cleaned.is_empty() {
                    if !result.is_empty() {
                        result.push(' ');
                    }
                    result.push_str(&cleaned);
                }
            }
        }
    }

    if result.is_empty() {
        None
    } else {
        Some(result.chars().take(5000).collect())
    }
}

fn extract_skytrax_history(wikitext: &str) -> Option<serde_json::Map<String, Value>> {
    let mut history = serde_json::Map::new();

    for cap in RE_SKYTRAX.captures_iter(wikitext) {
        let (year, stars) = if let (Some(y), Some(s)) = (cap.get(1), cap.get(2)) {
            (y.as_str(), s.as_str())
        } else if let (Some(s), Some(y)) = (cap.get(3), cap.get(4)) {
            (y.as_str(), s.as_str())
        } else {
            continue;
        };

        if let (Ok(y), Ok(s)) = (year.parse::<i16>(), stars.parse::<i64>()) {
            if (2000..=2030).contains(&y) && (1..=5).contains(&s) {
                history.insert(year.to_string(), Value::Number(s.into()));
            }
        }
    }

    if history.is_empty() {
        None
    } else {
        Some(history)
    }
}

/// Fetch ACI ASQ awards from the Wikipedia article
/// "List_of_Airport_Service_Quality_Award_winners".
///
/// Returns a JSONB-compatible map like:
/// {"2019": {"1st": "Best Airport Europe >20M"}, "2007": {"3rd": "Best Airport Europe >20M"}}
async fn fetch_aci_awards(
    client: &reqwest::Client,
    airport_name: &str,
) -> Option<serde_json::Map<String, Value>> {
    let url =
        "https://en.wikipedia.org/w/api.php?action=parse&page=List_of_Airport_Service_Quality_Award_winners&prop=wikitext&format=json";

    let resp: WikiParseResponse = client.get(url).send().await.ok()?.json().await.ok()?;
    let wikitext = resp.parse?.wikitext?.content?;

    // Build search terms from the combined airport name + city + short_name string.
    let name_lower = airport_name.to_lowercase();
    let search_terms: Vec<String> = {
        let mut terms = Vec::new();
        // Split by space and add each significant word (>3 chars)
        for word in name_lower.split_whitespace() {
            let w = word.trim();
            if w.len() > 3 && w != "airport" && w != "international" && w != "de" && w != "the" {
                if !terms.contains(&w.to_string()) {
                    terms.push(w.to_string());
                }
            }
        }
        // Also add multi-word combinations like "porto airport", "munich airport"
        if let Some(stripped) = name_lower.strip_suffix(" airport") {
            terms.push(stripped.to_string());
        }
        terms
    };

    let mut awards: serde_json::Map<String, Value> = serde_json::Map::new();
    let mut current_section = String::new();

    for line in wikitext.lines() {
        let trimmed = line.trim();

        // Track section headers for category context
        if trimmed.starts_with("==") {
            current_section = strip_wiki_markup(trimmed)
                .trim_matches('=')
                .trim()
                .to_string();
            continue;
        }

        // Skip non-table rows
        if !trimmed.starts_with('|') && !trimmed.starts_with("||") {
            continue;
        }

        let line_lower = trimmed.to_lowercase();
        let line_clean = strip_wiki_markup(trimmed);

        // Check if this line mentions our airport
        let matches = search_terms.iter().any(|term| line_lower.contains(term));
        if !matches {
            continue;
        }

        // Extract year from this line or nearby context
        if let Some(year_cap) = RE_YEAR.captures(&line_clean) {
            let year = year_cap[1].parse::<i16>().ok()?;
            if !(2000..=2030).contains(&year) {
                continue;
            }

            // Determine placement: check column position in the table row.
            // Wiki tables use || to separate columns. Our airport's position
            // relative to the year tells us 1st/2nd/3rd.
            let cells: Vec<&str> = trimmed.split("||").collect();
            let mut placement = "winner";

            for (i, cell) in cells.iter().enumerate() {
                let cell_lower = cell.to_lowercase();
                if search_terms.iter().any(|t| cell_lower.contains(t)) {
                    placement = match i {
                        0 => "1st", // First data column after year
                        1 => "1st",
                        2 => "2nd",
                        3 => "3rd",
                        _ => "winner",
                    };
                    break;
                }
            }

            let year_str = year.to_string();
            let category = if current_section.is_empty() {
                "ASQ Award".to_string()
            } else {
                current_section.clone()
            };

            let entry = awards
                .entry(year_str)
                .or_insert_with(|| Value::Object(serde_json::Map::new()));

            if let Value::Object(ref mut map) = entry {
                map.insert(
                    placement.to_string(),
                    Value::String(category),
                );
            }
        }
    }

    if awards.is_empty() {
        None
    } else {
        Some(awards)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_wiki_markup_links() {
        assert_eq!(strip_wiki_markup("[[London Heathrow Airport]]"), "London Heathrow Airport");
        assert_eq!(strip_wiki_markup("[[Heathrow Airport|Heathrow]]"), "Heathrow");
    }

    #[test]
    fn strip_wiki_markup_templates_and_refs() {
        let input = "Opened in 1946{{cite web|url=http://example.com}}<ref>source</ref>.";
        let result = strip_wiki_markup(input);
        assert_eq!(result, "Opened in 1946.");
    }

    #[test]
    fn strip_wiki_markup_html_tags() {
        assert_eq!(strip_wiki_markup("<br/>hello<span>world</span>"), "helloworld");
    }

    #[test]
    fn extract_year_basic() {
        assert_eq!(extract_year("opened in 1946"), Some(1946));
        assert_eq!(extract_year("renovated 2019-2022"), Some(2019));
        assert_eq!(extract_year("no year here"), None);
    }

    #[test]
    fn parse_infobox_field_basic() {
        let wikitext = "{{Infobox airport\n| name = London Heathrow\n| opened = 1946\n| operator = [[Heathrow Airport Holdings]]\n}}";
        assert_eq!(parse_infobox_field(wikitext, "opened"), Some("1946".to_string()));
        assert_eq!(parse_infobox_field(wikitext, "operator"), Some("Heathrow Airport Holdings".to_string()));
        assert_eq!(parse_infobox_field(wikitext, "missing"), None);
    }

    #[test]
    fn parse_infobox_start_date_template() {
        let wikitext = "{{Infobox airport\n| name = Berlin Brandenburg Airport\n| opened = {{Start date|2020|10|31|df=y}}\n| operator = [[Flughafen Berlin Brandenburg|Flughafen Berlin Brandenburg GmbH]]\n| owner = States of [[Berlin]] and [[Brandenburg]], and the [[German government]]\n| terminals = 2\n}}\n== History ==\nSome text here.";
        // opened_year should extract 2020 from the Start date template
        let raw = parse_infobox_field_raw(wikitext, "opened");
        assert!(raw.is_some());
        assert_eq!(extract_year(&raw.unwrap()), Some(2020));
        // operator should strip the piped wikilink
        assert_eq!(parse_infobox_field(wikitext, "operator"), Some("Flughafen Berlin Brandenburg GmbH".to_string()));
        // owner should strip multiple wikilinks
        let owner = parse_infobox_field(wikitext, "owner").unwrap();
        assert!(owner.contains("Berlin"));
        assert!(owner.contains("Brandenburg"));
        assert!(!owner.contains("[["));
        // terminals
        let terminals = parse_infobox_field(wikitext, "terminals")
            .and_then(|s| s.trim().parse::<i16>().ok());
        assert_eq!(terminals, Some(2));
    }

    #[test]
    fn extract_infobox_block_nested_templates() {
        let wikitext = "Some preamble\n{{Infobox airport\n| name = Test\n| opened = {{Start date|2020|10|31}}\n}}\n== History ==\nText";
        let block = extract_infobox_block(wikitext).unwrap();
        assert!(block.starts_with("{{Infobox airport"));
        assert!(block.ends_with("}}"));
        assert!(block.contains("Start date"));
    }

    #[test]
    fn parse_passenger_table_basic() {
        let wikitext = r#"
{| class="wikitable" style="text-align:right"
|+ Annual passenger traffic
! Year !! Total passengers
|-
| 2019 || 80,886,589
|-
| 2020 || 22,109,550
|-
| 2021 || 19,392,178
|}
"#;
        let result = parse_passenger_table(wikitext);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], (2019, 80_886_589));
        assert_eq!(result[1], (2020, 22_109_550));
        assert_eq!(result[2], (2021, 19_392_178));
    }

    #[test]
    fn parse_passenger_table_ignores_non_pax_tables() {
        let wikitext = r#"
{| class="wikitable"
|+ Runway specifications
! Runway !! Length
|-
| 09L/27R || 3,902
|}
"#;
        let result = parse_passenger_table(wikitext);
        assert!(result.is_empty());
    }

    #[test]
    fn parse_passenger_table_keeps_larger_value() {
        let wikitext = r#"
{| class="wikitable"
|+ Passenger traffic
! Year !! Domestic !! International !! Total
|-
| 2019 || 5,000,000 || 20,000,000 || 25,000,000
|}
"#;
        let result = parse_passenger_table(wikitext);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], (2019, 25_000_000));
    }

    #[test]
    fn extract_section_text_basic() {
        let wikitext = "== History ==\nThe airport was opened in 1946.\nIt has grown significantly.\n== Terminals ==\nTerminal 5 opened in 2008.";
        let result = extract_section_text(wikitext, &["history"]);
        assert!(result.is_some());
        let text = result.unwrap();
        assert!(text.contains("opened in 1946"));
        assert!(!text.contains("Terminal 5"));
    }

    #[test]
    fn extract_section_text_no_match() {
        let wikitext = "== Terminals ==\nTerminal 1.";
        let result = extract_section_text(wikitext, &["history", "renovation"]);
        assert!(result.is_none());
    }

    #[test]
    fn extract_skytrax_year_then_star() {
        let text = "In 2019 it was rated a 4-star airport by Skytrax.";
        let result = extract_skytrax_history(text);
        assert!(result.is_some());
        let map = result.unwrap();
        assert_eq!(map.get("2019").and_then(|v| v.as_i64()), Some(4));
    }

    #[test]
    fn extract_skytrax_star_then_year() {
        let text = "Rated 3-Star in 2017 by Skytrax.";
        let result = extract_skytrax_history(text);
        assert!(result.is_some());
        let map = result.unwrap();
        assert_eq!(map.get("2017").and_then(|v| v.as_i64()), Some(3));
    }

    #[test]
    fn extract_skytrax_no_match() {
        let result = extract_skytrax_history("No ratings mentioned here.");
        assert!(result.is_none());
    }

    #[test]
    fn article_title_extraction() {
        assert_eq!(
            article_title_from_url("https://en.wikipedia.org/wiki/Berlin_Brandenburg_Airport"),
            Some("Berlin_Brandenburg_Airport")
        );
        assert_eq!(article_title_from_url("https://example.com/no-wiki"), None);
    }

    #[test]
    fn parse_infobox_stats_basic() {
        let wikitext = r#"{{Infobox airport
| name = Istanbul Airport
| stat-year    = 2024
| stat1-header = Passengers
| stat1-data   = 83,859,729 {{increase}}
| stat2-header = Aircraft movements
| stat2-data   = 473,965 {{increase}}
}}"#;
        let stats = parse_infobox_stats(wikitext).unwrap();
        assert_eq!(stats.year, Some(2024));
        assert_eq!(stats.total_pax, Some(83_859_729));
        assert_eq!(stats.aircraft_movements, Some(473_965));
        assert_eq!(stats.domestic_pax, None);
        assert_eq!(stats.international_pax, None);
    }

    #[test]
    fn parse_infobox_stats_with_domestic_international() {
        let wikitext = r#"{{Infobox airport
| name = Test Airport
| stat-year    = 2023
| stat1-header = Passengers
| stat1-data   = 32,433,694
| stat2-header = Aircraft movements
| stat2-data   = 250,000
| stat3-header = Domestic
| stat3-data   = 1,329,005
| stat4-header = International
| stat4-data   = 31,104,689
}}"#;
        let stats = parse_infobox_stats(wikitext).unwrap();
        assert_eq!(stats.year, Some(2023));
        assert_eq!(stats.total_pax, Some(32_433_694));
        assert_eq!(stats.aircraft_movements, Some(250_000));
        assert_eq!(stats.domestic_pax, Some(1_329_005));
        assert_eq!(stats.international_pax, Some(31_104_689));
    }

    #[test]
    fn parse_infobox_stats_with_decrease_template() {
        let wikitext = r#"{{Infobox airport
| name = Test Airport
| stat-year = 2020
| stat1-header = Passengers
| stat1-data = 22,109,550 {{decrease}}
}}"#;
        let stats = parse_infobox_stats(wikitext).unwrap();
        assert_eq!(stats.year, Some(2020));
        assert_eq!(stats.total_pax, Some(22_109_550));
    }

    #[test]
    fn parse_infobox_stats_no_stat_year() {
        let wikitext = r#"{{Infobox airport
| name = Test Airport
| stat1-header = Passengers
| stat1-data = 1,000,000
}}"#;
        assert!(parse_infobox_stats(wikitext).is_none());
    }

    #[test]
    fn parse_infobox_stats_no_data_fields() {
        let wikitext = r#"{{Infobox airport
| name = Test Airport
| stat-year = 2024
}}"#;
        assert!(parse_infobox_stats(wikitext).is_none());
    }

    #[test]
    fn parse_infobox_stats_with_refs() {
        let wikitext = r#"{{Infobox airport
| name = Test Airport
| stat-year = 2024
| stat1-header = Passengers
| stat1-data = 5,000,000<ref>source</ref> {{steady}}
}}"#;
        let stats = parse_infobox_stats(wikitext).unwrap();
        assert_eq!(stats.total_pax, Some(5_000_000));
    }
}
