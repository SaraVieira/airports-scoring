//! Statistics Denmark (DST) passenger data fetcher.
//!
//! Downloads annual passenger counts per airport from the DST StatBank API.
//! Table FLYV31: Passengers on bigger public, manned Danish airports.
//! Free CSV API, no authentication needed.

use anyhow::{Context, Result};
use chrono::{Datelike, NaiveDate, Utc};
use sqlx::PgPool;
use tracing::{info, warn};

use crate::models::{Airport, FetchResult};

const DST_API_URL: &str = "https://api.statbank.dk/v1/data";

/// Map IATA code to DST airport code.
fn iata_to_dst_code(iata: &str) -> Option<&'static str> {
    match iata {
        "CPH" => Some("10015"),
        "BLL" => Some("10020"),
        "AAR" => Some("10025"),
        "AAL" => Some("10030"),
        "KRP" => Some("10035"),
        "EBJ" => Some("10040"),
        "RNN" => Some("10045"),
        "SGD" => Some("10050"),
        "RKE" => Some("10055"),
        "TED" => Some("10060"),
        "ODE" => Some("10065"),
        _ => None,
    }
}

/// Fetch passenger data from Statistics Denmark for a Danish airport.
/// Only runs for airports with country_code "DK".
pub async fn fetch(pool: &PgPool, airport: &Airport, _full_refresh: bool) -> Result<FetchResult> {
    let iata = airport
        .iata_code
        .as_deref()
        .context("Airport has no IATA code")?;

    if airport.country_code != "DK" {
        return Ok(FetchResult {
            records_processed: 0,
            last_record_date: None,
        });
    }

    let dst_code = match iata_to_dst_code(iata) {
        Some(code) => code,
        None => {
            info!(airport = iata, "No DST mapping for this Danish airport, skipping");
            return Ok(FetchResult {
                records_processed: 0,
                last_record_date: None,
            });
        }
    };

    // DST publishes annual data — current year is usually not available yet
    let last_year = Utc::now().year() - 1;
    let start_year = last_year - 9;
    let year_values: Vec<String> = (start_year..=last_year)
        .map(|y| y.to_string())
        .collect();

    // Request total passengers (7010) + departing passengers (7030)
    // DST reports in thousands
    let query = serde_json::json!({
        "table": "FLYV31",
        "format": "CSV",
        "lang": "en",
        "variables": [
            { "code": "LUFTHAVN", "values": [dst_code] },
            { "code": "PASSAGER", "values": ["7010"] },
            { "code": "Tid", "values": year_values }
        ]
    });

    info!(airport = iata, dst_code, "Fetching DST Denmark passenger data");

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let resp = client
        .post(DST_API_URL)
        .json(&query)
        .send()
        .await
        .context("Failed to call DST API")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        warn!(airport = iata, status = %status, body = %body, "DST API error");
        anyhow::bail!("DST API returned {status} for {iata}");
    }

    let body = resp.text().await?;

    // Parse CSV: LUFTHAVN;PASSAGER;TID;INDHOLD
    // Values are in thousands
    let mut records = 0i32;
    let mut latest_date: Option<NaiveDate> = None;

    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b';')
        .has_headers(true)
        .from_reader(body.as_bytes());

    for result in rdr.records() {
        let record = match result {
            Ok(r) => r,
            Err(e) => {
                warn!(airport = iata, error = %e, "Failed to parse DST CSV row");
                continue;
            }
        };

        let year: i16 = match record.get(2).and_then(|v| v.trim().parse().ok()) {
            Some(y) => y,
            None => continue,
        };

        let pax_thousands: i64 = match record.get(3).and_then(|v| v.trim().parse().ok()) {
            Some(p) => p,
            None => continue,
        };

        // DST reports in thousands
        let total_pax = pax_thousands * 1000;

        if total_pax == 0 {
            continue;
        }

        sqlx::query(
            r#"
            INSERT INTO pax_yearly (airport_id, year, total_pax, source)
            VALUES ($1, $2, $3, 'dst_denmark')
            ON CONFLICT (airport_id, year) DO UPDATE SET
                total_pax = GREATEST(EXCLUDED.total_pax, pax_yearly.total_pax),
                source = 'dst_denmark'
            "#,
        )
        .bind(airport.id)
        .bind(year)
        .bind(total_pax)
        .execute(pool)
        .await
        .with_context(|| format!("Failed to upsert DST pax for {iata} year {year}"))?;

        records += 1;
        let d = NaiveDate::from_ymd_opt(year as i32, 12, 31).unwrap();
        latest_date = Some(match latest_date {
            Some(prev) if d > prev => d,
            Some(prev) => prev,
            None => d,
        });
    }

    info!(airport = iata, records, "DST Denmark pax data upserted");

    Ok(FetchResult {
        records_processed: records,
        last_record_date: latest_date,
    })
}
