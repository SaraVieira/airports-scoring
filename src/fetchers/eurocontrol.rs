use anyhow::{Context, Result};
use bzip2::read::BzDecoder;
use chrono::{Datelike, NaiveDate, Utc};
use rust_decimal::Decimal;
use sqlx::PgPool;
use std::collections::HashMap;
use std::io::Read;
use tracing::{info, warn};

use crate::models::{Airport, FetchResult};

/// Eurocontrol Performance data download base URL.
const BASE_URL: &str = "https://www.eurocontrol.int/performance/data/download/csv";

/// Remote datasets to fetch (ones that aren't behind antibot).
const REMOTE_DATASETS: &[&str] = &[
    "airport_traffic",           // daily IFR traffic counts per airport
    "asma_additional_time",      // ASMA approach congestion proxy
    "taxi_out_additional_time",  // ground ops taxi-out proxy
];

/// Local apt_dly bz2 files directory (delay cause breakdown).
const APT_DLY_DIR: &str = "data/aena/ert_dly";

/// User-agent to avoid antibot blocking on some datasets.
const UA: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36";

/// Intermediate aggregation bucket keyed by (year, month).
#[derive(Debug, Default)]
struct MonthBucket {
    total_flights: i64,
    delayed_flights: i64,
    total_delay_minutes: f64,
    delay_observations: i64,
    // ATFM delay cause breakdown (minutes)
    delay_weather_min: f64,
    delay_carrier_min: f64,
    delay_atc_min: f64,
    delay_airport_min: f64,
    total_atfm_delay_min: f64,
    atfm_flights: i64,
}

/// Fetch delay and operational statistics from Eurocontrol ANS Performance CSVs
/// and local apt_dly bz2 files for delay cause breakdown.
pub async fn fetch(pool: &PgPool, airport: &Airport, full_refresh: bool) -> Result<FetchResult> {
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

    let last_run: Option<(Option<NaiveDate>,)> = sqlx::query_as(
        "SELECT last_record_date FROM pipeline_runs \
         WHERE airport_id = $1 AND source = 'eurocontrol' AND status = 'success' \
         ORDER BY completed_at DESC LIMIT 1",
    )
    .bind(airport.id)
    .fetch_optional(pool)
    .await?;

    let start_year = if full_refresh {
        2014 // apt_dly files go back to 2014
    } else {
        match last_run {
            Some((Some(d),)) => d.year(),
            _ => current_year - 2,
        }
    };

    // ── Remote datasets (airport_traffic, asma, taxi_out) ──────────
    for dataset in REMOTE_DATASETS {
        let remote_start = start_year.max(current_year - 2); // remote only has ~3 years
        for year in remote_start..=current_year {
            let url = format!("{}/{}_{}.csv", BASE_URL, dataset, year);
            info!(url = %url, "Downloading Eurocontrol CSV");

            let resp = match client.get(&url).send().await {
                Ok(r) => r,
                Err(e) => {
                    warn!(dataset = *dataset, year = year, error = %e, "Failed to download, skipping");
                    continue;
                }
            };

            if !resp.status().is_success() {
                warn!(dataset = *dataset, year = year, status = %resp.status(), "Non-success status, skipping");
                continue;
            }

            let body = resp.text().await?;
            if body.starts_with("<!DOCTYPE") || body.starts_with("<html") {
                warn!(dataset = *dataset, year = year, "Got HTML instead of CSV (antibot), skipping");
                continue;
            }

            process_csv(&body, icao, false, &mut buckets, &mut latest_date);
        }
    }

    // ── Local apt_dly bz2 files (delay cause breakdown) ────────────
    for year in start_year..=current_year {
        let path = format!("{}/apt_dly_{}.csv.bz2", APT_DLY_DIR, year);
        if !std::path::Path::new(&path).exists() {
            continue;
        }

        info!(path = %path, "Reading local apt_dly bz2");
        let file = std::fs::File::open(&path)?;
        let mut decoder = BzDecoder::new(file);
        let mut raw_bytes = Vec::new();
        if let Err(e) = decoder.read_to_end(&mut raw_bytes) {
            warn!(path = %path, error = %e, "Failed to decompress bz2, skipping");
            continue;
        }
        let csv_data = String::from_utf8_lossy(&raw_bytes);

        process_csv(&csv_data, icao, true, &mut buckets, &mut latest_date);
    }

    // ── Upsert aggregated monthly stats ────────────────────────────
    let mut records_processed: i32 = 0;

    for ((year, month), bucket) in &buckets {
        let avg_delay = if bucket.total_flights > 0 {
            let avg = bucket.total_delay_minutes / bucket.total_flights as f64;
            let mut d = Decimal::from_f64_retain(avg).unwrap_or_default();
            d.rescale(2);
            Some(d)
        } else {
            None
        };

        // delay_pct from ATFM data: delayed flights / total arriving flights
        let delay_pct = if bucket.atfm_flights > 0 && bucket.delayed_flights > 0 {
            let pct = (bucket.delayed_flights as f64 / bucket.atfm_flights as f64) * 100.0;
            let mut d = Decimal::from_f64_retain(pct.min(100.0)).unwrap_or_default();
            d.rescale(2);
            Some(d)
        } else {
            None
        };

        let delay_weather_pct = compute_cause_pct(bucket.delay_weather_min, bucket.total_atfm_delay_min);
        let delay_carrier_pct = compute_cause_pct(bucket.delay_carrier_min, bucket.total_atfm_delay_min);
        let delay_atc_pct = compute_cause_pct(bucket.delay_atc_min, bucket.total_atfm_delay_min);
        let delay_airport_pct = compute_cause_pct(bucket.delay_airport_min, bucket.total_atfm_delay_min);

        let upsert_result = sqlx::query(
            r#"
            INSERT INTO operational_stats
                (airport_id, period_year, period_month, period_type,
                 total_flights, delay_pct, avg_delay_minutes,
                 delay_weather_pct, delay_carrier_pct, delay_atc_pct, delay_airport_pct,
                 source)
            VALUES ($1, $2, $3, 'monthly', $4, $5, $6, $7, $8, $9, $10, 'eurocontrol')
            ON CONFLICT (airport_id, period_year, period_month, source)
            DO UPDATE SET
                total_flights     = EXCLUDED.total_flights,
                delay_pct         = COALESCE(EXCLUDED.delay_pct, operational_stats.delay_pct),
                avg_delay_minutes = COALESCE(EXCLUDED.avg_delay_minutes, operational_stats.avg_delay_minutes),
                delay_weather_pct = COALESCE(EXCLUDED.delay_weather_pct, operational_stats.delay_weather_pct),
                delay_carrier_pct = COALESCE(EXCLUDED.delay_carrier_pct, operational_stats.delay_carrier_pct),
                delay_atc_pct     = COALESCE(EXCLUDED.delay_atc_pct, operational_stats.delay_atc_pct),
                delay_airport_pct = COALESCE(EXCLUDED.delay_airport_pct, operational_stats.delay_airport_pct)
            "#,
        )
        .bind(airport.id)
        .bind(*year)
        .bind(Some(*month))
        .bind(bucket.total_flights as i32)
        .bind(delay_pct)
        .bind(avg_delay)
        .bind(delay_weather_pct)
        .bind(delay_carrier_pct)
        .bind(delay_atc_pct)
        .bind(delay_airport_pct)
        .execute(pool)
        .await;

        match upsert_result {
            Ok(_) => { records_processed += 1; }
            Err(e) => {
                warn!(year = *year, month = *month, error = %e, "Failed to upsert operational_stats");
            }
        }
    }

    info!(airport = icao, records = records_processed, "Eurocontrol fetch complete");

    Ok(FetchResult {
        records_processed,
        last_record_date: latest_date,
    })
}

/// Process a CSV string (from remote download or local bz2) into the buckets.
fn process_csv(
    csv_data: &str,
    icao: &str,
    is_apt_dly: bool,
    buckets: &mut HashMap<(i16, i16), MonthBucket>,
    latest_date: &mut Option<NaiveDate>,
) {
    let mut rdr = csv::ReaderBuilder::new()
        .flexible(true)
        .has_headers(true)
        .trim(csv::Trim::All)
        .from_reader(csv_data.as_bytes());

    let headers = match rdr.headers() {
        Ok(h) => h.clone(),
        Err(_) => return,
    };

    let icao_col = match find_col(&headers, &["apt_icao", "icao"]) {
        Some(c) => c,
        None => return,
    };

    let year_col = find_col(&headers, &["year"]);
    let month_col = find_col(&headers, &["month_num"]);

    // Traffic columns (airport_traffic, asma, taxi_out)
    let flight_col = find_col(&headers, &["flt_tot_1", "flt_tot_ifr_2", "tf"]);
    let delay_col = find_col(&headers, &["total_add_time_min", "flt_dly_1"]);

    // apt_dly columns
    let atfm_flights_col = find_col_exact(&headers, "flt_arr_1");
    let atfm_total_dly_col = find_col_exact(&headers, "dly_apt_arr_1");
    let delayed_flights_col = find_col_exact(&headers, "flt_arr_1_dly");
    // Cause codes
    let dly_weather_v_col = find_col_exact(&headers, "dly_apt_arr_v_1");
    let dly_weather_w_col = find_col_exact(&headers, "dly_apt_arr_w_1");
    let dly_atc_c_col = find_col_exact(&headers, "dly_apt_arr_c_1");
    let dly_atc_e_col = find_col_exact(&headers, "dly_apt_arr_e_1");
    let dly_atc_r_col = find_col_exact(&headers, "dly_apt_arr_r_1");
    let dly_atc_s_col = find_col_exact(&headers, "dly_apt_arr_s_1");
    let dly_atc_n_col = find_col_exact(&headers, "dly_apt_arr_n_1");
    let dly_carrier_a_col = find_col_exact(&headers, "dly_apt_arr_a_1");
    let dly_airport_g_col = find_col_exact(&headers, "dly_apt_arr_g_1");

    for result in rdr.records() {
        let record = match result {
            Ok(r) => r,
            Err(_) => continue,
        };

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

        // Traffic data
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

        // apt_dly data
        if is_apt_dly {
            if let Some(arr_flights) = atfm_flights_col
                .and_then(|c| record.get(c))
                .and_then(|v| v.trim().parse::<i64>().ok())
            {
                bucket.atfm_flights += arr_flights;
            }

            if let Some(delayed) = delayed_flights_col
                .and_then(|c| record.get(c))
                .and_then(|v| v.trim().parse::<i64>().ok())
            {
                bucket.delayed_flights += delayed;
            }

            if let Some(total_dly) = atfm_total_dly_col
                .and_then(|c| record.get(c))
                .and_then(|v| v.trim().parse::<f64>().ok())
            {
                bucket.total_atfm_delay_min += total_dly;
            }

            // Weather: V + W
            bucket.delay_weather_min += parse_f64_col(&record, dly_weather_v_col)
                + parse_f64_col(&record, dly_weather_w_col);

            // ATC: C + E + R + S + N
            bucket.delay_atc_min += parse_f64_col(&record, dly_atc_c_col)
                + parse_f64_col(&record, dly_atc_e_col)
                + parse_f64_col(&record, dly_atc_r_col)
                + parse_f64_col(&record, dly_atc_s_col)
                + parse_f64_col(&record, dly_atc_n_col);

            // Carrier: A
            bucket.delay_carrier_min += parse_f64_col(&record, dly_carrier_a_col);

            // Airport: G
            bucket.delay_airport_min += parse_f64_col(&record, dly_airport_g_col);
        }

        if let Some(record_date) = NaiveDate::from_ymd_opt(y as i32, m as u32, 1) {
            *latest_date = Some(match *latest_date {
                Some(prev) if record_date > prev => record_date,
                Some(prev) => prev,
                None => record_date,
            });
        }
    }
}

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

fn find_col_exact(headers: &csv::StringRecord, name: &str) -> Option<usize> {
    for (i, h) in headers.iter().enumerate() {
        if h.trim().eq_ignore_ascii_case(name) {
            return Some(i);
        }
    }
    None
}

fn parse_f64_col(record: &csv::StringRecord, col: Option<usize>) -> f64 {
    col.and_then(|c| record.get(c))
        .and_then(|v| v.trim().parse::<f64>().ok())
        .unwrap_or(0.0)
}

fn compute_cause_pct(cause_min: f64, total_min: f64) -> Option<Decimal> {
    if total_min > 0.0 && cause_min > 0.0 {
        let pct = (cause_min / total_min) * 100.0;
        let mut d = Decimal::from_f64_retain(pct).unwrap_or_default();
        d.rescale(2);
        Some(d)
    } else {
        None
    }
}
