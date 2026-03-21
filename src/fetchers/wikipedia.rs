use anyhow::{Context, Result};
use regex::Regex;
use serde::Deserialize;
use serde_json::Value;
use sqlx::PgPool;
use tracing::info;

use crate::models::{Airport, FetchResult};

const USER_AGENT: &str = "AirportIntelligencePlatform/1.0";

/// Extract the article title from a Wikipedia URL.
/// e.g. "https://en.wikipedia.org/wiki/Berlin_Brandenburg_Airport" -> "Berlin_Brandenburg_Airport"
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
pub async fn fetch(pool: &PgPool, airport: &Airport, _full_refresh: bool) -> Result<FetchResult> {
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

    // 1. Get revision ID for change detection.
    let revision_id = fetch_revision_id(&client, title).await?;

    // 2. Get full wikitext for parsing.
    let wikitext = fetch_wikitext(&client, title).await?;

    // 3. Parse infobox fields.
    let opened_year = parse_infobox_field(&wikitext, "opened")
        .and_then(|s| extract_year(&s));
    let operator_raw = parse_infobox_field(&wikitext, "operator");
    let owner_raw = parse_infobox_field(&wikitext, "owner");
    let terminal_count = parse_infobox_field(&wikitext, "terminals")
        .and_then(|s| s.trim().parse::<i16>().ok());

    // 4. Parse passenger statistics tables -> pax_yearly.
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

    // 5. Extract renovation/ownership/milestone notes.
    let renovation_notes = extract_section_text(&wikitext, &["expansion", "renovation", "development", "construction"]);
    let ownership_notes = extract_section_text(&wikitext, &["ownership", "privatisation", "privatization", "shareholders"]);
    let milestone_notes = extract_section_text(&wikitext, &["history", "milestone", "timeline"]);

    // 6. Extract Skytrax history from wikitext (e.g. star rating mentions).
    let skytrax_history = extract_skytrax_history(&wikitext);

    // 7. Upsert wikipedia_snapshots.
    sqlx::query(
        "INSERT INTO wikipedia_snapshots
         (airport_id, opened_year, operator_raw, owner_raw, terminal_count,
          renovation_notes, ownership_notes, milestone_notes,
          skytrax_history, wikipedia_url, article_revision_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
    )
    .bind(airport.id)
    .bind(opened_year)
    .bind(operator_raw.as_deref())
    .bind(owner_raw.as_deref())
    .bind(terminal_count)
    .bind(renovation_notes.as_deref())
    .bind(ownership_notes.as_deref())
    .bind(milestone_notes.as_deref())
    .bind(skytrax_history.as_ref().map(|v| serde_json::to_value(v).ok()).flatten())
    .bind(wiki_url)
    .bind(revision_id)
    .execute(pool)
    .await
    .context("Failed to insert wikipedia_snapshot")?;

    let total = pax_count + 1; // +1 for the snapshot itself
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

/// Parse a simple infobox field like `| opened = 2020` from wikitext.
fn parse_infobox_field(wikitext: &str, field: &str) -> Option<String> {
    let pattern = format!(r"(?mi)^\|\s*{}\s*=\s*(.+)$", regex::escape(field));
    let re = Regex::new(&pattern).ok()?;
    re.captures(wikitext)
        .and_then(|c| c.get(1))
        .map(|m| {
            // Strip wikitext markup like [[...]], {{...}}, <ref>...</ref>
            let raw = m.as_str().trim();
            strip_wiki_markup(raw)
        })
}

/// Extract a 4-digit year from a string.
fn extract_year(s: &str) -> Option<i16> {
    let re = Regex::new(r"\b(19\d{2}|20\d{2})\b").ok()?;
    re.captures(s)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse::<i16>().ok())
}

/// Strip common wikitext markup: [[links]], {{templates}}, <ref> tags, HTML tags.
fn strip_wiki_markup(text: &str) -> String {
    let mut s = text.to_string();
    // Remove <ref>...</ref> and <ref ... />
    if let Ok(re) = Regex::new(r"<ref[^>]*>.*?</ref>|<ref[^/]*/?>") {
        s = re.replace_all(&s, "").to_string();
    }
    // [[Link|Display]] -> Display, [[Link]] -> Link
    if let Ok(re) = Regex::new(r"\[\[(?:[^|\]]*\|)?([^\]]*)\]\]") {
        s = re.replace_all(&s, "$1").to_string();
    }
    // {{convert|...}} and other templates -> just remove
    if let Ok(re) = Regex::new(r"\{\{[^}]*\}\}") {
        s = re.replace_all(&s, "").to_string();
    }
    // Remove remaining HTML tags
    if let Ok(re) = Regex::new(r"<[^>]+>") {
        s = re.replace_all(&s, "").to_string();
    }
    s.trim().to_string()
}

/// Parse passenger statistics table from wikitext.
/// Handles both row-oriented and column-oriented tables.
/// Returns Vec<(year, total_pax)>.
fn parse_passenger_table(wikitext: &str) -> Vec<(i16, i64)> {
    let mut results = Vec::new();
    let year_re = Regex::new(r"\b(19\d{2}|20\d{2})\b").unwrap();
    let num_re = Regex::new(r"[\d,]+").unwrap();

    // Look for passenger-related table sections
    let lower = wikitext.to_lowercase();
    let _passenger_section_starts: Vec<usize> = lower
        .match_indices("passenger")
        .filter_map(|(idx, _)| {
            // Check if there's a wikitable nearby
            let search_range = &lower[idx..lower.len().min(idx + 2000)];
            if search_range.contains("wikitable") || search_range.contains("class=\"wikitable") {
                Some(idx)
            } else {
                None
            }
        })
        .collect();

    // Also find tables that contain year + passenger data patterns
    let table_starts: Vec<usize> = wikitext
        .match_indices("{|")
        .map(|(idx, _)| idx)
        .collect();

    for &start in &table_starts {
        let table_end = wikitext[start..].find("|}").map(|e| start + e + 2);
        let table_end = match table_end {
            Some(e) => e,
            None => continue,
        };
        let table_text = &wikitext[start..table_end];
        let table_lower = table_text.to_lowercase();

        // Only process tables that look like passenger statistics
        if !table_lower.contains("passenger") && !table_lower.contains("pax") && !table_lower.contains("traffic") {
            continue;
        }

        // Parse row by row: look for rows containing a year and a large number
        for line in table_text.lines() {
            let clean = strip_wiki_markup(line);

            if let Some(year_cap) = year_re.captures(&clean) {
                let year: i16 = year_cap[1].parse().unwrap();
                if year < 1950 || year > 2030 {
                    continue;
                }

                // Find the largest number in the line (likely total passengers)
                let numbers: Vec<i64> = num_re
                    .find_iter(&clean)
                    .filter_map(|m| {
                        let s = m.as_str().replace(',', "");
                        s.parse::<i64>().ok()
                    })
                    .filter(|&n| n > 10_000) // passenger counts are > 10k
                    .collect();

                if let Some(&max_pax) = numbers.iter().max() {
                    // Avoid duplicates: keep the larger value for a given year
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
        // Fallback: scan full wikitext for year + passenger patterns
        for line in wikitext.lines() {
            let lower_line = line.to_lowercase();
            if !lower_line.contains("passenger") && !lower_line.contains("pax") {
                continue;
            }
            let clean = strip_wiki_markup(line);
            if let Some(year_cap) = year_re.captures(&clean) {
                let year: i16 = year_cap[1].parse().unwrap();
                let numbers: Vec<i64> = num_re
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

/// Extract text from a wikitext section matching any of the given keywords.
fn extract_section_text(wikitext: &str, keywords: &[&str]) -> Option<String> {
    let lines: Vec<&str> = wikitext.lines().collect();
    let mut result = String::new();

    for (i, line) in lines.iter().enumerate() {
        let lower = line.to_lowercase();
        // Match section headers like == History == or === Expansion ===
        if lower.starts_with("==") && keywords.iter().any(|k| lower.contains(k)) {
            let header_level = line.chars().take_while(|c| *c == '=').count();
            // Collect text until next section of same or higher level
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
        // Truncate to reasonable length
        Some(result.chars().take(5000).collect())
    }
}

/// Extract Skytrax star rating history from wikitext.
/// Looks for patterns like "3-star" or "4-Star" with nearby years.
fn extract_skytrax_history(wikitext: &str) -> Option<serde_json::Map<String, Value>> {
    let re = Regex::new(r"(?i)(19\d{2}|20\d{2})\D{0,30}(\d)\s*-?\s*star|(\d)\s*-?\s*star\D{0,30}(19\d{2}|20\d{2})").ok()?;
    let mut history = serde_json::Map::new();

    for cap in re.captures_iter(wikitext) {
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
