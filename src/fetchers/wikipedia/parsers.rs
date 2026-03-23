use std::sync::LazyLock;

use regex::Regex;
use serde_json::Value;

// Pre-compiled regexes used in strip_wiki_markup (called many times per article).
pub(crate) static RE_REF: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"<ref[^>]*>.*?</ref>|<ref[^/]*/?>").unwrap()
});
pub(crate) static RE_WIKILINK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\[\[(?:[^|\]]*\|)?([^\]]*)\]\]").unwrap()
});
pub(crate) static RE_TEMPLATE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\{\{[^}]*\}\}").unwrap()
});
pub(crate) static RE_HTML_TAG: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"<[^>]+>").unwrap()
});
pub(crate) static RE_YEAR: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(19\d{2}|20\d{2})\b").unwrap()
});
pub(crate) static RE_NUMBER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[\d,]+").unwrap()
});
pub(crate) static RE_SKYTRAX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(19\d{2}|20\d{2})\D{0,30}(\d)\s*-?\s*star|(\d)\s*-?\s*star\D{0,30}(19\d{2}|20\d{2})").unwrap()
});

pub(crate) fn article_title_from_url(url: &str) -> Option<&str> {
    url.rsplit_once("/wiki/").map(|(_, title)| title)
}

/// Extract the raw (unstripped) value of an infobox field.
/// This scans only inside the first `{{Infobox airport` block to avoid
/// false-positive matches from other templates or article body text.
pub(crate) fn parse_infobox_field_raw(wikitext: &str, field: &str) -> Option<String> {
    let infobox = extract_infobox_block(wikitext)?;
    let pattern = format!(r"(?mi)^\|\s*{}\s*=\s*(.+)$", regex::escape(field));
    let re = Regex::new(&pattern).ok()?;
    re.captures(&infobox)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().trim().to_string())
}

pub(crate) fn parse_infobox_field(wikitext: &str, field: &str) -> Option<String> {
    let raw = parse_infobox_field_raw(wikitext, field)?;
    let stripped = strip_wiki_markup(&raw);
    if stripped.is_empty() { None } else { Some(stripped) }
}

/// Extract the `{{Infobox airport ... }}` block from wikitext.
/// Handles nested `{{ }}` pairs so we find the correct closing `}}`.
pub(crate) fn extract_infobox_block(wikitext: &str) -> Option<String> {
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

/// Extract a potentially multi-line infobox field value.
/// Captures everything from `| field = ...` until the next `| other_field =` line.
/// This handles fields like `hub` that use `{{ubl|...}}` templates spanning many lines.
pub(crate) fn extract_multiline_infobox_field(infobox: &str, field: &str) -> Option<String> {
    let pattern = format!(r"(?mi)^\|\s*{}\s*=\s*", regex::escape(field));
    let re = Regex::new(&pattern).ok()?;
    let m = re.find(infobox)?;
    let start = m.end();

    // Find the next `| field_name =` line, which marks the end of this field's value
    let next_field_re = Regex::new(r"(?m)^\|\s*\w+\s*=").unwrap();
    let end = next_field_re.find(&infobox[start..])
        .map(|m2| start + m2.start())
        .unwrap_or(infobox.len());

    let value = infobox[start..end].trim();
    if value.is_empty() { None } else { Some(value.to_string()) }
}

pub(crate) fn extract_year(s: &str) -> Option<i16> {
    RE_YEAR.captures(s)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse::<i16>().ok())
}

pub(crate) fn strip_wiki_markup(text: &str) -> String {
    let s = RE_REF.replace_all(text, "");
    let s = RE_WIKILINK.replace_all(&s, "$1");
    let s = RE_TEMPLATE.replace_all(&s, "");
    let s = RE_HTML_TAG.replace_all(&s, "");
    s.trim().to_string()
}

/// Parsed stat* infobox fields.
#[derive(Debug, Default)]
pub(crate) struct InfoboxStats {
    pub(crate) year: Option<i16>,
    pub(crate) total_pax: Option<i64>,
    pub(crate) domestic_pax: Option<i64>,
    pub(crate) international_pax: Option<i64>,
    pub(crate) aircraft_movements: Option<i32>,
}

/// Parse `stat-year`, `stat{N}-header`, and `stat{N}-data` fields from the infobox.
pub(crate) fn parse_infobox_stats(wikitext: &str) -> Option<InfoboxStats> {
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

pub(crate) fn parse_passenger_table(wikitext: &str) -> Vec<(i16, i64)> {
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
                if !(1950..=2030).contains(&year) {
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

pub(crate) fn extract_section_text(wikitext: &str, keywords: &[&str]) -> Option<String> {
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

/// A single hub/focus-city/operating-base entry for an airline at this airport.
#[derive(Debug)]
pub(crate) struct HubEntry {
    pub(crate) airline: String,
    pub(crate) status_type: String,
}

/// Parsed ground-transport data from a Wikipedia article section.
#[derive(Debug, Default)]
pub(crate) struct GroundTransportData {
    pub(crate) has_metro: bool,
    pub(crate) has_rail: bool,
    pub(crate) has_tram: bool,
    pub(crate) has_bus: bool,
    pub(crate) has_direct_rail: bool,
    pub(crate) transport_modes_count: i16,
    pub(crate) raw_notes: Option<String>,
}

/// Parse airline hub/focus-city/operating-base entries from the infobox.
///
/// Reads the `hub`, `focus_city`, and `operating_base` fields and extracts
/// airline names from wikilinks (`[[Foo|Bar]]`) or plain text separated by
/// `<br>`, newlines, or bullet characters.
pub(crate) fn parse_hub_status(wikitext: &str) -> Vec<HubEntry> {
    let fields = [
        ("hub", "hub"),
        ("focus_city", "focus_city"),
        ("operating_base", "operating_base"),
    ];

    let mut entries = Vec::new();
    let infobox = match extract_infobox_block(wikitext) {
        Some(ib) => ib,
        None => return entries,
    };

    for (field, status_type) in &fields {
        // Extract the full multi-line field value from the infobox.
        // Fields like `hub` often use {{ubl|...}} spanning many lines.
        let raw = match extract_multiline_infobox_field(&infobox, field) {
            Some(r) => r,
            None => continue,
        };

        // Extract all wikilink display names — this is the most reliable source.
        for cap in RE_WIKILINK.captures_iter(&raw) {
            if let Some(m) = cap.get(1) {
                let name = m.as_str().trim().to_string();
                if is_valid_airline_name(&name) {
                    entries.push(HubEntry {
                        airline: name,
                        status_type: status_type.to_string(),
                    });
                }
            }
        }

        // If no wikilinks found, try plain-text after stripping all markup.
        // The {{ubl|...}} template and <br> tags are common separators.
        if !entries.iter().any(|e| e.status_type == *status_type) {
            // Strip templates, refs, html tags
            let stripped = strip_wiki_markup(&raw);
            for fragment in stripped.split(['\n', '*', '•', '|']) {
                let name = fragment.trim().to_string();
                if is_valid_airline_name(&name) {
                    entries.push(HubEntry {
                        airline: name,
                        status_type: status_type.to_string(),
                    });
                }
            }
        }
    }

    entries
}

/// Check if a string looks like a valid airline name (not template junk).
fn is_valid_airline_name(name: &str) -> bool {
    if name.len() < 2 || name.len() > 80 {
        return false;
    }
    // Filter out wiki template parameters and markup artifacts
    if name.contains('=') || name.contains('{') || name.contains('}') {
        return false;
    }
    // Filter out common non-airline values from infobox fields
    let lower = name.to_lowercase();
    if lower.starts_with("class")
        || lower.starts_with("nowrap")
        || lower.starts_with("ubl")
        || lower.starts_with("unbulleted")
        || lower.starts_with("plainlist")
        || lower.starts_with("hlist")
        || lower == "and"
        || lower == "or"
    {
        return false;
    }
    true
}

/// Parse ground-transport information from a Wikipedia article section.
///
/// Looks for a section whose heading matches one of several transport-related
/// keywords, then scans the text for mode keywords and direct-rail indicators.
pub(crate) fn parse_ground_transport(wikitext: &str) -> GroundTransportData {
    let mut data = GroundTransportData::default();

    let text = match extract_section_text(
        wikitext,
        &[
            "ground transport",
            "ground transportation",
            "public transport",
            "access",
        ],
    ) {
        Some(t) => t,
        None => return data,
    };

    let lower = text.to_lowercase();

    if lower.contains("metro") || lower.contains("subway") || lower.contains("underground") {
        data.has_metro = true;
    }
    if lower.contains("rail")
        || lower.contains("train")
        || lower.contains("railway")
        || lower.contains("express")
        || lower.contains("cercan")
        || lower.contains("rodalies")
    {
        data.has_rail = true;
    }
    if lower.contains("tram") || lower.contains("light rail") || lower.contains("streetcar") {
        data.has_tram = true;
    }
    if lower.contains("bus") || lower.contains("coach") || lower.contains("shuttle") {
        data.has_bus = true;
    }

    if lower.contains("station at")
        || lower.contains("below the terminal")
        || lower.contains("directly connect")
        || lower.contains("integrated")
        || lower.contains("airport station")
    {
        data.has_direct_rail = true;
    }

    data.transport_modes_count = [data.has_metro, data.has_rail, data.has_tram, data.has_bus]
        .iter()
        .filter(|&&b| b)
        .count() as i16;

    if !text.is_empty() {
        data.raw_notes = Some(text.chars().take(2000).collect());
    }

    data
}

pub(crate) fn extract_skytrax_history(wikitext: &str) -> Option<serde_json::Map<String, Value>> {
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
