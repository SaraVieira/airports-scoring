use anyhow::{bail, Result};
use sqlx::PgPool;
use tracing::{error, info, warn};

use crate::config::SeedAirport;
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

/// Data sources run by the Rust CLI per-airport.
/// ourairports is run once at the start of every job (bulk bootstrap).
/// "reviews" runs both Skytrax + Google reviews scrapers.
/// "sentiment" runs the ML pipeline on unprocessed reviews.
pub const ALL_SOURCES: &[&str] = &[
    "eurocontrol",
    "metar",
    "opensky",
    "routes",
    "eurostat",
    "caa",
    "aena",
    "wikipedia",
    "reviews",
    "sentiment",
    "carbon_accreditation",
    "priority_pass",
];

/// Additional source aliases accepted by --source but not in ALL_SOURCES.
const SOURCE_ALIASES: &[&str] = &["skytrax", "google_reviews"];

/// Dispatch to the correct fetcher for a given source name.
pub(crate) async fn dispatch_fetcher(
    pool: &PgPool,
    airport: &Airport,
    source: &str,
    full_refresh: bool,
    seed_iata_codes: &[&str],
    seed_airports: &[SeedAirport],
) -> Result<FetchResult> {
    match source {
        "ourairports" => fetchers::ourairports::fetch(pool, airport, full_refresh, seed_iata_codes).await,
        "eurocontrol" => fetchers::eurocontrol::fetch(pool, airport, full_refresh).await,
        "metar" => fetchers::metar::fetch(pool, airport, full_refresh).await,
        "opensky" => fetchers::opensky::fetch(pool, airport, full_refresh).await,
        "opdi" => fetchers::opdi::fetch(pool, airport, full_refresh).await,
        "routes" => fetchers::routes::fetch(pool, airport, full_refresh).await,
        "eurostat" => fetchers::eurostat::fetch(pool, airport, full_refresh).await,
        "caa" => fetchers::caa::fetch(pool, airport, full_refresh).await,
        "aena" => fetchers::aena::fetch(pool, airport, full_refresh).await,
        "openflights" => fetchers::openflights::fetch(pool, airport, full_refresh).await,
        "wikipedia" => fetchers::wikipedia::fetch(pool, airport, full_refresh).await,
        // Unified reviews: runs Skytrax + Google in sequence
        "reviews" => fetchers::reviews::fetch(pool, airport, full_refresh, seed_airports).await,
        // Individual review sources (for targeted runs)
        "skytrax" => fetchers::skytrax::fetch(pool, airport, full_refresh, seed_airports).await,
        "google_reviews" => fetchers::google_reviews::fetch(pool, airport, full_refresh, seed_airports).await,
        "sentiment" => fetchers::sentiment::fetch(pool, airport, full_refresh).await,
        "carbon_accreditation" => fetchers::carbon_accreditation::fetch(pool, airport, full_refresh).await,
        "priority_pass" => fetchers::priority_pass::fetch(pool, airport, full_refresh).await,
        other => bail!("Unknown source: {}", other),
    }
}

/// Check if a source name is valid (either in ALL_SOURCES or a known alias).
fn is_valid_source(source: &str) -> bool {
    ALL_SOURCES.contains(&source) || SOURCE_ALIASES.contains(&source)
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
    seed_iata_codes: &[&str],
    seed_airports: &[SeedAirport],
) -> Result<()> {
    let sources: Vec<&str> = match source {
        Some(s) => {
            if !is_valid_source(s) {
                bail!(
                    "Unknown source '{}'. Valid sources: {}, {}",
                    s,
                    ALL_SOURCES.join(", "),
                    SOURCE_ALIASES.join(", ")
                );
            }
            vec![s]
        }
        None => ALL_SOURCES.to_vec(),
    };

    // Run sources per airport.
    for airport in airports {
        let iata = airport
            .iata_code
            .as_deref()
            .unwrap_or("???");

        // Track which pax-producing sources returned data for this airport.
        let mut pax_records_by_source: Vec<(&str, i32)> = Vec::new();

        for &src in &sources {
            info!(airport = iata, source = src, "Starting fetch");

            let run_id = db::start_pipeline_run(pool, airport.id, src).await?;

            match dispatch_fetcher(pool, airport, src, full_refresh, seed_iata_codes, seed_airports).await {
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

    // Compute all-time scores after all fetchers have run.
    if score {
        info!("Computing all-time scores for all airports");
        scoring::score_airports(pool, airports).await?;
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
