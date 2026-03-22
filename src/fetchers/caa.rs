use anyhow::{Context, Result};
use chrono::NaiveDate;
use sqlx::PgPool;
use std::collections::HashMap;
use tracing::{info, warn};

use crate::models::{Airport, FetchResult};

/// Local CAA data directory.
const CAA_DIR: &str = "data/caa";

/// Map IATA codes to CAA airport names (uppercase as they appear in CSVs).
fn iata_to_caa_name(iata: &str) -> Option<&'static str> {
    match iata {
        "LHR" => Some("HEATHROW"),
        "LGW" => Some("GATWICK"),
        "LTN" => Some("LUTON"),
        "STN" => Some("STANSTED"),
        "MAN" => Some("MANCHESTER"),
        "BHX" => Some("BIRMINGHAM"),
        "EDI" => Some("EDINBURGH"),
        "BRS" => Some("BRISTOL"),
        "GLA" => Some("GLASGOW"),
        _ => None,
    }
}

/// Aggregated data for one airport-year.
#[derive(Debug, Default)]
struct CaaYear {
    international_pax: Option<i64>,
    domestic_pax: Option<i64>,
    aircraft_movements: Option<i32>,
}

/// Fetch UK CAA airport statistics from local CSV files.
/// Reads table_10_1 (international pax), table_10_2 (domestic pax),
/// and table_03_1 (aircraft movements) from data/caa/{year}/.
pub async fn fetch(pool: &PgPool, airport: &Airport, _full_refresh: bool) -> Result<FetchResult> {
    let iata = airport
        .iata_code
        .as_deref()
        .context("Airport has no IATA code")?;

    // Only process UK airports
    if airport.country_code != "GB" {
        return Ok(FetchResult {
            records_processed: 0,
            last_record_date: None,
        });
    }

    let caa_name = match iata_to_caa_name(iata) {
        Some(name) => name,
        None => {
            warn!(airport = iata, "No CAA name mapping for this airport");
            return Ok(FetchResult {
                records_processed: 0,
                last_record_date: None,
            });
        }
    };

    let mut years: HashMap<i16, CaaYear> = HashMap::new();

    // Scan all year directories
    let caa_path = std::path::Path::new(CAA_DIR);
    if !caa_path.exists() {
        warn!("CAA data directory not found: {}", CAA_DIR);
        return Ok(FetchResult {
            records_processed: 0,
            last_record_date: None,
        });
    }

    let mut year_dirs: Vec<i16> = Vec::new();
    for entry in std::fs::read_dir(caa_path)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            if let Some(name) = entry.file_name().to_str() {
                if let Ok(year) = name.parse::<i16>() {
                    year_dirs.push(year);
                }
            }
        }
    }
    year_dirs.sort();

    for year in &year_dirs {
        let dir = format!("{}/{}", CAA_DIR, year);

        // Table 10.1 — International passengers
        let intl_path = format!("{}/table_10_1.csv", dir);
        if let Some(pax) = parse_pax_from_table(&intl_path, caa_name) {
            years.entry(*year).or_default().international_pax = Some(pax);
        }

        // Table 10.2 — Domestic passengers
        let dom_path = format!("{}/table_10_2.csv", dir);
        if let Some(pax) = parse_pax_from_table(&dom_path, caa_name) {
            years.entry(*year).or_default().domestic_pax = Some(pax);
        }

        // Table 03.1 — Aircraft movements
        let mov_path = format!("{}/table_03_1.csv", dir);
        if let Some(mov) = parse_movements(&mov_path, caa_name) {
            years.entry(*year).or_default().aircraft_movements = Some(mov);
        }
    }

    // Upsert into pax_yearly
    let mut records_processed: i32 = 0;
    let mut latest_date: Option<NaiveDate> = None;

    for (year, data) in &years {
        let total_pax = match (data.international_pax, data.domestic_pax) {
            (Some(i), Some(d)) => Some(i + d),
            (Some(i), None) => Some(i),
            (None, Some(d)) => Some(d),
            (None, None) => None,
        };

        if total_pax.is_none() && data.aircraft_movements.is_none() {
            continue;
        }

        sqlx::query(
            r#"
            INSERT INTO pax_yearly (airport_id, year, total_pax, international_pax, domestic_pax, aircraft_movements, source)
            VALUES ($1, $2, $3, $4, $5, $6, 'caa')
            ON CONFLICT (airport_id, year) DO UPDATE SET
                total_pax = COALESCE(EXCLUDED.total_pax, pax_yearly.total_pax),
                international_pax = COALESCE(EXCLUDED.international_pax, pax_yearly.international_pax),
                domestic_pax = COALESCE(EXCLUDED.domestic_pax, pax_yearly.domestic_pax),
                aircraft_movements = COALESCE(EXCLUDED.aircraft_movements, pax_yearly.aircraft_movements)
            "#,
        )
        .bind(airport.id)
        .bind(*year)
        .bind(total_pax)
        .bind(data.international_pax)
        .bind(data.domestic_pax)
        .bind(data.aircraft_movements)
        .execute(pool)
        .await
        .with_context(|| format!("Failed to upsert CAA data for year {}", year))?;

        records_processed += 1;

        let year_date = NaiveDate::from_ymd_opt(*year as i32, 12, 31).unwrap();
        latest_date = Some(match latest_date {
            Some(prev) if year_date > prev => year_date,
            Some(prev) => prev,
            None => year_date,
        });
    }

    info!(
        airport = iata,
        records = records_processed,
        years = years.len(),
        "CAA fetch complete"
    );

    Ok(FetchResult {
        records_processed,
        last_record_date: latest_date,
    })
}

/// Parse a passenger count from a CAA CSV table for a given airport.
/// Finds the airport row by `rpt_apt_name` and reads `total_pax_tp`.
fn parse_pax_from_table(path: &str, airport_name: &str) -> Option<i64> {
    if !std::path::Path::new(path).exists() {
        return None;
    }

    let mut rdr = csv::ReaderBuilder::new()
        .flexible(true)
        .has_headers(true)
        .trim(csv::Trim::All)
        .from_path(path)
        .ok()?;

    let headers = rdr.headers().ok()?.clone();

    let name_idx = headers.iter().position(|h| h.trim().eq_ignore_ascii_case("rpt_apt_name"))?;
    let val_idx = headers.iter().position(|h| h.trim().eq_ignore_ascii_case("total_pax_tp"))?;

    for record in rdr.records().flatten() {
        let name = record.get(name_idx)?.trim();
        if name.eq_ignore_ascii_case(airport_name) {
            let val = record.get(val_idx)?.trim();
            let clean: String = val.chars().filter(|c| c.is_ascii_digit()).collect();
            return clean.parse::<i64>().ok();
        }
    }

    None
}

/// Parse aircraft movements (grand_total) from table_03_1 for a given airport.
fn parse_movements(path: &str, airport_name: &str) -> Option<i32> {
    if !std::path::Path::new(path).exists() {
        return None;
    }

    let mut rdr = csv::ReaderBuilder::new()
        .flexible(true)
        .has_headers(true)
        .trim(csv::Trim::All)
        .from_path(path)
        .ok()?;

    let headers = rdr.headers().ok()?.clone();

    let name_idx = headers.iter().position(|h| h.trim().eq_ignore_ascii_case("rpt_apt_name"))?;
    let mov_idx = headers.iter().position(|h| {
        let h = h.trim().to_lowercase();
        h == "grand_total" || h == "total"
    })?;

    for record in rdr.records().flatten() {
        let name = record.get(name_idx)?.trim();
        if name.eq_ignore_ascii_case(airport_name) {
            let val = record.get(mov_idx)?.trim();
            let clean: String = val.chars().filter(|c| c.is_ascii_digit()).collect();
            return clean.parse::<i32>().ok();
        }
    }

    None
}
