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
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialise tracing (respects RUST_LOG env var, defaults to info).
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    // Validate: need either airport codes or --all.
    if cli.airports.is_empty() && !cli.all {
        bail!("Provide at least one IATA code, or use --all for all seed airports.");
    }

    // Connect to Postgres.
    let pool = db::get_pool().await?;
    info!("Connected to database");

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
    )
    .await?;

    info!("Pipeline complete");
    Ok(())
}
