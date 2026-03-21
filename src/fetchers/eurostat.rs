use anyhow::{Context, Result};
use chrono::NaiveDate;
use serde_json::Value;
use sqlx::PgPool;
use tracing::{info, warn};

use crate::models::{Airport, FetchResult};

/// Eurostat REST API base for aviation passenger data.
/// Dataset avia_paoa: air passenger transport by reporting airport.
const EUROSTAT_API: &str =
    "https://ec.europa.eu/eurostat/api/dissemination/statistics/1.0/data/avia_paoa";

/// Fetch passenger traffic statistics from Eurostat.
/// Uses rep_airp dimension with format {COUNTRY}_{ICAO}.
pub async fn fetch(pool: &PgPool, airport: &Airport, _full_refresh: bool) -> Result<FetchResult> {
    let iata = airport
        .iata_code
        .as_deref()
        .context("Airport has no IATA code")?;
    let icao = airport
        .icao_code
        .as_deref()
        .context("Airport has no ICAO code for Eurostat lookup")?;

    let country = &airport.country_code;

    // Eurostat rep_airp format: {COUNTRY}_{ICAO}, e.g. DE_EDDM, FR_LFPG
    let rep_airp = format!("{}_{}", country, icao);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()?;

    let url = format!(
        "{}?format=JSON&lang=en&freq=A&unit=PAS&tra_meas=PAS_CRD&schedule=TOT&tra_cov=TOTAL&rep_airp={}&sinceTimePeriod=2000",
        EUROSTAT_API, rep_airp
    );

    info!(url = %url, "Querying Eurostat API");

    let resp = match client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => {
            warn!(error = %e, "Eurostat request failed");
            return Ok(FetchResult {
                records_processed: 0,
                last_record_date: None,
            });
        }
    };

    if !resp.status().is_success() {
        warn!(
            status = %resp.status(),
            rep_airp = rep_airp,
            "Eurostat non-success response"
        );
        return Ok(FetchResult {
            records_processed: 0,
            last_record_date: None,
        });
    }

    let body: Value = match resp.json().await {
        Ok(v) => v,
        Err(e) => {
            warn!(error = %e, "Failed to parse Eurostat JSON");
            return Ok(FetchResult {
                records_processed: 0,
                last_record_date: None,
            });
        }
    };

    let time_index = body
        .pointer("/dimension/time/category/index")
        .and_then(|v| v.as_object());

    let values = body.get("value").and_then(|v| v.as_object());

    let (time_index, values) = match (time_index, values) {
        (Some(t), Some(v)) => (t, v),
        _ => {
            warn!(rep_airp = rep_airp, "No time/value data in Eurostat response");
            return Ok(FetchResult {
                records_processed: 0,
                last_record_date: None,
            });
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

    let mut records_processed: i32 = 0;
    let mut latest_date: Option<NaiveDate> = None;

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
