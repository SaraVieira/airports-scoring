use anyhow::{Context, Result};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use sqlx::PgPool;
use std::collections::HashMap;
use tracing::{info, warn};

use crate::models::{Airport, FetchResult};

/// Known Eurocontrol ANS Performance CSV dataset base URLs.
const BASE_URL: &str = "https://ansperformance.eu/csv";

/// Dataset filenames we care about.
const DATASETS: &[&str] = &[
    "apt_dly.csv",  // airport traffic / daily traffic counts
    "apt_dly_atfm.csv", // arrival ATFM delays
    "asma.csv",     // ASMA additional time
    "txout.csv",    // taxi-out additional time
];

/// Intermediate aggregation bucket keyed by (year, month).
#[derive(Debug, Default)]
struct MonthBucket {
    total_flights: i64,
    delayed_flights: i64,
    total_delay_minutes: f64,
    delay_observations: i64,
}

/// Fetch delay and operational statistics from Eurocontrol.
pub async fn fetch(pool: &PgPool, airport: &Airport, _full_refresh: bool) -> Result<FetchResult> {
    let icao = airport
        .icao_code
        .as_deref()
        .context("Airport has no ICAO code")?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()?;

    let mut buckets: HashMap<(i16, i16), MonthBucket> = HashMap::new();
    let mut latest_date: Option<NaiveDate> = None;

    for dataset in DATASETS {
        let url = format!("{}/{}", BASE_URL, dataset);
        info!(url = %url, "Downloading Eurocontrol CSV");

        let resp = match client.get(&url).send().await {
            Ok(r) => r,
            Err(e) => {
                warn!(dataset = dataset, error = %e, "Failed to download dataset, skipping");
                continue;
            }
        };

        if !resp.status().is_success() {
            warn!(dataset = dataset, status = %resp.status(), "Non-success status, skipping");
            continue;
        }

        let body = resp.text().await?;
        let mut rdr = csv::ReaderBuilder::new()
            .flexible(true)
            .has_headers(true)
            .from_reader(body.as_bytes());

        let headers = rdr.headers()?.clone();

        // Find relevant column indices (case-insensitive partial match)
        let icao_col = find_col(&headers, &["apt_icao", "icao", "airport"]);
        let year_col = find_col(&headers, &["year"]);
        let month_col = find_col(&headers, &["month_num", "month"]);
        let flights_col = find_col(&headers, &["flt_tot", "total_flights", "flights", "flt"]);
        let delay_col = find_col(&headers, &["dly_atfm", "avg_add", "delay", "avg_atfm"]);

        let icao_col = match icao_col {
            Some(c) => c,
            None => {
                warn!(dataset = dataset, "No ICAO column found, skipping");
                continue;
            }
        };

        for result in rdr.records() {
            let record = match result {
                Ok(r) => r,
                Err(_) => continue,
            };

            // Check if this record matches our airport
            let rec_icao = record.get(icao_col).unwrap_or("").trim();
            if !rec_icao.eq_ignore_ascii_case(icao) {
                continue;
            }

            let year: i16 = match year_col.and_then(|c| record.get(c)) {
                Some(v) => match v.trim().parse() {
                    Ok(y) => y,
                    Err(_) => continue,
                },
                None => continue,
            };

            let month: i16 = match month_col.and_then(|c| record.get(c)) {
                Some(v) => match v.trim().parse() {
                    Ok(m) if (1..=12).contains(&m) => m,
                    _ => continue,
                },
                None => continue,
            };

            let bucket = buckets.entry((year, month)).or_default();

            if let Some(col) = flights_col {
                if let Some(val) = record.get(col) {
                    if let Ok(f) = val.trim().replace(',', "").parse::<i64>() {
                        bucket.total_flights += f;
                    }
                }
            }

            if let Some(col) = delay_col {
                if let Some(val) = record.get(col) {
                    if let Ok(d) = val.trim().parse::<f64>() {
                        bucket.total_delay_minutes += d;
                        bucket.delay_observations += 1;
                        if d > 0.0 {
                            bucket.delayed_flights += 1;
                        }
                    }
                }
            }

            // Track the latest date for pipeline state
            if let Some(d) = NaiveDate::from_ymd_opt(year as i32, month as u32, 1) {
                latest_date = Some(match latest_date {
                    Some(prev) if d > prev => d,
                    Some(prev) => prev,
                    None => d,
                });
            }
        }
    }

    // Upsert aggregated data into operational_stats
    let mut records_processed: i32 = 0;

    for ((year, month), bucket) in &buckets {
        let total = if bucket.total_flights > 0 {
            Some(bucket.total_flights as i32)
        } else {
            None
        };

        let delayed = if bucket.delayed_flights > 0 {
            Some(bucket.delayed_flights as i32)
        } else {
            None
        };

        let delay_pct = match (bucket.delayed_flights, bucket.total_flights) {
            (d, t) if t > 0 => {
                Some(Decimal::from_f64_retain(d as f64 / t as f64 * 100.0).unwrap_or_default())
            }
            _ => None,
        };

        let avg_delay = if bucket.delay_observations > 0 {
            Some(
                Decimal::from_f64_retain(
                    bucket.total_delay_minutes / bucket.delay_observations as f64,
                )
                .unwrap_or_default(),
            )
        } else {
            None
        };

        sqlx::query(
            r#"
            INSERT INTO operational_stats
                (airport_id, period_year, period_month, period_type,
                 total_flights, delayed_flights, delay_pct, avg_delay_minutes, source)
            VALUES ($1, $2, $3, 'monthly', $4, $5, $6, $7, 'eurocontrol')
            ON CONFLICT (airport_id, period_year, period_month, source)
                WHERE period_month IS NOT NULL
            DO UPDATE SET
                total_flights     = EXCLUDED.total_flights,
                delayed_flights   = EXCLUDED.delayed_flights,
                delay_pct         = EXCLUDED.delay_pct,
                avg_delay_minutes = EXCLUDED.avg_delay_minutes
            "#,
        )
        .bind(airport.id)
        .bind(year)
        .bind(month)
        .bind(total)
        .bind(delayed)
        .bind(delay_pct)
        .bind(avg_delay)
        .execute(pool)
        .await
        .with_context(|| format!("Failed to upsert operational_stats for {}-{}", year, month))?;

        records_processed += 1;
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

/// Find the first matching column index from a list of possible names.
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
