use anyhow::{Context, Result};
use serde::Deserialize;
use sqlx::PgPool;
use std::collections::HashMap;
use tracing::{info, warn};

use crate::models::{Airport, FetchResult};

/// Jonty/airline-route-data — weekly-updated route data from FlightRadar24.
const JONTY_URL: &str =
    "https://raw.githubusercontent.com/Jonty/airline-route-data/refs/heads/main/airline_routes.json";

// ── Jonty JSON structures ────────────────────────────────

#[derive(Debug, Deserialize)]
struct JontyAirport {
    #[serde(default)]
    routes: Vec<JontyRoute>,
}

#[derive(Debug, Deserialize)]
struct JontyRoute {
    iata: String,
    #[serde(default)]
    carriers: Vec<JontyCarrier>,
}

#[derive(Debug, Deserialize)]
struct JontyCarrier {
    iata: Option<String>,
    name: Option<String>,
}

/// Unified route fetcher: runs OPDI first (real flight counts),
/// then fills gaps with Jonty/airline-route-data (weekly-updated, airline names).
pub async fn fetch(pool: &PgPool, airport: &Airport, full_refresh: bool) -> Result<FetchResult> {
    let iata = airport
        .iata_code
        .as_deref()
        .context("Airport has no IATA code")?;

    // Step 1: Run OPDI (primary — actual flight counts)
    let opdi_result = super::opdi::fetch(pool, airport, full_refresh).await;
    let opdi_routes = match &opdi_result {
        Ok(r) => {
            info!(airport = iata, routes = r.records_processed, "OPDI routes fetched");
            r.records_processed
        }
        Err(e) => {
            warn!(airport = iata, error = %e, "OPDI fetch failed, will try Jonty fallback");
            0
        }
    };

    // Step 2: Fetch Jonty data to fill gaps
    let jonty_routes = match fetch_jonty(pool, airport).await {
        Ok(count) => {
            info!(airport = iata, routes = count, "Jonty fallback routes inserted");
            count
        }
        Err(e) => {
            warn!(airport = iata, error = %e, "Jonty fallback also failed");
            0
        }
    };

    let total = opdi_routes + jonty_routes;
    let last_date = opdi_result.ok().and_then(|r| r.last_record_date);

    Ok(FetchResult {
        records_processed: total,
        last_record_date: last_date,
    })
}

/// Fetch routes from Jonty/airline-route-data and insert only routes
/// that don't already exist (from OPDI).
async fn fetch_jonty(pool: &PgPool, airport: &Airport) -> Result<i32> {
    let iata = airport
        .iata_code
        .as_deref()
        .context("Airport has no IATA code")?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()?;

    info!(airport = iata, "Downloading Jonty airline-route-data");
    let resp = client.get(JONTY_URL).send().await?;
    if !resp.status().is_success() {
        anyhow::bail!("Jonty download failed: {}", resp.status());
    }

    let data: HashMap<String, JontyAirport> = resp.json().await?;

    let airport_data = match data.get(iata) {
        Some(a) => a,
        None => {
            info!(airport = iata, "Airport not found in Jonty data");
            return Ok(0);
        }
    };

    // Get existing OPDI destination ICAOs to avoid duplicates
    let existing: Vec<(Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT destination_icao, destination_iata FROM routes WHERE origin_id = $1",
    )
    .bind(airport.id)
    .fetch_all(pool)
    .await?;

    let existing_iatas: std::collections::HashSet<String> = existing
        .iter()
        .filter_map(|(_, iata)| iata.clone())
        .collect();

    let existing_icaos: std::collections::HashSet<String> = existing
        .iter()
        .filter_map(|(icao, _)| icao.clone())
        .collect();

    // Batch-load IATA→ICAO mapping to avoid N+1 queries
    let iata_icao_rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT iata, icao FROM all_airports WHERE iata IS NOT NULL",
    )
    .fetch_all(pool)
    .await?;

    let iata_to_icao: HashMap<String, String> = iata_icao_rows.into_iter().collect();

    let mut inserted = 0i32;

    for route in &airport_data.routes {
        // Skip self-referencing routes
        if route.iata == iata {
            continue;
        }

        // Skip if we already have this route from OPDI
        if existing_iatas.contains(&route.iata) {
            continue;
        }

        // Look up ICAO from pre-loaded mapping
        let dest_icao = iata_to_icao.get(&route.iata).cloned();

        // Skip if we already have this ICAO from OPDI
        if let Some(ref icao) = dest_icao {
            if existing_icaos.contains(icao) {
                continue;
            }
        }

        // Build airline name from carriers
        let airline_name = route
            .carriers
            .iter()
            .filter_map(|c| c.name.as_deref())
            .collect::<Vec<_>>()
            .join(" · ");

        let airline_iata = route
            .carriers
            .first()
            .and_then(|c| c.iata.as_deref());

        let result = sqlx::query(
            r#"
            INSERT INTO routes
                (origin_id, destination_icao, destination_iata, airline_iata, airline_name,
                 data_source)
            SELECT $1, $2, $3, $4, $5, 'jonty'
            WHERE NOT EXISTS (
                SELECT 1 FROM routes
                WHERE origin_id = $1
                  AND (destination_iata = $3 OR destination_icao = $2)
            )
            "#,
        )
        .bind(airport.id)
        .bind(&dest_icao)
        .bind(&route.iata)
        .bind(airline_iata)
        .bind(if airline_name.is_empty() { None } else { Some(&airline_name) })
        .execute(pool)
        .await;

        match result {
            Ok(_) => inserted += 1,
            Err(e) => {
                warn!(
                    dest = %route.iata,
                    error = %e,
                    "Failed to insert Jonty route"
                );
            }
        }
    }

    Ok(inserted)
}
