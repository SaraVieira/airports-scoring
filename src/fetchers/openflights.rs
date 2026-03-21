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
pub async fn fetch(pool: &PgPool, airport: &Airport, _full_refresh: bool) -> Result<FetchResult> {
    let icao = airport
        .icao_code
        .as_deref()
        .context("Airport has no ICAO code")?;
    let iata = airport
        .iata_code
        .as_deref()
        .context("Airport has no IATA code")?;

    let client = reqwest::Client::new();
    let text = client
        .get(ROUTES_URL)
        .send()
        .await
        .context("Failed to download OpenFlights routes.dat")?
        .text()
        .await
        .context("Failed to read OpenFlights response body")?;

    let valid_to = NaiveDate::parse_from_str(VALID_TO, "%Y-%m-%d").unwrap();
    let mut records: i32 = 0;

    for line in text.lines() {
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() < 9 {
            continue;
        }

        let airline_iata = fields[0].trim().replace('\\', "");
        let source_airport = fields[2].trim();
        let dest_airport = fields[4].trim();

        // Filter: only routes originating or arriving at this airport (by IATA or ICAO).
        let is_origin = source_airport == iata || source_airport == icao;
        let is_dest = dest_airport == iata || dest_airport == icao;

        if !is_origin && !is_dest {
            continue;
        }

        // Determine the "other" airport for this route.
        let (dest_iata_val, origin_is_us) = if is_origin {
            (dest_airport, true)
        } else {
            (source_airport, false)
        };

        // We always store routes with this airport as origin_id.
        // For arrivals, the "destination" in the route table is the other end.
        let _ = origin_is_us; // both directions stored with our airport as origin

        let airline = if airline_iata == "\\N" || airline_iata.is_empty() {
            None
        } else {
            Some(airline_iata.as_str())
        };

        let dest_code = if dest_iata_val == "\\N" || dest_iata_val.is_empty() {
            continue;
        } else {
            dest_iata_val
        };

        sqlx::query(
            r#"
            INSERT INTO routes (origin_id, destination_iata, airline_iata, data_source,
                                first_observed, last_observed)
            VALUES ($1, $2, $3, 'openflights', $4, $4)
            ON CONFLICT (origin_id, destination_icao, airline_icao, data_source) DO NOTHING
            "#,
        )
        .bind(airport.id)
        .bind(dest_code)
        .bind(airline)
        .bind(valid_to)
        .execute(pool)
        .await
        .with_context(|| {
            format!(
                "Failed to insert OpenFlights route {} -> {}",
                iata, dest_code
            )
        })?;

        records += 1;
    }

    info!(airport = iata, routes = records, "OpenFlights import complete");

    Ok(FetchResult {
        records_processed: records,
        last_record_date: Some(valid_to),
    })
}
