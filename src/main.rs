mod config;
mod db;
mod fetchers;
mod models;
mod pipeline;
mod scoring;

use anyhow::{bail, Result};
use clap::Parser;
use tracing::info;
use tracing_subscriber::EnvFilter;

/// Airport Intelligence Platform — data fetch orchestrator.
///
/// Fetches data from multiple sources (OurAirports, Eurocontrol, METAR,
/// OpenSky, OPDI, Eurostat, CAA, Skytrax, Sentiment) and upserts into
/// Postgres.
#[derive(Parser, Debug)]
#[command(name = "airport-fetch", version, about)]
struct Cli {
    /// IATA codes of airports to process (e.g. LHR CDG AMS).
    /// Ignored when --all is set.
    #[arg(value_name = "AIRPORTS")]
    airports: Vec<String>,

    /// Process all airports in the seed set.
    #[arg(long)]
    all: bool,

    /// Only fetch from this source.
    /// Valid sources: ourairports, eurocontrol, metar, opensky, opdi,
    /// eurostat, caa, skytrax, sentiment.
    #[arg(long, value_name = "SOURCE")]
    source: Option<String>,

    /// Ignore incremental state and do a full refresh.
    #[arg(long)]
    full_refresh: bool,

    /// Compute airport scores after fetching data.
    #[arg(long)]
    score: bool,

    /// Reference year for scoring (defaults to current year).
    #[arg(long, value_name = "YEAR")]
    reference_year: Option<i16>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file (silently ignore if missing).
    dotenvy::dotenv().ok();

    // Initialise tracing (respects RUST_LOG env var, defaults to info).
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    // Load airport config from airports.json.
    let seed_config = config::load_seed_airports(None)?;
    let seed_iata_codes = config::seed_iata_codes(&seed_config);
    info!(count = seed_iata_codes.len(), "Loaded seed airports from airports.json");

    // Validate: need either airport codes or --all.
    if cli.airports.is_empty() && !cli.all {
        bail!("Provide at least one IATA code, or use --all for all seed airports.");
    }

    // Connect to Postgres.
    let pool = db::get_pool().await?;
    info!("Connected to database");

    // Bootstrap: if source is ourairports, run it first before resolving airports.
    // OurAirports is what populates the airports table — can't query it before seeding.
    if cli.source.as_deref() == Some("ourairports") || cli.source.is_none() {
        info!("Running OurAirports bootstrap fetch");
        let result = fetchers::ourairports::fetch_all(&pool, cli.full_refresh, &seed_iata_codes).await?;
        info!(records = result.records_processed, "OurAirports bootstrap complete");

        // If only ourairports was requested, we're done.
        if cli.source.as_deref() == Some("ourairports") {
            info!("Pipeline complete");
            return Ok(());
        }
    }

    // Resolve the list of airports.
    let airports = if cli.all {
        info!("Fetching all seed airports");
        db::get_seed_airports(&pool).await?
    } else {
        let mut list = Vec::with_capacity(cli.airports.len());
        for code in &cli.airports {
            let airport = db::get_airport_by_iata(&pool, code).await?;
            list.push(airport);
        }
        list
    };

    if airports.is_empty() {
        bail!("No airports matched the criteria.");
    }

    info!(count = airports.len(), "Airports to process");

    // Run the pipeline.
    pipeline::run_pipeline(
        &pool,
        &airports,
        cli.source.as_deref(),
        cli.full_refresh,
        cli.score,
        cli.reference_year,
        &seed_iata_codes,
    )
    .await?;

    info!("Pipeline complete");
    Ok(())
}
