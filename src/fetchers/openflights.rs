use anyhow::{Context, Result};
use chrono::NaiveDate;
use sqlx::PgPool;
use tracing::info;

use crate::models::{Airport, FetchResult};

const ROUTES_URL: &str =
    "https://raw.githubusercontent.com/jpatokal/openflights/master/data/routes.dat";

/// Frozen date for the OpenFlights dataset (last updated June 2014).
const VALID_TO: &str = "2014-06-01";

/// One-time import of OpenFlights routes.dat as a pre-2022 route baseline.
/// Not used in scoring — only for route globe visualization where OPDI data is absent.
///
/// CSV columns (no header): airline, airline_id, source_airport, source_airport_id,
/// dest_airport, dest_airport_id, codeshare, stops, equipment
///
/// OpenFlights uses IATA codes only. A partial unique index on
/// (origin_id, destination_iata, airline_iata, data_source) WHERE data_source = 'openflights'
/// handles dedup. OPDI/OpenSky routes use a separate ICAO-based index.
pub async fn fetch(pool: &PgPool, airport: &Airport, full_refresh: bool) -> Result<FetchResult> {
    let icao = airport
        .icao_code
        .as_deref()
        .context("Airport has no ICAO code")?;
    let iata = airport
        .iata_code
        .as_deref()
        .context("Airport has no IATA code")?;

    // OpenFlights is a frozen dataset — skip if already imported for this airport.
    if !full_refresh {
        let existing: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM routes WHERE origin_id = $1 AND data_source = 'openflights'",
        )
        .bind(airport.id)
        .fetch_one(pool)
        .await?;

        if existing.0 > 0 {
            info!(airport = iata, routes = existing.0, "OpenFlights already imported, skipping");
            return Ok(FetchResult {
                records_processed: 0,
                last_record_date: None,
            });
        }
    }

    let text = download_routes_cached().await?;

    let valid_to = NaiveDate::parse_from_str(VALID_TO, "%Y-%m-%d").unwrap();
    let mut records: i32 = 0;

    for line in text.lines() {
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() < 9 {
            continue;
        }

        let airline_raw = fields[0].trim().replace('\\', "");
        let source_airport = fields[2].trim();
        let dest_airport = fields[4].trim();

        let is_origin = source_airport == iata || source_airport == icao;
        let is_dest = dest_airport == iata || dest_airport == icao;

        if !is_origin && !is_dest {
            continue;
        }

        // "Other" end of the route — stored as destination regardless of direction.
        let dest_code = if is_origin { dest_airport } else { source_airport };

        if dest_code == "\\N" || dest_code.is_empty() {
            continue;
        }

        let airline = if airline_raw == "\\N" || airline_raw.is_empty() {
            None
        } else {
            Some(airline_raw.as_str())
        };

        sqlx::query(
            r#"
            INSERT INTO routes (origin_id, destination_iata, airline_iata,
                                data_source, first_observed, last_observed)
            VALUES ($1, $2, $3, 'openflights', $4, $4)
            ON CONFLICT (origin_id, destination_iata, airline_iata, data_source)
                WHERE data_source = 'openflights'
            DO NOTHING
            "#,
        )
        .bind(airport.id)
        .bind(dest_code)
        .bind(airline)
        .bind(valid_to)
        .execute(pool)
        .await
        .with_context(|| {
            format!("Failed to insert OpenFlights route {} -> {}", iata, dest_code)
        })?;

        records += 1;
    }

    info!(airport = iata, routes = records, "OpenFlights import complete");

    Ok(FetchResult {
        records_processed: records,
        last_record_date: Some(valid_to),
    })
}

use std::sync::OnceLock;
use tokio::sync::Mutex;

/// Global cache for routes.dat to avoid re-downloading per airport.
static ROUTES_CACHE: OnceLock<Mutex<Option<String>>> = OnceLock::new();

async fn download_routes_cached() -> Result<String> {
    let mutex = ROUTES_CACHE.get_or_init(|| Mutex::new(None));
    let mut guard = mutex.lock().await;

    if let Some(ref cached) = *guard {
        return Ok(cached.clone());
    }

    let client = reqwest::Client::new();
    let text = client
        .get(ROUTES_URL)
        .send()
        .await
        .context("Failed to download OpenFlights routes.dat")?
        .text()
        .await
        .context("Failed to read OpenFlights response body")?;

    *guard = Some(text.clone());
    Ok(text)
}
