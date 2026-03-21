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

    let opened_year = parse_infobox_field(&wikitext, "opened")
        .and_then(|s| extract_year(&s));
    let operator_raw = parse_infobox_field(&wikitext, "operator");
    let owner_raw = parse_infobox_field(&wikitext, "owner");
    let terminal_count = parse_infobox_field(&wikitext, "terminals")
        .and_then(|s| s.trim().parse::<i16>().ok());

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
    .bind(skytrax_history.as_ref().and_then(|v| serde_json::to_value(v).ok()))
    .bind(wiki_url)
    .bind(revision_id)
    .execute(pool)
    .await
    .context("Failed to insert wikipedia_snapshot")?;

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

fn parse_infobox_field(wikitext: &str, field: &str) -> Option<String> {
    // Dynamic regex per field name — can't be static.
    let pattern = format!(r"(?mi)^\|\s*{}\s*=\s*(.+)$", regex::escape(field));
    let re = Regex::new(&pattern).ok()?;
    re.captures(wikitext)
        .and_then(|c| c.get(1))
        .map(|m| strip_wiki_markup(m.as_str().trim()))
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
        let wikitext = "| name = London Heathrow\n| opened = 1946\n| operator = [[Heathrow Airport Holdings]]";
        assert_eq!(parse_infobox_field(wikitext, "opened"), Some("1946".to_string()));
        assert_eq!(parse_infobox_field(wikitext, "operator"), Some("Heathrow Airport Holdings".to_string()));
        assert_eq!(parse_infobox_field(wikitext, "missing"), None);
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
}
