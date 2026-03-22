use super::parsers::*;

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
