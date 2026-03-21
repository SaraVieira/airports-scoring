use anyhow::{bail, Result};
use chrono::Datelike;
use sqlx::PgPool;
use tracing::{error, info, warn};

use crate::db;
use crate::fetchers;
use crate::models::{Airport, FetchResult};
use crate::scoring;

/// Countries that have a dedicated passenger data fetcher.
/// If an airport's country is NOT in this list, we emit a warning
/// because we're relying solely on Wikipedia/Eurostat for pax data.
const COUNTRIES_WITH_PAX_FETCHER: &[&str] = &[
    "ES", // AENA
];

/// Sources that produce passenger data (write to pax_yearly).
const PAX_SOURCES: &[&str] = &["aena", "eurostat", "wikipedia"];

/// Data sources run by the Rust CLI.
/// skytrax and sentiment are handled by the Python ML pipeline
/// and must be requested explicitly via --source skytrax/sentiment.
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
];

/// Dispatch to the correct fetcher for a given source name.
async fn dispatch_fetcher(
    pool: &PgPool,
    airport: &Airport,
    source: &str,
    full_refresh: bool,
    seed_iata_codes: &[&str],
) -> Result<FetchResult> {
    match source {
        "ourairports" => fetchers::ourairports::fetch(pool, airport, full_refresh, seed_iata_codes).await,
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
            warn!(source = source, "This source requires the Python ML pipeline — run python/skytrax_scraper.py or python/sentiment_pipeline.py directly");
            Ok(FetchResult { records_processed: 0, last_record_date: None })
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
    seed_iata_codes: &[&str],
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
        match fetchers::ourairports::fetch_all(pool, full_refresh, seed_iata_codes).await {
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

        // Track which pax-producing sources returned data for this airport.
        let mut pax_records_by_source: Vec<(&str, i32)> = Vec::new();

        for &src in &sources {
            // Skip ourairports — already handled above.
            if src == "ourairports" {
                continue;
            }

            info!(airport = iata, source = src, "Starting fetch");

            let run_id = db::start_pipeline_run(pool, airport.id, src).await?;

            match dispatch_fetcher(pool, airport, src, full_refresh, seed_iata_codes).await {
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

                    if PAX_SOURCES.contains(&src) {
                        pax_records_by_source.push((src, result.records_processed));
                    }
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

                    if PAX_SOURCES.contains(&src) {
                        pax_records_by_source.push((src, 0));
                    }
                }
            }
        }

        // Check passenger data coverage for this airport.
        if source.is_none() {
            check_pax_coverage(airport, iata, &pax_records_by_source);
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

/// Check whether an airport has passenger data from at least one source.
/// Warns if no country-specific pax fetcher exists, errors if Wikipedia
/// also failed to provide pax data.
fn check_pax_coverage(airport: &Airport, iata: &str, pax_results: &[(&str, i32)]) {
    let has_country_fetcher = COUNTRIES_WITH_PAX_FETCHER.contains(&airport.country_code.as_str());

    let country_pax = pax_results
        .iter()
        .filter(|(src, _)| *src != "wikipedia")
        .any(|(_, count)| *count > 0);

    let wiki_pax = pax_results
        .iter()
        .find(|(src, _)| *src == "wikipedia")
        .map_or(false, |(_, count)| *count > 0);

    let total_pax = country_pax || wiki_pax;

    if !has_country_fetcher {
        warn!(
            airport = iata,
            country = %airport.country_code,
            "No country-specific passenger data fetcher for this country — \
             relying on Wikipedia/Eurostat only"
        );
    }

    if !total_pax {
        error!(
            airport = iata,
            country = %airport.country_code,
            "No passenger data found from any source (country fetcher, Eurostat, or Wikipedia)"
        );
    }
}
