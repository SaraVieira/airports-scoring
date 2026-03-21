#![allow(dead_code)]
use std::collections::HashMap;

use anyhow::{Context, Result};
use rust_decimal::Decimal;
use serde::Deserialize;
use sqlx::PgPool;
use tracing::info;

use crate::models::{Airport, FetchResult};

// Seed IATA codes are loaded from airports.json at startup —
// see config::load_seed_airports(). The fetch_all() function
// receives them as a parameter.

const AIRPORTS_CSV_URL: &str =
    "https://davidmegginson.github.io/ourairports-data/airports.csv";
const RUNWAYS_CSV_URL: &str =
    "https://davidmegginson.github.io/ourairports-data/runways.csv";
const FREQUENCIES_CSV_URL: &str =
    "https://davidmegginson.github.io/ourairports-data/airport-frequencies.csv";
const NAVAIDS_CSV_URL: &str =
    "https://davidmegginson.github.io/ourairports-data/navaids.csv";

// ── CSV row structs ─────────────────────────────────────────────

/// Row from airports.csv
#[derive(Debug, Deserialize)]
struct CsvAirport {
    id: i64,
    ident: String,
    #[serde(rename = "type")]
    airport_type: String,
    name: String,
    latitude_deg: Option<f64>,
    longitude_deg: Option<f64>,
    elevation_ft: Option<String>,
    continent: Option<String>,
    iso_country: Option<String>,
    iso_region: Option<String>,
    municipality: Option<String>,
    scheduled_service: Option<String>,
    gps_code: Option<String>,
    iata_code: Option<String>,
    local_code: Option<String>,
    home_link: Option<String>,
    wikipedia_link: Option<String>,
    keywords: Option<String>,
}

/// Row from runways.csv
#[derive(Debug, Deserialize)]
struct CsvRunway {
    id: i64,
    airport_ref: i64,
    airport_ident: Option<String>,
    length_ft: Option<String>,
    width_ft: Option<String>,
    surface: Option<String>,
    lighted: Option<String>,
    closed: Option<String>,
    le_ident: Option<String>,
    le_latitude_deg: Option<String>,
    le_longitude_deg: Option<String>,
    le_elevation_ft: Option<String>,
    #[serde(rename = "le_heading_degT")]
    le_heading_deg_t: Option<String>,
    le_displaced_threshold_ft: Option<String>,
    he_ident: Option<String>,
    he_latitude_deg: Option<String>,
    he_longitude_deg: Option<String>,
    he_elevation_ft: Option<String>,
    #[serde(rename = "he_heading_degT")]
    he_heading_deg_t: Option<String>,
    he_displaced_threshold_ft: Option<String>,
}

/// Row from airport-frequencies.csv
#[derive(Debug, Deserialize)]
struct CsvFrequency {
    id: i64,
    airport_ref: i64,
    airport_ident: Option<String>,
    #[serde(rename = "type")]
    freq_type: Option<String>,
    description: Option<String>,
    frequency_mhz: Option<f64>,
}

/// Row from navaids.csv
#[derive(Debug, Deserialize)]
struct CsvNavaid {
    id: i64,
    #[serde(rename = "filename")]
    _filename: Option<String>,
    ident: Option<String>,
    name: Option<String>,
    #[serde(rename = "type")]
    navaid_type: Option<String>,
    frequency_khz: Option<String>,
    latitude_deg: Option<f64>,
    longitude_deg: Option<f64>,
    elevation_ft: Option<String>,
    iso_country: Option<String>,
    dme_frequency_khz: Option<String>,
    dme_channel: Option<String>,
    dme_latitude_deg: Option<f64>,
    dme_longitude_deg: Option<f64>,
    dme_elevation_ft: Option<String>,
    slaved_variation_deg: Option<String>,
    magnetic_variation_deg: Option<String>,
    #[serde(rename = "usageType")]
    usage_type: Option<String>,
    power: Option<String>,
    associated_airport: Option<String>,
}

// ── Helper: parse optional numeric strings ──────────────────────

fn parse_opt_i32(s: &Option<String>) -> Option<i32> {
    s.as_deref()
        .and_then(|v| v.trim().parse::<i32>().ok())
}

fn parse_opt_f64(s: &Option<String>) -> Option<f64> {
    s.as_deref()
        .and_then(|v| v.trim().parse::<f64>().ok())
}

fn parse_opt_decimal(s: &Option<String>) -> Option<Decimal> {
    s.as_deref()
        .and_then(|v| v.trim().parse::<Decimal>().ok())
}

fn parse_bool_field(s: &Option<String>) -> Option<bool> {
    s.as_deref().map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
}

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
    let mut records: i32 = 0;
    // We need to collect the mapping from ourairports_id -> db airport id
    // for inserting runways and frequencies.
    let mut oa_id_to_db_id: HashMap<i64, i32> = HashMap::new();

    for airport in &seed_airports {
        let lat = airport.latitude_deg.unwrap_or(0.0);
        let lon = airport.longitude_deg.unwrap_or(0.0);
        let iata = airport.iata_code.as_deref().unwrap_or("");
        let country = airport
            .iso_country
            .as_deref()
            .unwrap_or("XX");
        let municipality = airport
            .municipality
            .as_deref()
            .unwrap_or("Unknown");
        let scheduled = airport
            .scheduled_service
            .as_deref()
            .map(|v| v == "yes")
            .unwrap_or(false);

        let row: (i32,) = sqlx::query_as(
            r#"
            INSERT INTO airports (
                iata_code, icao_code, ourairports_id,
                name, city, country_code,
                location, elevation_ft,
                airport_type, scheduled_service,
                wikipedia_url, website_url,
                in_seed_set
            ) VALUES (
                $1, $2, $3,
                $4, $5, $6,
                ST_MakePoint($7, $8)::geography, $9,
                $10, $11,
                $12, $13,
                TRUE
            )
            ON CONFLICT (iata_code) DO UPDATE SET
                icao_code        = EXCLUDED.icao_code,
                ourairports_id   = EXCLUDED.ourairports_id,
                name             = EXCLUDED.name,
                city             = EXCLUDED.city,
                country_code     = EXCLUDED.country_code,
                location         = EXCLUDED.location,
                elevation_ft     = EXCLUDED.elevation_ft,
                airport_type     = EXCLUDED.airport_type,
                scheduled_service = EXCLUDED.scheduled_service,
                wikipedia_url    = EXCLUDED.wikipedia_url,
                website_url      = EXCLUDED.website_url,
                in_seed_set      = TRUE,
                updated_at       = NOW()
            RETURNING id
            "#,
        )
        .bind(iata)
        .bind(airport.gps_code.as_deref().or(Some(&airport.ident)))
        .bind(airport.id as i32)
        .bind(&airport.name)
        .bind(municipality)
        .bind(country)
        .bind(lon) // ST_MakePoint takes (lon, lat)
        .bind(lat)
        .bind(parse_opt_i32(&airport.elevation_ft))
        .bind(&airport.airport_type)
        .bind(scheduled)
        .bind(airport.wikipedia_link.as_deref())
        .bind(airport.home_link.as_deref())
        .fetch_one(pool)
        .await
        .with_context(|| format!("Failed to upsert airport {}", iata))?;

        oa_id_to_db_id.insert(airport.id, row.0);
        records += 1;
    }

    info!(count = records, "Upserted airports");

    // 4. Parse and insert runways.
    let csv_runways = parse_csv::<CsvRunway>(&runways_text)?;
    let mut runway_count: i32 = 0;

    // Group runways by airport_ref for batch delete+insert.
    let mut runways_by_airport: HashMap<i64, Vec<&CsvRunway>> = HashMap::new();
    for rwy in &csv_runways {
        if seed_oa_ids.contains_key(&rwy.airport_ref) {
            runways_by_airport
                .entry(rwy.airport_ref)
                .or_default()
                .push(rwy);
        }
    }

    for (oa_id, rwys) in &runways_by_airport {
        let db_id = match oa_id_to_db_id.get(oa_id) {
            Some(id) => *id,
            None => continue,
        };

        // Delete existing runways for this airport.
        sqlx::query("DELETE FROM runways WHERE airport_id = $1")
            .bind(db_id)
            .execute(pool)
            .await
            .context("Failed to delete existing runways")?;

        for rwy in rwys {
            let ident = match (&rwy.le_ident, &rwy.he_ident) {
                (Some(le), Some(he)) => Some(format!("{}/{}", le, he)),
                (Some(le), None) => Some(le.clone()),
                (None, Some(he)) => Some(he.clone()),
                (None, None) => None,
            };

            sqlx::query(
                r#"
                INSERT INTO runways (
                    airport_id, ident, le_ident, he_ident,
                    length_ft, width_ft, surface, lighted, closed,
                    le_latitude_deg, le_longitude_deg, le_elevation_ft,
                    "le_heading_degT", le_displaced_threshold_ft,
                    he_latitude_deg, he_longitude_deg, he_elevation_ft,
                    "he_heading_degT", he_displaced_threshold_ft
                ) VALUES (
                    $1, $2, $3, $4,
                    $5, $6, $7, $8, $9,
                    $10, $11, $12, $13, $14,
                    $15, $16, $17, $18, $19
                )
                "#,
            )
            .bind(db_id)
            .bind(&ident)
            .bind(rwy.le_ident.as_deref())
            .bind(rwy.he_ident.as_deref())
            .bind(parse_opt_i32(&rwy.length_ft))
            .bind(parse_opt_i32(&rwy.width_ft))
            .bind(rwy.surface.as_deref())
            .bind(parse_bool_field(&rwy.lighted))
            .bind(parse_bool_field(&rwy.closed))
            .bind(parse_opt_f64(&rwy.le_latitude_deg))
            .bind(parse_opt_f64(&rwy.le_longitude_deg))
            .bind(parse_opt_i32(&rwy.le_elevation_ft))
            .bind(parse_opt_decimal(&rwy.le_heading_deg_t))
            .bind(parse_opt_i32(&rwy.le_displaced_threshold_ft))
            .bind(parse_opt_f64(&rwy.he_latitude_deg))
            .bind(parse_opt_f64(&rwy.he_longitude_deg))
            .bind(parse_opt_i32(&rwy.he_elevation_ft))
            .bind(parse_opt_decimal(&rwy.he_heading_deg_t))
            .bind(parse_opt_i32(&rwy.he_displaced_threshold_ft))
            .execute(pool)
            .await
            .context("Failed to insert runway")?;

            runway_count += 1;
        }
    }

    info!(count = runway_count, "Inserted runways");

    // 5. Parse and insert frequencies.
    let csv_frequencies = parse_csv::<CsvFrequency>(&frequencies_text)?;
    let mut freq_count: i32 = 0;

    let mut freqs_by_airport: HashMap<i64, Vec<&CsvFrequency>> = HashMap::new();
    for freq in &csv_frequencies {
        if seed_oa_ids.contains_key(&freq.airport_ref) {
            freqs_by_airport
                .entry(freq.airport_ref)
                .or_default()
                .push(freq);
        }
    }

    for (oa_id, freqs) in &freqs_by_airport {
        let db_id = match oa_id_to_db_id.get(oa_id) {
            Some(id) => *id,
            None => continue,
        };

        // Delete existing frequencies for this airport.
        sqlx::query("DELETE FROM frequencies WHERE airport_id = $1")
            .bind(db_id)
            .execute(pool)
            .await
            .context("Failed to delete existing frequencies")?;

        for freq in freqs {
            let mhz = match freq.frequency_mhz {
                Some(v) => Decimal::try_from(v).unwrap_or_default(),
                None => continue,
            };

            sqlx::query(
                r#"
                INSERT INTO frequencies (airport_id, freq_type, description, frequency_mhz)
                VALUES ($1, $2, $3, $4)
                "#,
            )
            .bind(db_id)
            .bind(freq.freq_type.as_deref())
            .bind(freq.description.as_deref())
            .bind(mhz)
            .execute(pool)
            .await
            .context("Failed to insert frequency")?;

            freq_count += 1;
        }
    }

    info!(count = freq_count, "Inserted frequencies");

    // 6. Parse and insert navaids.
    let csv_navaids = parse_csv::<CsvNavaid>(&navaids_text)?;
    let mut navaid_count: i32 = 0;

    // Build a reverse map from ICAO ident -> db airport id.
    let mut icao_to_db_id: HashMap<String, i32> = HashMap::new();
    for airport in &seed_airports {
        let icao = airport
            .gps_code
            .as_deref()
            .unwrap_or(&airport.ident);
        if let Some(&db_id) = oa_id_to_db_id.get(&airport.id) {
            icao_to_db_id.insert(icao.to_string(), db_id);
        }
    }

    // Group navaids by associated_airport.
    let mut navaids_by_airport: HashMap<String, Vec<&CsvNavaid>> = HashMap::new();
    for nav in &csv_navaids {
        if let Some(ref assoc) = nav.associated_airport {
            if icao_to_db_id.contains_key(assoc) {
                navaids_by_airport
                    .entry(assoc.clone())
                    .or_default()
                    .push(nav);
            }
        }
    }

    for (icao, navs) in &navaids_by_airport {
        let db_id = match icao_to_db_id.get(icao) {
            Some(id) => *id,
            None => continue,
        };

        // Delete existing navaids for this airport.
        sqlx::query("DELETE FROM navaids WHERE airport_id = $1")
            .bind(db_id)
            .execute(pool)
            .await
            .context("Failed to delete existing navaids")?;

        for nav in navs {
            sqlx::query(
                r#"
                INSERT INTO navaids (
                    airport_id, ident, name, navaid_type,
                    frequency_khz, latitude_deg, longitude_deg,
                    elevation_ft, associated_airport_icao
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                "#,
            )
            .bind(db_id)
            .bind(nav.ident.as_deref())
            .bind(nav.name.as_deref())
            .bind(nav.navaid_type.as_deref())
            .bind(parse_opt_i32(&nav.frequency_khz))
            .bind(nav.latitude_deg)
            .bind(nav.longitude_deg)
            .bind(parse_opt_i32(&nav.elevation_ft))
            .bind(nav.associated_airport.as_deref())
            .execute(pool)
            .await
            .context("Failed to insert navaid")?;

            navaid_count += 1;
        }
    }

    info!(count = navaid_count, "Inserted navaids");

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
