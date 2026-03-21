use anyhow::{Context, Result};
use chrono::{NaiveDate, Utc};
use serde::Deserialize;
use sqlx::PgPool;
use std::collections::HashMap;
use tracing::{info, warn};

use crate::models::{Airport, FetchResult};

/// Maximum window per OpenSky API request (seconds). API allows 7 days but
/// we use 2-day chunks to stay well within limits.
const CHUNK_SECS: i64 = 2 * 24 * 3600;

/// OpenSky flight record (arrival or departure).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct FlightRecord {
    icao24: Option<String>,
    first_seen: Option<i64>,
    est_departure_airport: Option<String>,
    last_seen: Option<i64>,
    est_arrival_airport: Option<String>,
    callsign: Option<String>,
    est_departure_airport_horiz_distance: Option<i64>,
    est_departure_airport_vert_distance: Option<i64>,
    est_arrival_airport_horiz_distance: Option<i64>,
    est_arrival_airport_vert_distance: Option<i64>,
    departure_airport_candidates_count: Option<i32>,
    arrival_airport_candidates_count: Option<i32>,
}

/// Aggregation key for route counts.
#[derive(Debug, Hash, Eq, PartialEq, Clone)]
struct RouteKey {
    origin_icao: String,
    destination_icao: String,
    airline_prefix: Option<String>,
}

/// Fetch route and flight data from the OpenSky Network API.
pub async fn fetch(pool: &PgPool, airport: &Airport, _full_refresh: bool) -> Result<FetchResult> {
    let icao = airport
        .icao_code
        .as_deref()
        .context("Airport has no ICAO code")?;

    let username = std::env::var("OPENSKY_USERNAME").unwrap_or_default();
    let password = std::env::var("OPENSKY_PASSWORD").unwrap_or_default();

    if username.is_empty() || password.is_empty() {
        anyhow::bail!(
            "OPENSKY_USERNAME and OPENSKY_PASSWORD env vars are required for the OpenSky fetcher"
        );
    }

    // Determine start time
    let now = Utc::now();
    let last_date: Option<(NaiveDate,)> = sqlx::query_as(
        r#"
        SELECT last_record_date
        FROM pipeline_runs
        WHERE airport_id = $1 AND source = 'opensky' AND status = 'success'
        ORDER BY completed_at DESC
        LIMIT 1
        "#,
    )
    .bind(airport.id)
    .fetch_optional(pool)
    .await?;

    let start_ts = match last_date {
        Some((d,)) => d.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp(),
        None => now.timestamp() - 30 * 24 * 3600,
    };
    let end_ts = now.timestamp();

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()?;

    let mut route_counts: HashMap<RouteKey, (i64, NaiveDate, NaiveDate)> = HashMap::new();

    // Iterate in 2-day chunks, fetching both arrivals and departures
    let mut chunk_begin = start_ts;
    while chunk_begin < end_ts {
        let chunk_end = (chunk_begin + CHUNK_SECS).min(end_ts);

        for direction in &["arrival", "departure"] {
            let url = format!(
                "https://opensky-network.org/api/flights/{}?airport={}&begin={}&end={}",
                direction, icao, chunk_begin, chunk_end
            );

            let resp = match client
                .get(&url)
                .basic_auth(&username, Some(&password))
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    warn!(url = %url, error = %e, "OpenSky request failed, skipping chunk");
                    continue;
                }
            };

            if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                warn!("OpenSky rate limit hit, stopping");
                break;
            }

            if !resp.status().is_success() {
                warn!(status = %resp.status(), url = %url, "OpenSky non-success, skipping");
                continue;
            }

            let flights: Vec<FlightRecord> = match resp.json().await {
                Ok(f) => f,
                Err(e) => {
                    warn!(error = %e, "Failed to parse OpenSky JSON, skipping chunk");
                    continue;
                }
            };

            for flight in &flights {
                let origin = match &flight.est_departure_airport {
                    Some(o) if !o.is_empty() => o.clone(),
                    _ => continue,
                };
                let dest = match &flight.est_arrival_airport {
                    Some(d) if !d.is_empty() => d.clone(),
                    _ => continue,
                };

                // Extract airline prefix from callsign (first 3 chars if alpha)
                let airline = flight.callsign.as_ref().and_then(|cs| {
                    let cs = cs.trim();
                    if cs.len() >= 3 && cs[..3].chars().all(|c| c.is_ascii_alphabetic()) {
                        Some(cs[..3].to_uppercase())
                    } else {
                        None
                    }
                });

                let key = RouteKey {
                    origin_icao: origin,
                    destination_icao: dest,
                    airline_prefix: airline,
                };

                // Determine the flight date from last_seen or first_seen
                let ts = flight.last_seen.or(flight.first_seen).unwrap_or(chunk_begin);
                let flight_date = chrono::DateTime::from_timestamp(ts, 0)
                    .map(|dt| dt.naive_utc().date())
                    .unwrap_or_else(|| {
                        chrono::DateTime::from_timestamp(chunk_begin, 0)
                            .unwrap()
                            .naive_utc()
                            .date()
                    });

                let entry = route_counts
                    .entry(key)
                    .or_insert((0, flight_date, flight_date));
                entry.0 += 1;
                if flight_date < entry.1 {
                    entry.1 = flight_date;
                }
                if flight_date > entry.2 {
                    entry.2 = flight_date;
                }
            }
        }

        chunk_begin = chunk_end;
    }

    // Upsert route aggregations
    let mut records_processed: i32 = 0;
    let mut latest_date: Option<NaiveDate> = None;

    for (key, (count, first_seen, last_seen)) in &route_counts {
        sqlx::query(
            r#"
            INSERT INTO routes
                (origin_id, destination_icao, airline_icao,
                 flights_per_month, first_observed, last_observed, data_source)
            VALUES ($1, $2, $3, $4, $5, $6, 'opensky')
            ON CONFLICT (origin_id, destination_icao, airline_icao, data_source)
            DO UPDATE SET
                flights_per_month = EXCLUDED.flights_per_month,
                first_observed    = LEAST(routes.first_observed, EXCLUDED.first_observed),
                last_observed     = GREATEST(routes.last_observed, EXCLUDED.last_observed),
                updated_at        = NOW()
            "#,
        )
        .bind(airport.id)
        .bind(&key.destination_icao)
        .bind(&key.airline_prefix)
        .bind(*count as i32)
        .bind(first_seen)
        .bind(last_seen)
        .execute(pool)
        .await
        .with_context(|| {
            format!(
                "Failed to upsert route {}-{}",
                key.origin_icao, key.destination_icao
            )
        })?;

        records_processed += 1;
        latest_date = Some(match latest_date {
            Some(prev) if *last_seen > prev => *last_seen,
            Some(prev) => prev,
            None => *last_seen,
        });
    }

    info!(
        airport = icao,
        routes = records_processed,
        "OpenSky fetch complete"
    );

    Ok(FetchResult {
        records_processed,
        last_record_date: latest_date,
    })
}
