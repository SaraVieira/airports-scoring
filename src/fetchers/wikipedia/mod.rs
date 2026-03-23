mod aci;
mod parsers;

#[cfg(test)]
mod tests;

use anyhow::{Context, Result};
use sqlx::PgPool;
use tracing::info;

use crate::models::{Airport, FetchResult};

use aci::{fetch_aci_awards, WikiParseResponse, WikiQueryResponse};
use parsers::{
    article_title_from_url, extract_section_text, extract_skytrax_history, extract_year,
    parse_ground_transport, parse_hub_status, parse_infobox_field, parse_infobox_field_raw,
    parse_infobox_stats, parse_passenger_table,
};

pub const USER_AGENT: &str = "AirportIntelligencePlatform/1.0";

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

    // --- Hub status ---
    let hub_entries = parse_hub_status(&wikitext);
    if !hub_entries.is_empty() {
        sqlx::query("DELETE FROM hub_status WHERE airport_id = $1 AND source = 'wikipedia'")
            .bind(airport.id)
            .execute(pool)
            .await
            .with_context(|| format!("Failed to delete old hub_status for {}", iata))?;

        for entry in &hub_entries {
            sqlx::query(
                "INSERT INTO hub_status (airport_id, airline_name, status_type, source)
                 VALUES ($1, $2, $3, 'wikipedia')",
            )
            .bind(airport.id)
            .bind(&entry.airline)
            .bind(&entry.status_type)
            .execute(pool)
            .await
            .with_context(|| format!("Failed to insert hub_status for {}", iata))?;
        }

        info!(airport = iata, count = hub_entries.len(), "Parsed hub status entries");
    }

    // --- Ground transport ---
    let ground = parse_ground_transport(&wikitext);
    if ground.transport_modes_count > 0 {
        sqlx::query(
            "INSERT INTO ground_transport
                 (airport_id, has_metro, has_rail, has_tram, has_bus, has_direct_rail,
                  transport_modes_count, raw_notes)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             ON CONFLICT (airport_id) DO UPDATE SET
                 has_metro             = EXCLUDED.has_metro,
                 has_rail              = EXCLUDED.has_rail,
                 has_tram              = EXCLUDED.has_tram,
                 has_bus               = EXCLUDED.has_bus,
                 has_direct_rail       = EXCLUDED.has_direct_rail,
                 transport_modes_count = EXCLUDED.transport_modes_count,
                 raw_notes             = EXCLUDED.raw_notes,
                 fetched_at            = NOW()",
        )
        .bind(airport.id)
        .bind(ground.has_metro)
        .bind(ground.has_rail)
        .bind(ground.has_tram)
        .bind(ground.has_bus)
        .bind(ground.has_direct_rail)
        .bind(ground.transport_modes_count)
        .bind(ground.raw_notes.as_deref())
        .execute(pool)
        .await
        .with_context(|| format!("Failed to upsert ground_transport for {}", iata))?;

        info!(
            airport = iata,
            modes = ground.transport_modes_count,
            has_direct_rail = ground.has_direct_rail,
            "Parsed ground transport"
        );
    }

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
