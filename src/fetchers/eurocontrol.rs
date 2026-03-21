use anyhow::{Context, Result};
use chrono::{Datelike, NaiveDate, Utc};
use rust_decimal::Decimal;
use sqlx::PgPool;
use std::collections::HashMap;
use tracing::{info, warn};

use crate::models::{Airport, FetchResult};

/// Eurocontrol Performance data download base URL.
/// CSV files are named {dataset}_{year}.csv
const BASE_URL: &str = "https://www.eurocontrol.int/performance/data/download/csv";

/// Datasets to fetch. Each contains daily or monthly data filterable by APT_ICAO.
const DATASETS: &[&str] = &[
    "airport_traffic",       // daily IFR traffic counts per airport
    "asma_additional_time",  // ASMA approach congestion proxy
    "taxi_out_additional_time", // ground ops taxi-out proxy
];

/// User-agent to avoid antibot blocking on some datasets.
const UA: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36";

/// Intermediate aggregation bucket keyed by (year, month).
#[derive(Debug, Default)]
struct MonthBucket {
    total_flights: i64,
    delayed_flights: i64,
    total_delay_minutes: f64,
    delay_observations: i64,
}

/// Fetch delay and operational statistics from Eurocontrol ANS Performance CSVs.
pub async fn fetch(pool: &PgPool, airport: &Airport, _full_refresh: bool) -> Result<FetchResult> {
    let icao = airport
        .icao_code
        .as_deref()
        .context("Airport has no ICAO code")?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .user_agent(UA)
        .build()?;

    let mut buckets: HashMap<(i16, i16), MonthBucket> = HashMap::new();
    let mut latest_date: Option<NaiveDate> = None;

    let current_year = Utc::now().year();
    // Fetch last 3 years of data
    let start_year = current_year - 2;

    for dataset in DATASETS {
        for year in start_year..=current_year {
            let url = format!("{}/{}_{}.csv", BASE_URL, dataset, year);
            info!(url = %url, "Downloading Eurocontrol CSV");

            let resp = match client.get(&url).send().await {
                Ok(r) => r,
                Err(e) => {
                    warn!(dataset = *dataset, year = year, error = %e, "Failed to download dataset, skipping");
                    continue;
                }
            };

            if !resp.status().is_success() {
                warn!(dataset = *dataset, year = year, status = %resp.status(), "Non-success status, skipping");
                continue;
            }

            let body = resp.text().await?;

            // Check if response is HTML (antibot) rather than CSV
            if body.starts_with("<!DOCTYPE") || body.starts_with("<html") {
                warn!(dataset = *dataset, year = year, "Got HTML instead of CSV (antibot), skipping");
                continue;
            }

            let mut rdr = csv::ReaderBuilder::new()
                .flexible(true)
                .has_headers(true)
                .trim(csv::Trim::All)
                .from_reader(body.as_bytes());

            let headers = rdr.headers()?.clone();

            // Find the ICAO column
            let icao_col = find_col(&headers, &["apt_icao", "icao"]);
            let icao_col = match icao_col {
                Some(c) => c,
                None => {
                    warn!(dataset = *dataset, "No ICAO column found, skipping");
                    continue;
                }
            };

            let year_col = find_col(&headers, &["year"]);
            let month_col = find_col(&headers, &["month_num"]);
            let flight_col = find_col(&headers, &["flt_tot_1", "flt_tot_ifr_2", "tf"]);
            let delay_col = find_col(&headers, &["total_add_time_min", "flt_dly_1"]);

            for result in rdr.records() {
                let record = match result {
                    Ok(r) => r,
                    Err(_) => continue,
                };

                // Filter by airport ICAO
                let rec_icao = record.get(icao_col).unwrap_or("").trim();
                if rec_icao != icao {
                    continue;
                }

                let y: i16 = match year_col.and_then(|c| record.get(c)).and_then(|v| v.trim().parse().ok()) {
                    Some(y) => y,
                    None => continue,
                };

                let m: i16 = match month_col.and_then(|c| record.get(c)).and_then(|v| v.trim().parse().ok()) {
                    Some(m) if (1..=12).contains(&m) => m,
                    _ => continue,
                };

                let bucket = buckets.entry((y, m)).or_default();

                if let Some(flights) = flight_col
                    .and_then(|c| record.get(c))
                    .and_then(|v| v.trim().parse::<i64>().ok())
                {
                    bucket.total_flights += flights;
                }

                if let Some(delay) = delay_col
                    .and_then(|c| record.get(c))
                    .and_then(|v| v.trim().parse::<f64>().ok())
                {
                    bucket.total_delay_minutes += delay;
                    bucket.delay_observations += 1;
                }

                let record_date = NaiveDate::from_ymd_opt(y as i32, m as u32, 1).unwrap();
                latest_date = Some(match latest_date {
                    Some(prev) if record_date > prev => record_date,
                    Some(prev) => prev,
                    None => record_date,
                });
            }
        }
    }

    // Upsert aggregated monthly stats
    let mut records_processed: i32 = 0;

    for ((year, month), bucket) in &buckets {
        let avg_delay = if bucket.delay_observations > 0 {
            let avg = bucket.total_delay_minutes / bucket.delay_observations as f64;
            let mut d = Decimal::from_f64_retain(avg).unwrap_or_default();
            d.rescale(2);
            Some(d)
        } else {
            None
        };

        let delay_pct = if bucket.total_flights > 0 && bucket.delayed_flights > 0 {
            let pct = (bucket.delayed_flights as f64 / bucket.total_flights as f64) * 100.0;
            let mut d = Decimal::from_f64_retain(pct).unwrap_or_default();
            d.rescale(2);
            Some(d)
        } else {
            None
        };

        let upsert_result = sqlx::query(
            r#"
            INSERT INTO operational_stats
                (airport_id, period_year, period_month, period_type,
                 total_flights, delay_pct, avg_delay_minutes, source)
            VALUES ($1, $2, $3, 'monthly', $4, $5, $6, 'eurocontrol')
            ON CONFLICT (airport_id, period_year, period_month, source)
            DO UPDATE SET
                total_flights     = EXCLUDED.total_flights,
                delay_pct         = EXCLUDED.delay_pct,
                avg_delay_minutes = EXCLUDED.avg_delay_minutes
            "#,
        )
        .bind(airport.id)
        .bind(*year)
        .bind(Some(*month))
        .bind(bucket.total_flights as i32)
        .bind(delay_pct)
        .bind(avg_delay)
        .execute(pool)
        .await;

        match upsert_result {
            Ok(_) => { records_processed += 1; }
            Err(e) => {
                warn!(
                    year = *year, month = *month,
                    error = %e,
                    "Failed to upsert operational_stats, skipping month"
                );
            }
        }
    }

    info!(
        airport = icao,
        records = records_processed,
        "Eurocontrol fetch complete"
    );

    Ok(FetchResult {
        records_processed,
        last_record_date: latest_date,
    })
}

/// Find a column index by trying multiple candidate header names.
fn find_col(headers: &csv::StringRecord, candidates: &[&str]) -> Option<usize> {
    for (i, h) in headers.iter().enumerate() {
        let lower = h.trim().to_lowercase();
        for &candidate in candidates {
            if lower == candidate || lower.contains(candidate) {
                return Some(i);
            }
        }
    }
    None
}
