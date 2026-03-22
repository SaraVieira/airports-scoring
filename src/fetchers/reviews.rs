use anyhow::{Context, Result};
use sqlx::PgPool;
use tracing::{error, info};

use crate::config::SeedAirport;
use crate::models::{Airport, FetchResult};

/// Unified reviews fetcher — runs Skytrax scraper then Google Reviews scraper
/// in sequence. Either sub-fetcher can fail without breaking the other.
pub async fn fetch(
    pool: &PgPool,
    airport: &Airport,
    full_refresh: bool,
    seed_airports: &[SeedAirport],
) -> Result<FetchResult> {
    let iata = airport
        .iata_code
        .as_deref()
        .context("Airport has no IATA code")?;

    let mut total_records: i32 = 0;
    let mut latest_date = None;

    // 1. Skytrax reviews
    info!(airport = iata, "Running Skytrax reviews...");
    match super::skytrax::fetch(pool, airport, full_refresh).await {
        Ok(result) => {
            info!(
                airport = iata,
                records = result.records_processed,
                "Skytrax reviews completed"
            );
            total_records += result.records_processed;
            if result.last_record_date > latest_date {
                latest_date = result.last_record_date;
            }
        }
        Err(e) => {
            error!(
                airport = iata,
                error = %e,
                "Skytrax reviews failed, continuing with Google"
            );
        }
    }

    // 2. Google reviews
    info!(airport = iata, "Running Google reviews...");
    match super::google_reviews::fetch(pool, airport, full_refresh, seed_airports)
        .await
    {
        Ok(result) => {
            info!(
                airport = iata,
                records = result.records_processed,
                "Google reviews completed"
            );
            total_records += result.records_processed;
            if result.last_record_date > latest_date {
                latest_date = result.last_record_date;
            }
        }
        Err(e) => {
            error!(
                airport = iata,
                error = %e,
                "Google reviews failed"
            );
        }
    }

    Ok(FetchResult {
        records_processed: total_records,
        last_record_date: latest_date,
    })
}
