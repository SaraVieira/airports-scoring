mod db;
mod types;

use std::collections::HashMap;

use anyhow::{Context, Result};
use sqlx::PgPool;
use tracing::info;

use crate::models::{Airport, FetchResult};
use types::CsvAirport;

// Seed IATA codes are loaded from the supported_airports table at startup.
// The fetch_all() function receives them as a parameter.

const AIRPORTS_CSV_URL: &str =
    "https://davidmegginson.github.io/ourairports-data/airports.csv";
const RUNWAYS_CSV_URL: &str =
    "https://davidmegginson.github.io/ourairports-data/runways.csv";
const FREQUENCIES_CSV_URL: &str =
    "https://davidmegginson.github.io/ourairports-data/airport-frequencies.csv";
const NAVAIDS_CSV_URL: &str =
    "https://davidmegginson.github.io/ourairports-data/navaids.csv";

// ── Public API ──────────────────────────────────────────────────

/// Fetch airport, runway, and frequency data from OurAirports CSV files.
///
/// Because OurAirports is a bulk download, the per-airport `fetch` delegates
/// to `fetch_all` which processes every seed airport in one pass.
pub async fn fetch(pool: &PgPool, _airport: &Airport, full_refresh: bool, seed_iata_codes: &[&str]) -> Result<FetchResult> {
    fetch_all(pool, full_refresh, seed_iata_codes).await
}

/// Download all three OurAirports CSVs and upsert seed airports, their
/// runways, and frequencies into Postgres.
pub async fn fetch_all(pool: &PgPool, _full_refresh: bool, seed_iata_codes: &[&str]) -> Result<FetchResult> {
    let client = reqwest::Client::new();

    // 1. Download all four CSVs in parallel.
    let (airports_text, runways_text, frequencies_text, navaids_text) = tokio::try_join!(
        download_csv(&client, AIRPORTS_CSV_URL),
        download_csv(&client, RUNWAYS_CSV_URL),
        download_csv(&client, FREQUENCIES_CSV_URL),
        download_csv(&client, NAVAIDS_CSV_URL),
    )?;

    // 2. Parse airports CSV and filter to seed set.
    let csv_airports = parse_csv::<CsvAirport>(&airports_text)?;
    let seed_airports: Vec<&CsvAirport> = csv_airports
        .iter()
        .filter(|a| {
            a.iata_code
                .as_deref()
                .map(|code| seed_iata_codes.contains(&code))
                .unwrap_or(false)
        })
        .collect();

    info!(
        total_csv = csv_airports.len(),
        seed_matched = seed_airports.len(),
        "Parsed airports CSV"
    );

    // Build a map from ourairports_id -> iata_code for seed airports,
    // so we can match runways/frequencies later.
    let seed_oa_ids: HashMap<i64, &str> = seed_airports
        .iter()
        .map(|a| (a.id, a.iata_code.as_deref().unwrap_or("")))
        .collect();

    // 3. Upsert airports.
    let (records, oa_id_to_db_id) = db::upsert_airports(pool, &seed_airports).await?;

    // 4. Parse and insert runways.
    let csv_runways = parse_csv::<types::CsvRunway>(&runways_text)?;
    let runway_count = db::insert_runways(pool, &csv_runways, &seed_oa_ids, &oa_id_to_db_id).await?;

    // 5. Parse and insert frequencies.
    let csv_frequencies = parse_csv::<types::CsvFrequency>(&frequencies_text)?;
    let freq_count = db::insert_frequencies(pool, &csv_frequencies, &seed_oa_ids, &oa_id_to_db_id).await?;

    // 6. Parse and insert navaids.
    let csv_navaids = parse_csv::<types::CsvNavaid>(&navaids_text)?;
    let navaid_count = db::insert_navaids(pool, &csv_navaids, &seed_airports, &oa_id_to_db_id).await?;

    let total = records + runway_count + freq_count + navaid_count;
    info!(total = total, "OurAirports fetch complete");

    Ok(FetchResult {
        records_processed: total,
        last_record_date: None,
    })
}

// ── Internal helpers ────────────────────────────────────────────

async fn download_csv(client: &reqwest::Client, url: &str) -> Result<String> {
    info!(url = url, "Downloading CSV");
    let resp = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("Failed to GET {}", url))?;
    let text = resp
        .text()
        .await
        .with_context(|| format!("Failed to read body from {}", url))?;
    Ok(text)
}

fn parse_csv<T: serde::de::DeserializeOwned>(text: &str) -> Result<Vec<T>> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_reader(text.as_bytes());
    rdr.deserialize()
        .collect::<std::result::Result<Vec<T>, _>>()
        .context("Failed to parse CSV")
}
