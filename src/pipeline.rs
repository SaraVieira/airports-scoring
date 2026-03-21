use anyhow::{bail, Result};
use chrono::Datelike;
use sqlx::PgPool;
use tracing::{error, info};

use crate::db;
use crate::fetchers;
use crate::models::{Airport, FetchResult};
use crate::scoring;

/// All known data sources.
pub const ALL_SOURCES: &[&str] = &[
    "ourairports",
    "eurocontrol",
    "metar",
    "opensky",
    "opdi",
    "eurostat",
    "caa",
    "aena",
    "openflights",
    "wikipedia",
    "skytrax",
    "sentiment",
];

/// Dispatch to the correct fetcher for a given source name.
async fn dispatch_fetcher(
    pool: &PgPool,
    airport: &Airport,
    source: &str,
    full_refresh: bool,
) -> Result<FetchResult> {
    match source {
        "ourairports" => fetchers::ourairports::fetch(pool, airport, full_refresh).await,
        "eurocontrol" => fetchers::eurocontrol::fetch(pool, airport, full_refresh).await,
        "metar" => fetchers::metar::fetch(pool, airport, full_refresh).await,
        "opensky" => fetchers::opensky::fetch(pool, airport, full_refresh).await,
        "opdi" => fetchers::opdi::fetch(pool, airport, full_refresh).await,
        "eurostat" => fetchers::eurostat::fetch(pool, airport, full_refresh).await,
        "caa" => fetchers::caa::fetch(pool, airport, full_refresh).await,
        "aena" => fetchers::aena::fetch(pool, airport, full_refresh).await,
        "openflights" => fetchers::openflights::fetch(pool, airport, full_refresh).await,
        "wikipedia" => fetchers::wikipedia::fetch(pool, airport, full_refresh).await,
        "skytrax" | "sentiment" => {
            // Skytrax and sentiment are handled by the Python ML pipeline.
            bail!("Source '{}' is not yet implemented — use Python pipeline", source)
        }
        other => bail!("Unknown source: {}", other),
    }
}

/// Run the pipeline for a set of airports and (optionally) a single source.
///
/// If `source` is `None`, all sources are run for each airport.
/// If `score` is true and no specific source filter is set, compute scores after fetching.
pub async fn run_pipeline(
    pool: &PgPool,
    airports: &[Airport],
    source: Option<&str>,
    full_refresh: bool,
    score: bool,
    reference_year: Option<i16>,
) -> Result<()> {
    let sources: Vec<&str> = match source {
        Some(s) => {
            if !ALL_SOURCES.contains(&s) {
                bail!(
                    "Unknown source '{}'. Valid sources: {}",
                    s,
                    ALL_SOURCES.join(", ")
                );
            }
            vec![s]
        }
        None => ALL_SOURCES.to_vec(),
    };

    // Handle ourairports specially: it's a bulk download, so call it once
    // rather than per-airport.
    if sources.contains(&"ourairports") {
        info!(source = "ourairports", "Starting bulk fetch (once for all airports)");
        let run_id = db::start_pipeline_run(pool, airports[0].id, "ourairports").await?;
        match fetchers::ourairports::fetch_all(pool, full_refresh).await {
            Ok(result) => {
                info!(
                    source = "ourairports",
                    records = result.records_processed,
                    "Fetch completed"
                );
                db::complete_pipeline_run(
                    pool,
                    run_id,
                    "success",
                    result.records_processed,
                    result.last_record_date,
                    None,
                )
                .await?;
            }
            Err(e) => {
                error!(
                    source = "ourairports",
                    error = %e,
                    "Fetch failed"
                );
                db::complete_pipeline_run(
                    pool,
                    run_id,
                    "failed",
                    0,
                    None,
                    Some(&e.to_string()),
                )
                .await?;
            }
        }
    }

    // Run remaining sources per airport.
    for airport in airports {
        let iata = airport
            .iata_code
            .as_deref()
            .unwrap_or("???");

        for &src in &sources {
            // Skip ourairports — already handled above.
            if src == "ourairports" {
                continue;
            }

            info!(airport = iata, source = src, "Starting fetch");

            let run_id = db::start_pipeline_run(pool, airport.id, src).await?;

            match dispatch_fetcher(pool, airport, src, full_refresh).await {
                Ok(result) => {
                    info!(
                        airport = iata,
                        source = src,
                        records = result.records_processed,
                        "Fetch completed"
                    );
                    db::complete_pipeline_run(
                        pool,
                        run_id,
                        "success",
                        result.records_processed,
                        result.last_record_date,
                        None,
                    )
                    .await?;
                }
                Err(e) => {
                    error!(
                        airport = iata,
                        source = src,
                        error = %e,
                        "Fetch failed"
                    );
                    db::complete_pipeline_run(
                        pool,
                        run_id,
                        "failed",
                        0,
                        None,
                        Some(&e.to_string()),
                    )
                    .await?;
                }
            }
        }
    }

    // Compute scores after all fetchers have run.
    if score {
        let year = reference_year.unwrap_or_else(|| {
            chrono::Utc::now().naive_utc().date().year() as i16
        });
        info!(reference_year = year, "Computing scores for all airports");
        scoring::score_airports(pool, airports, year).await?;
    }

    Ok(())
}
