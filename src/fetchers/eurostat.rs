use anyhow::{Context, Result};
use chrono::NaiveDate;
use serde_json::Value;
use sqlx::PgPool;
use tracing::{info, warn};

use crate::models::{Airport, FetchResult};

/// Eurostat REST API base for aviation passenger data.
const EUROSTAT_API: &str =
    "https://ec.europa.eu/eurostat/api/dissemination/statistics/1.0/data/avia_paoa";

/// Fetch passenger traffic statistics from Eurostat.
pub async fn fetch(pool: &PgPool, airport: &Airport, _full_refresh: bool) -> Result<FetchResult> {
    // Eurostat uses IATA-style airport codes (e.g., "LHR", "CDG")
    // but in their own format which may be country-prefixed like "UK_LHR"
    // or just the IATA code. We try both.
    let iata = airport
        .iata_code
        .as_deref()
        .context("Airport has no IATA code for Eurostat lookup")?;

    let country = &airport.country_code;

    // Eurostat airport codes can be like "UK_LHR" or just "LHR" depending on dataset
    let airport_codes = vec![
        format!("{}_{}", country, iata),
        iata.to_string(),
    ];

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()?;

    let mut records_processed: i32 = 0;
    let mut latest_date: Option<NaiveDate> = None;

    for airport_code in &airport_codes {
        let url = format!(
            "{}?format=JSON&lang=en&freq=A&unit=PAS&tra_meas=PAS_CRD&airport={}",
            EUROSTAT_API, airport_code
        );

        info!(url = %url, "Querying Eurostat API");

        let resp = match client.get(&url).send().await {
            Ok(r) => r,
            Err(e) => {
                warn!(error = %e, "Eurostat request failed");
                continue;
            }
        };

        if !resp.status().is_success() {
            warn!(
                status = %resp.status(),
                airport_code = airport_code,
                "Eurostat non-success response"
            );
            continue;
        }

        let body: Value = match resp.json().await {
            Ok(v) => v,
            Err(e) => {
                warn!(error = %e, "Failed to parse Eurostat JSON");
                continue;
            }
        };

        // Eurostat JSON format has:
        // - "dimension" -> "time" -> "category" -> "index" mapping year strings to indices
        // - "value" mapping string indices to numeric values
        let time_index = body
            .pointer("/dimension/time/category/index")
            .and_then(|v| v.as_object());

        let values = body.get("value").and_then(|v| v.as_object());

        let (time_index, values) = match (time_index, values) {
            (Some(t), Some(v)) => (t, v),
            _ => {
                warn!(airport_code = airport_code, "No time/value data in Eurostat response");
                continue;
            }
        };

        // Build reverse map: index -> year
        let mut index_to_year: std::collections::HashMap<String, i16> =
            std::collections::HashMap::new();
        for (year_str, idx_val) in time_index {
            if let Ok(year) = year_str.parse::<i16>() {
                if let Some(idx) = idx_val.as_u64() {
                    index_to_year.insert(idx.to_string(), year);
                }
            }
        }

        for (idx_str, pax_val) in values {
            let year = match index_to_year.get(idx_str) {
                Some(y) => *y,
                None => continue,
            };

            let total_pax = match pax_val.as_f64() {
                Some(v) => v as i64,
                None => continue,
            };

            sqlx::query(
                r#"
                INSERT INTO pax_yearly (airport_id, year, total_pax, source)
                VALUES ($1, $2, $3, 'eurostat')
                ON CONFLICT (airport_id, year) DO UPDATE SET
                    total_pax = EXCLUDED.total_pax
                "#,
            )
            .bind(airport.id)
            .bind(year)
            .bind(total_pax)
            .execute(pool)
            .await
            .with_context(|| format!("Failed to upsert pax_yearly for year {}", year))?;

            records_processed += 1;

            let year_date = NaiveDate::from_ymd_opt(year as i32, 12, 31).unwrap();
            latest_date = Some(match latest_date {
                Some(prev) if year_date > prev => year_date,
                Some(prev) => prev,
                None => year_date,
            });
        }

        // If we got data with this code variant, no need to try the other
        if records_processed > 0 {
            break;
        }
    }

    info!(
        airport = iata,
        records = records_processed,
        "Eurostat fetch complete"
    );

    Ok(FetchResult {
        records_processed,
        last_record_date: latest_date,
    })
}
