mod config;
mod db;
mod fetchers;
#[allow(dead_code)]
mod models;
mod pipeline;
mod scoring;
pub mod scraper_pool;
mod server;

use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use tokio::sync::broadcast;
use tracing::info;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

/// Airport Intelligence Platform — data pipeline and API server.
#[derive(Parser, Debug)]
#[command(name = "airport-fetch", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Fetch data from external sources and upsert into Postgres.
    ///
    /// Fetches from OurAirports, Eurocontrol, METAR, OpenSky, OPDI,
    /// Eurostat, CAA, Reviews, Sentiment, and more.
    Fetch {
        /// IATA codes of airports to process (e.g. LHR CDG AMS).
        /// Ignored when --all is set.
        #[arg(value_name = "AIRPORTS")]
        airports: Vec<String>,

        /// Process all airports in the seed set.
        #[arg(long)]
        all: bool,

        /// Only fetch from this source.
        /// Valid sources: reviews (Skytrax + Google), skytrax (Skytrax only),
        /// google_reviews (Google only), eurocontrol, metar, opensky, routes,
        /// eurostat, caa, aena, wikipedia, sentiment.
        #[arg(long, value_name = "SOURCE")]
        source: Option<String>,

        /// Ignore incremental state and do a full refresh.
        #[arg(long)]
        full_refresh: bool,

        /// Compute all-time airport scores after fetching data.
        #[arg(long)]
        score: bool,
    },

    /// Sync Eurocontrol datasets into local database cache.
    ///
    /// Downloads all Eurocontrol CSV datasets (airport_traffic, ASMA,
    /// taxi-out, taxi-in, vertical flight efficiency, slot adherence)
    /// and ingests them into the eurocontrol_raw table.
    SyncEurocontrol {
        /// Re-download all data including historical apt_dly files back to 2014.
        #[arg(long)]
        full_refresh: bool,
    },

    /// Start the HTTP API server.
    Serve {
        /// Port to listen on.
        #[arg(long, default_value_t = 3001)]
        port: u16,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file (silently ignore if missing).
    dotenvy::dotenv().ok();

    // Create a broadcast channel for streaming logs over SSE.
    let (log_sender, _) = broadcast::channel::<server::logs::LogEntry>(1000);

    // Build the tracing subscriber with an optional broadcast log layer.
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let fmt_layer = tracing_subscriber::fmt::layer();
    let broadcast_layer = server::logs::BroadcastLogLayer::new(log_sender.clone());

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .with(broadcast_layer)
        .init();

    let cli = Cli::parse();

    match cli.command {
        Command::Fetch {
            airports,
            all,
            source,
            full_refresh,
            score,
        } => {
            run_fetch(airports, all, source, full_refresh, score).await?;
        }
        Command::SyncEurocontrol { full_refresh } => {
            let pool = db::get_pool().await?;
            info!("Connected to database");
            let result = fetchers::eurocontrol::sync::run_sync(&pool, full_refresh).await?;
            info!(
                datasets = result.datasets_synced,
                rows = result.total_rows,
                errors = result.errors.len(),
                "Eurocontrol sync complete"
            );
        }
        Command::Serve { port } => {
            server::run(port, log_sender).await?;
        }
    }

    Ok(())
}

async fn run_fetch(
    airports: Vec<String>,
    all: bool,
    source: Option<String>,
    full_refresh: bool,
    score: bool,
) -> Result<()> {
    // Validate: need either airport codes or --all.
    if airports.is_empty() && !all {
        bail!("Provide at least one IATA code, or use --all for all seed airports.");
    }

    // Connect to Postgres.
    let pool = db::get_pool().await?;
    info!("Connected to database");

    // Load seed config from database.
    let seed_config = config::load_seed_airports_from_db(&pool).await?;
    let seed_iata_codes = config::seed_iata_codes(&seed_config);
    info!(count = seed_iata_codes.len(), "Loaded seed airports from database");

    // Bootstrap: OurAirports populates the airports table.
    // Only run if explicitly requested or if the DB is empty.
    if source.as_deref() == Some("ourairports") {
        info!("Running OurAirports fetch (explicitly requested)");
        let result = fetchers::ourairports::fetch_all(&pool, full_refresh, &seed_iata_codes).await?;
        info!(records = result.records_processed, "OurAirports fetch complete");
        info!("Pipeline complete");
        return Ok(());
    }

    // Check if airports exist; if not, bootstrap from OurAirports first.
    let existing = db::get_seed_airports(&pool).await.unwrap_or_default();
    if existing.is_empty() {
        info!("No airports in DB — running OurAirports bootstrap");
        let result = fetchers::ourairports::fetch_all(&pool, full_refresh, &seed_iata_codes).await?;
        info!(records = result.records_processed, "OurAirports bootstrap complete");
    }

    // Resolve the list of airports.
    let resolved_airports = if all {
        info!("Fetching all seed airports");
        db::get_seed_airports(&pool).await?
    } else {
        let mut list = Vec::with_capacity(airports.len());
        for code in &airports {
            let airport = db::get_airport_by_iata(&pool, code).await?;
            list.push(airport);
        }
        list
    };

    if resolved_airports.is_empty() {
        bail!("No airports matched the criteria.");
    }

    info!(count = resolved_airports.len(), "Airports to process");

    // Run the pipeline.
    pipeline::run_pipeline(
        &pool,
        &resolved_airports,
        source.as_deref(),
        full_refresh,
        score,
        &seed_iata_codes,
        &seed_config,
    )
    .await?;

    info!("Pipeline complete");
    Ok(())
}
