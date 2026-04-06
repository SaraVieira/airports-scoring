use anyhow::{Context, Result};
use chrono::{Datelike, NaiveDate, Utc};
use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use sqlx::PgPool;
use std::collections::HashMap;
use tracing::{info, warn};

use crate::models::{Airport, FetchResult};

pub(crate) mod csv_parser;
pub mod sync;
use csv_parser::compute_cause_pct;

/// Intermediate aggregation bucket keyed by (year, month).
#[derive(Debug, Default)]
struct MonthBucket {
    total_flights: i64,
    delayed_flights: i64,
    // ATFM delay cause breakdown (minutes)
    delay_weather_min: f64,
    delay_carrier_min: f64,
    delay_atc_min: f64,
    delay_airport_min: f64,
    total_atfm_delay_min: f64,
    atfm_flights: i64,
    // ASMA / Taxi / Slot / VFE
    asma_additional_min: f64,
    asma_flights: i64,
    taxi_out_additional_min: f64,
    taxi_out_flights: i64,
    taxi_in_additional_min: f64,
    taxi_in_flights: i64,
    cdo_flights: i64,
    cco_flights: i64,
    total_flights_vfe_descent: i64,
    total_flights_vfe_climb: i64,
    delta_co2_kg_descent: f64,
    delta_co2_kg_climb: f64,
    slot_early: i64,
    slot_on_time: i64,
    slot_late: i64,
}

/// Raw row from eurocontrol_raw table.
#[derive(sqlx::FromRow)]
struct RawRow {
    dataset: String,
    year: i16,
    month: i16,
    total_flights: Option<i32>,
    additional_time_min: Option<Decimal>,
    reference_flights: Option<i32>,
    arr_flights: Option<i32>,
    delayed_flights: Option<i32>,
    total_atfm_delay_min: Option<Decimal>,
    dly_weather_min: Option<Decimal>,
    dly_atc_min: Option<Decimal>,
    dly_carrier_min: Option<Decimal>,
    dly_airport_min: Option<Decimal>,
    cdo_flights: Option<i32>,
    cco_flights: Option<i32>,
    total_flights_vfe: Option<i32>,
    delta_co2_kg_descent: Option<Decimal>,
    delta_co2_kg_climb: Option<Decimal>,
    slot_early: Option<i32>,
    slot_on_time: Option<i32>,
    slot_late: Option<i32>,
}

/// Fetch operational statistics from the local eurocontrol_raw table
/// (populated by `sync-eurocontrol`) and aggregate into operational_stats.
pub async fn fetch(pool: &PgPool, airport: &Airport, full_refresh: bool) -> Result<FetchResult> {
    let icao = airport
        .icao_code
        .as_deref()
        .context("Airport has no ICAO code")?;

    let current_year = Utc::now().year() as i16;

    let last_run: Option<(Option<NaiveDate>,)> = sqlx::query_as(
        "SELECT last_record_date FROM pipeline_runs \
         WHERE airport_id = $1 AND source = 'eurocontrol' AND status = 'success' \
         ORDER BY completed_at DESC LIMIT 1",
    )
    .bind(airport.id)
    .fetch_optional(pool)
    .await?;

    let start_year = if full_refresh {
        2014_i16
    } else {
        match last_run {
            Some((Some(d),)) => d.year() as i16,
            _ => current_year - 2,
        }
    };

    // Check if eurocontrol_raw has data; if not, warn and return early
    let raw_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM eurocontrol_raw WHERE apt_icao = $1 AND year >= $2",
    )
    .bind(icao)
    .bind(start_year)
    .fetch_one(pool)
    .await?;

    if raw_count.0 == 0 {
        warn!(
            airport = icao,
            "No eurocontrol_raw data found. Run `sync-eurocontrol` first."
        );
        return Ok(FetchResult {
            records_processed: 0,
            last_record_date: None,
        });
    }

    // ── Read from eurocontrol_raw ────────────────────────────────
    let rows: Vec<RawRow> = sqlx::query_as(
        "SELECT dataset, year, month, total_flights, \
                additional_time_min, reference_flights, \
                arr_flights, delayed_flights, total_atfm_delay_min, \
                dly_weather_min, dly_atc_min, dly_carrier_min, dly_airport_min, \
                cdo_flights, cco_flights, total_flights_vfe, \
                delta_co2_kg_descent, delta_co2_kg_climb, \
                slot_early, slot_on_time, slot_late \
         FROM eurocontrol_raw \
         WHERE apt_icao = $1 AND year >= $2 \
         ORDER BY year, month",
    )
    .bind(icao)
    .bind(start_year)
    .fetch_all(pool)
    .await?;

    info!(airport = icao, rows = rows.len(), "Read eurocontrol_raw data");

    // ── Aggregate into monthly buckets ───────────────────────────
    let mut buckets: HashMap<(i16, i16), MonthBucket> = HashMap::new();
    let mut latest_date: Option<NaiveDate> = None;

    for row in &rows {
        let bucket = buckets.entry((row.year, row.month)).or_default();

        match row.dataset.as_str() {
            "airport_traffic" => {
                if let Some(f) = row.total_flights {
                    bucket.total_flights += f as i64;
                }
            }
            "asma_additional_time" => {
                if let Some(add) = row.additional_time_min.as_ref().and_then(|d| d.to_f64()) {
                    bucket.asma_additional_min += add;
                    bucket.asma_flights += row.reference_flights.unwrap_or(0) as i64;
                }
            }
            "taxi_out_additional_time" => {
                if let Some(add) = row.additional_time_min.as_ref().and_then(|d| d.to_f64()) {
                    bucket.taxi_out_additional_min += add;
                    bucket.taxi_out_flights += row.reference_flights.unwrap_or(0) as i64;
                }
            }
            "taxi_in_additional_time" => {
                if let Some(add) = row.additional_time_min.as_ref().and_then(|d| d.to_f64()) {
                    bucket.taxi_in_additional_min += add;
                    bucket.taxi_in_flights += row.reference_flights.unwrap_or(0) as i64;
                }
            }
            "apt_dly" => {
                if let Some(f) = row.arr_flights {
                    bucket.atfm_flights += f as i64;
                }
                if let Some(d) = row.delayed_flights {
                    bucket.delayed_flights += d as i64;
                }
                if let Some(v) = row.total_atfm_delay_min.as_ref().and_then(|d| d.to_f64()) {
                    bucket.total_atfm_delay_min += v;
                }
                if let Some(v) = row.dly_weather_min.as_ref().and_then(|d| d.to_f64()) {
                    bucket.delay_weather_min += v;
                }
                if let Some(v) = row.dly_atc_min.as_ref().and_then(|d| d.to_f64()) {
                    bucket.delay_atc_min += v;
                }
                if let Some(v) = row.dly_carrier_min.as_ref().and_then(|d| d.to_f64()) {
                    bucket.delay_carrier_min += v;
                }
                if let Some(v) = row.dly_airport_min.as_ref().and_then(|d| d.to_f64()) {
                    bucket.delay_airport_min += v;
                }
            }
            "vertical_flight_efficiency" => {
                if let Some(f) = row.total_flights_vfe {
                    bucket.total_flights_vfe_descent += f as i64;
                }
                if let Some(f) = row.total_flights {
                    // climb flights stored in total_flights for VFE
                    bucket.total_flights_vfe_climb += f as i64;
                }
                if let Some(f) = row.cdo_flights {
                    bucket.cdo_flights += f as i64;
                }
                if let Some(f) = row.cco_flights {
                    bucket.cco_flights += f as i64;
                }
                if let Some(v) = row.delta_co2_kg_descent.as_ref().and_then(|d| d.to_f64()) {
                    bucket.delta_co2_kg_descent += v;
                }
                if let Some(v) = row.delta_co2_kg_climb.as_ref().and_then(|d| d.to_f64()) {
                    bucket.delta_co2_kg_climb += v;
                }
            }
            "atfm_slot_adherence" => {
                if let Some(v) = row.slot_early {
                    bucket.slot_early += v as i64;
                }
                if let Some(v) = row.slot_on_time {
                    bucket.slot_on_time += v as i64;
                }
                if let Some(v) = row.slot_late {
                    bucket.slot_late += v as i64;
                }
            }
            _ => {}
        }

        if let Some(d) = NaiveDate::from_ymd_opt(row.year as i32, row.month as u32, 1) {
            latest_date = Some(match latest_date {
                Some(prev) if d > prev => d,
                Some(prev) => prev,
                None => d,
            });
        }
    }

    // ── Upsert aggregated monthly stats ──────────────────────────
    let mut records_processed: i32 = 0;

    for ((year, month), bucket) in &buckets {
        let avg_delay = if bucket.atfm_flights > 0 && bucket.total_atfm_delay_min > 0.0 {
            let avg = bucket.total_atfm_delay_min / bucket.atfm_flights as f64;
            Some(dec2(avg))
        } else {
            None
        };

        let delay_pct = if bucket.atfm_flights > 0 && bucket.delayed_flights > 0 {
            let pct = (bucket.delayed_flights as f64 / bucket.atfm_flights as f64) * 100.0;
            let mut d = Decimal::from_f64_retain(pct.min(100.0)).unwrap_or_default();
            d.rescale(2);
            Some(d)
        } else {
            None
        };

        let delay_weather_pct =
            compute_cause_pct(bucket.delay_weather_min, bucket.total_atfm_delay_min);
        let delay_carrier_pct =
            compute_cause_pct(bucket.delay_carrier_min, bucket.total_atfm_delay_min);
        let delay_atc_pct =
            compute_cause_pct(bucket.delay_atc_min, bucket.total_atfm_delay_min);
        let delay_airport_pct =
            compute_cause_pct(bucket.delay_airport_min, bucket.total_atfm_delay_min);

        // ASMA: additional time per flight
        let asma_additional = if bucket.asma_flights > 0 {
            let avg = bucket.asma_additional_min / bucket.asma_flights as f64;
            Some(dec2(avg))
        } else {
            None
        };

        // Taxi-out: additional time per flight
        let taxi_out_additional = if bucket.taxi_out_flights > 0 {
            let avg = bucket.taxi_out_additional_min / bucket.taxi_out_flights as f64;
            Some(dec2(avg))
        } else {
            None
        };

        // Taxi-in: additional time per flight
        let taxi_in_additional = if bucket.taxi_in_flights > 0 {
            let avg = bucket.taxi_in_additional_min / bucket.taxi_in_flights as f64;
            Some(dec2(avg))
        } else {
            None
        };

        // Slot adherence: % of regulated flights within window
        let slot_total = bucket.slot_early + bucket.slot_on_time + bucket.slot_late;
        let slot_adherence = if slot_total > 0 {
            let pct = (bucket.slot_on_time as f64 / slot_total as f64) * 100.0;
            Some(dec2(pct))
        } else {
            None
        };

        // CDO/CCO percentages
        let cdo_pct = if bucket.total_flights_vfe_descent > 0 {
            let pct = (bucket.cdo_flights as f64 / bucket.total_flights_vfe_descent as f64) * 100.0;
            Some(dec2(pct))
        } else {
            None
        };

        let cco_pct = if bucket.total_flights_vfe_climb > 0 {
            let pct = (bucket.cco_flights as f64 / bucket.total_flights_vfe_climb as f64) * 100.0;
            Some(dec2(pct))
        } else {
            None
        };

        // CO2 waste per flight (descent + climb combined)
        let total_vfe_flights = bucket.total_flights_vfe_descent + bucket.total_flights_vfe_climb;
        let co2_waste = if total_vfe_flights > 0 {
            let total_co2 = bucket.delta_co2_kg_descent + bucket.delta_co2_kg_climb;
            Some(dec2(total_co2 / total_vfe_flights as f64))
        } else {
            None
        };

        let upsert_result = sqlx::query(
            r#"
            INSERT INTO operational_stats
                (airport_id, period_year, period_month, period_type,
                 total_flights, delay_pct, avg_delay_minutes,
                 delay_weather_pct, delay_carrier_pct, delay_atc_pct, delay_airport_pct,
                 asma_additional_min, taxi_out_additional_min, taxi_in_additional_min,
                 slot_adherence_pct, cdo_pct, cco_pct, co2_waste_kg_per_flight,
                 source)
            VALUES ($1, $2, $3, 'monthly', $4, $5, $6, $7, $8, $9, $10,
                    $11, $12, $13, $14, $15, $16, $17, 'eurocontrol')
            ON CONFLICT (airport_id, period_year, period_month, source)
            DO UPDATE SET
                total_flights           = EXCLUDED.total_flights,
                delay_pct               = COALESCE(EXCLUDED.delay_pct, operational_stats.delay_pct),
                avg_delay_minutes       = COALESCE(EXCLUDED.avg_delay_minutes, operational_stats.avg_delay_minutes),
                delay_weather_pct       = COALESCE(EXCLUDED.delay_weather_pct, operational_stats.delay_weather_pct),
                delay_carrier_pct       = COALESCE(EXCLUDED.delay_carrier_pct, operational_stats.delay_carrier_pct),
                delay_atc_pct           = COALESCE(EXCLUDED.delay_atc_pct, operational_stats.delay_atc_pct),
                delay_airport_pct       = COALESCE(EXCLUDED.delay_airport_pct, operational_stats.delay_airport_pct),
                asma_additional_min     = COALESCE(EXCLUDED.asma_additional_min, operational_stats.asma_additional_min),
                taxi_out_additional_min = COALESCE(EXCLUDED.taxi_out_additional_min, operational_stats.taxi_out_additional_min),
                taxi_in_additional_min  = COALESCE(EXCLUDED.taxi_in_additional_min, operational_stats.taxi_in_additional_min),
                slot_adherence_pct      = COALESCE(EXCLUDED.slot_adherence_pct, operational_stats.slot_adherence_pct),
                cdo_pct                 = COALESCE(EXCLUDED.cdo_pct, operational_stats.cdo_pct),
                cco_pct                 = COALESCE(EXCLUDED.cco_pct, operational_stats.cco_pct),
                co2_waste_kg_per_flight = COALESCE(EXCLUDED.co2_waste_kg_per_flight, operational_stats.co2_waste_kg_per_flight)
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
        .bind(asma_additional)
        .bind(taxi_out_additional)
        .bind(taxi_in_additional)
        .bind(slot_adherence)
        .bind(cdo_pct)
        .bind(cco_pct)
        .bind(co2_waste)
        .execute(pool)
        .await;

        match upsert_result {
            Ok(_) => {
                records_processed += 1;
            }
            Err(e) => {
                warn!(
                    year = *year,
                    month = *month,
                    error = %e,
                    "Failed to upsert operational_stats"
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

/// Helper: f64 → Decimal with 2 decimal places.
fn dec2(val: f64) -> Decimal {
    let mut d = Decimal::from_f64_retain(val).unwrap_or_default();
    d.rescale(2);
    d
}
