use serde::Deserialize;
use serde_json::Value;

use super::parsers::{strip_wiki_markup, RE_YEAR};

#[derive(Debug, Deserialize)]
pub(super) struct WikiParseResponse {
    pub(super) parse: Option<WikiParse>,
}

#[derive(Debug, Deserialize)]
pub(super) struct WikiParse {
    pub(super) wikitext: Option<WikiText>,
}

#[derive(Debug, Deserialize)]
pub(super) struct WikiText {
    #[serde(rename = "*")]
    pub(super) content: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct WikiQueryResponse {
    pub(super) query: Option<WikiQuery>,
}

#[derive(Debug, Deserialize)]
pub(super) struct WikiQuery {
    pub(super) pages: Option<Value>,
}

/// Fetch ACI ASQ awards from the Wikipedia article
/// "List_of_Airport_Service_Quality_Award_winners".
///
/// Returns a JSONB-compatible map like:
/// {"2019": {"1st": "Best Airport Europe >20M"}, "2007": {"3rd": "Best Airport Europe >20M"}}
pub(super) async fn fetch_aci_awards(
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
