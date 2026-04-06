use anyhow::Result;
use bzip2::read::BzDecoder;
use chrono::{Datelike, NaiveDate, Utc};
use rust_decimal::Decimal;
use sqlx::PgPool;
use std::io::Read;
use tracing::{info, warn};

use super::csv_parser::{find_col, find_col_exact, parse_f64_col};

/// Eurocontrol Performance data download base URL.
const BASE_URL: &str = "https://www.eurocontrol.int/performance/data/download/csv";

/// User-agent to avoid antibot blocking.
const UA: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36";

/// Remote datasets to sync.
const REMOTE_DATASETS: &[&str] = &[
    "airport_traffic",
    "asma_additional_time",
    "taxi_out_additional_time",
    "taxi_in_additional_time",
    "vertical_flight_efficiency",
    "atfm_slot_adherence",
];

/// Result of a sync operation.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SyncResult {
    pub datasets_synced: usize,
    pub total_rows: i64,
    pub errors: Vec<String>,
}

/// A single raw record parsed from any Eurocontrol CSV.
#[derive(Debug)]
struct RawRecord {
    dataset: String,
    apt_icao: String,
    flight_date: Option<NaiveDate>,
    year: i16,
    month: i16,
    total_flights: Option<i32>,
    ifr_flights: Option<i32>,
    additional_time_min: Option<Decimal>,
    reference_time_min: Option<Decimal>,
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
    slot_departures: Option<i32>,
    slot_early: Option<i32>,
    slot_on_time: Option<i32>,
    slot_late: Option<i32>,
}

/// Download all Eurocontrol datasets and ingest into eurocontrol_raw.
pub async fn run_sync(pool: &PgPool, full_refresh: bool) -> Result<SyncResult> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(180))
        .user_agent(UA)
        .build()?;

    let current_year = Utc::now().year() as i16;
    let remote_start = current_year - 2;

    let mut result = SyncResult {
        datasets_synced: 0,
        total_rows: 0,
        errors: vec![],
    };

    // ── Remote datasets ──────────────────────────────────────────
    for dataset in REMOTE_DATASETS {
        for year in remote_start..=current_year {
            // Skip if already synced today (unless full_refresh)
            if !full_refresh && already_synced_today(pool, dataset, year).await? {
                info!(dataset, year, "Already synced today, skipping");
                continue;
            }

            let url = format!("{}/{}_{}.csv", BASE_URL, dataset, year);
            info!(url = %url, "Downloading Eurocontrol CSV");

            let body = match download_csv(&client, &url).await {
                Ok(b) => b,
                Err(e) => {
                    let msg = format!("{dataset} {year}: {e}");
                    warn!(%msg);
                    result.errors.push(msg);
                    continue;
                }
            };

            let records = parse_csv(&body, dataset, false);
            let count = bulk_upsert(pool, &records).await?;

            // Log the sync
            sqlx::query(
                "INSERT INTO eurocontrol_sync_log (dataset, year, row_count) VALUES ($1, $2, $3)",
            )
            .bind(dataset)
            .bind(year)
            .bind(count as i32)
            .execute(pool)
            .await?;

            info!(dataset, year, rows = count, "Synced remote dataset");
            result.total_rows += count as i64;
            result.datasets_synced += 1;
        }
    }

    // ── apt_dly (remote bz2 download) ─────────────────────────────
    let apt_dly_start = if full_refresh { 2014_i16 } else { remote_start };
    for year in apt_dly_start..=current_year {
        if !full_refresh && already_synced_today(pool, "apt_dly", year).await? {
            info!(year, "apt_dly already synced today, skipping");
            continue;
        }

        let url = format!("{}/apt_dly_{}.csv.bz2", BASE_URL, year);
        info!(url = %url, "Downloading apt_dly bz2");

        let csv_data = match download_and_decompress_bz2(&client, &url).await {
            Ok(data) => data,
            Err(e) => {
                let msg = format!("apt_dly {year}: {e}");
                warn!(%msg);
                result.errors.push(msg);
                continue;
            }
        };

        let records = parse_csv(&csv_data, "apt_dly", true);
        let count = bulk_upsert(pool, &records).await?;

        sqlx::query(
            "INSERT INTO eurocontrol_sync_log (dataset, year, row_count) VALUES ($1, $2, $3)",
        )
        .bind("apt_dly")
        .bind(year)
        .bind(count as i32)
        .execute(pool)
        .await?;

        info!(year, rows = count, "Synced apt_dly");
        result.total_rows += count as i64;
        result.datasets_synced += 1;
    }

    info!(
        datasets = result.datasets_synced,
        rows = result.total_rows,
        errors = result.errors.len(),
        "Eurocontrol sync complete"
    );

    Ok(result)
}

async fn already_synced_today(pool: &PgPool, dataset: &str, year: i16) -> Result<bool> {
    let row: Option<(i64,)> = sqlx::query_as(
        "SELECT COUNT(*) FROM eurocontrol_sync_log \
         WHERE dataset = $1 AND year = $2 AND synced_at > CURRENT_DATE",
    )
    .bind(dataset)
    .bind(year)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| r.0 > 0).unwrap_or(false))
}

async fn download_csv(client: &reqwest::Client, url: &str) -> Result<String> {
    let resp = client.get(url).send().await?;

    if !resp.status().is_success() {
        anyhow::bail!("HTTP {}", resp.status());
    }

    let body = resp.text().await?;
    if body.starts_with("<!DOCTYPE") || body.starts_with("<html") {
        anyhow::bail!("Got HTML instead of CSV (antibot)");
    }

    Ok(body)
}

async fn download_and_decompress_bz2(client: &reqwest::Client, url: &str) -> Result<String> {
    let resp = client.get(url).send().await?;

    if !resp.status().is_success() {
        anyhow::bail!("HTTP {}", resp.status());
    }

    let bytes = resp.bytes().await?;

    tokio::task::spawn_blocking(move || -> Result<String> {
        let mut decoder = BzDecoder::new(&bytes[..]);
        let mut raw_bytes = Vec::new();
        decoder.read_to_end(&mut raw_bytes)?;
        Ok(String::from_utf8_lossy(&raw_bytes).into_owned())
    })
    .await?
}

fn parse_csv(csv_data: &str, dataset: &str, is_apt_dly: bool) -> Vec<RawRecord> {
    let mut rdr = csv::ReaderBuilder::new()
        .flexible(true)
        .has_headers(true)
        .trim(csv::Trim::All)
        .from_reader(csv_data.as_bytes());

    let headers = match rdr.headers() {
        Ok(h) => h.clone(),
        Err(_) => return vec![],
    };

    let icao_col = match find_col(&headers, &["apt_icao", "icao"]) {
        Some(c) => c,
        None => return vec![],
    };

    let year_col = find_col(&headers, &["year"]);
    let month_col = find_col(&headers, &["month_num"]);
    let date_col = find_col_exact(&headers, "flt_date");

    // Columns vary by dataset type
    let flight_col = find_col(&headers, &["flt_tot_1", "flt_tot_ifr_2", "tf"]);
    let ifr_flight_col = find_col_exact(&headers, "FLT_TOT_IFR_2");
    let _delay_col = find_col(&headers, &["total_add_time_min", "flt_dly_1"]);

    // ASMA / taxi specific
    let add_time_col = find_col_exact(&headers, "TOTAL_ADD_TIME_MIN");
    let ref_time_col = find_col_exact(&headers, "TOTAL_REF_TIME_MIN");
    let ref_flights_col = find_col_exact(&headers, "TOTAL_REF_NB_FL");

    // apt_dly
    let atfm_flights_col = find_col_exact(&headers, "flt_arr_1");
    let atfm_total_dly_col = find_col_exact(&headers, "dly_apt_arr_1");
    let delayed_flights_col = find_col_exact(&headers, "flt_arr_1_dly");
    let dly_weather_v_col = find_col_exact(&headers, "dly_apt_arr_v_1");
    let dly_weather_w_col = find_col_exact(&headers, "dly_apt_arr_w_1");
    let dly_atc_c_col = find_col_exact(&headers, "dly_apt_arr_c_1");
    let dly_atc_e_col = find_col_exact(&headers, "dly_apt_arr_e_1");
    let dly_atc_r_col = find_col_exact(&headers, "dly_apt_arr_r_1");
    let dly_atc_s_col = find_col_exact(&headers, "dly_apt_arr_s_1");
    let dly_atc_n_col = find_col_exact(&headers, "dly_apt_arr_n_1");
    let dly_carrier_a_col = find_col_exact(&headers, "dly_apt_arr_a_1");
    let dly_airport_g_col = find_col_exact(&headers, "dly_apt_arr_g_1");

    // VFE
    let nbr_flights_descent_col = find_col_exact(&headers, "NBR_FLIGHTS_DESCENT");
    let nbr_cdo_col = find_col_exact(&headers, "NBR_CDO_FLIGHTS");
    let nbr_flights_climb_col = find_col_exact(&headers, "NBR_FLIGHTS_CLIMB");
    let nbr_cco_col = find_col_exact(&headers, "NBR_CCO_FLIGHTS");
    let co2_descent_col = find_col_exact(&headers, "TOT_DELTA_CO2_KG_DESCENT");
    let co2_climb_col = find_col_exact(&headers, "TOT_DELTA_CO2_KG_CLIMB");

    // Slot adherence
    let slot_dep_col = find_col_exact(&headers, "FLT_DEP_1");
    let slot_early_col = find_col_exact(&headers, "FLT_DEP_OUT_EARLY_1");
    let slot_on_time_col = find_col_exact(&headers, "FLT_DEP_IN_1");
    let slot_late_col = find_col_exact(&headers, "FLT_DEP_OUT_LATE_1");

    let mut records = Vec::new();

    for result in rdr.records() {
        let record = match result {
            Ok(r) => r,
            Err(_) => continue,
        };

        let apt_icao = record.get(icao_col).unwrap_or("").trim().to_string();
        if apt_icao.is_empty() {
            continue;
        }

        let year: i16 = match year_col
            .and_then(|c| record.get(c))
            .and_then(|v| v.trim().parse().ok())
        {
            Some(y) => y,
            None => continue,
        };

        let month: i16 = match month_col
            .and_then(|c| record.get(c))
            .and_then(|v| v.trim().parse().ok())
        {
            Some(m) if (1..=12).contains(&m) => m,
            _ => continue,
        };

        let flight_date = date_col
            .and_then(|c| record.get(c))
            .and_then(|v| {
                let trimmed = v.trim();
                // Handle both "2025-01-01" and "2025-01-01T00:00:00Z"
                let date_part = trimmed.split('T').next().unwrap_or(trimmed);
                NaiveDate::parse_from_str(date_part, "%Y-%m-%d").ok()
            });

        let mut raw = RawRecord {
            dataset: dataset.to_string(),
            apt_icao,
            flight_date,
            year,
            month,
            total_flights: None,
            ifr_flights: None,
            additional_time_min: None,
            reference_time_min: None,
            reference_flights: None,
            arr_flights: None,
            delayed_flights: None,
            total_atfm_delay_min: None,
            dly_weather_min: None,
            dly_atc_min: None,
            dly_carrier_min: None,
            dly_airport_min: None,
            cdo_flights: None,
            cco_flights: None,
            total_flights_vfe: None,
            delta_co2_kg_descent: None,
            delta_co2_kg_climb: None,
            slot_departures: None,
            slot_early: None,
            slot_on_time: None,
            slot_late: None,
        };

        // Parse dataset-specific columns
        match dataset {
            "airport_traffic" => {
                raw.total_flights = flight_col
                    .and_then(|c| record.get(c))
                    .and_then(|v| v.trim().parse().ok());
                raw.ifr_flights = ifr_flight_col
                    .and_then(|c| record.get(c))
                    .and_then(|v| v.trim().parse().ok());
            }
            "asma_additional_time" | "taxi_out_additional_time" | "taxi_in_additional_time" => {
                raw.total_flights = flight_col
                    .and_then(|c| record.get(c))
                    .and_then(|v| v.trim().parse().ok());
                raw.additional_time_min = parse_decimal(&record, add_time_col);
                raw.reference_time_min = parse_decimal(&record, ref_time_col);
                raw.reference_flights = ref_flights_col
                    .and_then(|c| record.get(c))
                    .and_then(|v| v.trim().parse().ok());
            }
            "apt_dly" if is_apt_dly => {
                raw.arr_flights = atfm_flights_col
                    .and_then(|c| record.get(c))
                    .and_then(|v| v.trim().parse().ok());
                raw.delayed_flights = delayed_flights_col
                    .and_then(|c| record.get(c))
                    .and_then(|v| v.trim().parse().ok());
                raw.total_atfm_delay_min = parse_decimal(&record, atfm_total_dly_col);

                let weather = parse_f64_col(&record, dly_weather_v_col)
                    + parse_f64_col(&record, dly_weather_w_col);
                raw.dly_weather_min = dec(weather);

                let atc = parse_f64_col(&record, dly_atc_c_col)
                    + parse_f64_col(&record, dly_atc_e_col)
                    + parse_f64_col(&record, dly_atc_r_col)
                    + parse_f64_col(&record, dly_atc_s_col)
                    + parse_f64_col(&record, dly_atc_n_col);
                raw.dly_atc_min = dec(atc);

                raw.dly_carrier_min = dec(parse_f64_col(&record, dly_carrier_a_col));
                raw.dly_airport_min = dec(parse_f64_col(&record, dly_airport_g_col));
            }
            "vertical_flight_efficiency" => {
                raw.total_flights_vfe = nbr_flights_descent_col
                    .and_then(|c| record.get(c))
                    .and_then(|v| v.trim().parse().ok());
                raw.cdo_flights = nbr_cdo_col
                    .and_then(|c| record.get(c))
                    .and_then(|v| v.trim().parse().ok());
                raw.cco_flights = nbr_cco_col
                    .and_then(|c| record.get(c))
                    .and_then(|v| v.trim().parse().ok());
                raw.delta_co2_kg_descent = parse_decimal(&record, co2_descent_col);
                raw.delta_co2_kg_climb = parse_decimal(&record, co2_climb_col);

                // Also capture climb flight count in total_flights for reference
                raw.total_flights = nbr_flights_climb_col
                    .and_then(|c| record.get(c))
                    .and_then(|v| v.trim().parse().ok());
            }
            "atfm_slot_adherence" => {
                raw.slot_departures = slot_dep_col
                    .and_then(|c| record.get(c))
                    .and_then(|v| v.trim().parse().ok());
                raw.slot_early = slot_early_col
                    .and_then(|c| record.get(c))
                    .and_then(|v| v.trim().parse().ok());
                raw.slot_on_time = slot_on_time_col
                    .and_then(|c| record.get(c))
                    .and_then(|v| v.trim().parse().ok());
                raw.slot_late = slot_late_col
                    .and_then(|c| record.get(c))
                    .and_then(|v| v.trim().parse().ok());
            }
            _ => {
                // Generic: just grab flight count and delay
                raw.total_flights = flight_col
                    .and_then(|c| record.get(c))
                    .and_then(|v| v.trim().parse().ok());
            }
        }

        records.push(raw);
    }

    records
}

fn parse_decimal(record: &csv::StringRecord, col: Option<usize>) -> Option<Decimal> {
    col.and_then(|c| record.get(c))
        .and_then(|v| v.trim().parse::<Decimal>().ok())
}

fn dec(val: f64) -> Option<Decimal> {
    if val == 0.0 {
        None
    } else {
        use rust_decimal::prelude::FromPrimitive;
        Decimal::from_f64(val)
    }
}

/// Bulk upsert records into eurocontrol_raw. Returns number of rows affected.
async fn bulk_upsert(pool: &PgPool, records: &[RawRecord]) -> Result<usize> {
    if records.is_empty() {
        return Ok(0);
    }

    let mut count = 0usize;

    // Process in batches of 500 to avoid query size limits
    for chunk in records.chunks(500) {
        let mut query = String::from(
            "INSERT INTO eurocontrol_raw (\
                dataset, apt_icao, flight_date, year, month, \
                total_flights, ifr_flights, \
                additional_time_min, reference_time_min, reference_flights, \
                arr_flights, delayed_flights, total_atfm_delay_min, \
                dly_weather_min, dly_atc_min, dly_carrier_min, dly_airport_min, \
                cdo_flights, cco_flights, total_flights_vfe, \
                delta_co2_kg_descent, delta_co2_kg_climb, \
                slot_departures, slot_early, slot_on_time, slot_late\
            ) VALUES ",
        );

        let mut param_idx = 1u32;
        for (i, _) in chunk.iter().enumerate() {
            if i > 0 {
                query.push_str(", ");
            }
            query.push('(');
            for j in 0..26 {
                if j > 0 {
                    query.push_str(", ");
                }
                query.push('$');
                query.push_str(&param_idx.to_string());
                param_idx += 1;
            }
            query.push(')');
        }

        query.push_str(
            " ON CONFLICT (dataset, apt_icao, year, month, flight_date) \
              DO UPDATE SET \
                total_flights = COALESCE(EXCLUDED.total_flights, eurocontrol_raw.total_flights), \
                ifr_flights = COALESCE(EXCLUDED.ifr_flights, eurocontrol_raw.ifr_flights), \
                additional_time_min = COALESCE(EXCLUDED.additional_time_min, eurocontrol_raw.additional_time_min), \
                reference_time_min = COALESCE(EXCLUDED.reference_time_min, eurocontrol_raw.reference_time_min), \
                reference_flights = COALESCE(EXCLUDED.reference_flights, eurocontrol_raw.reference_flights), \
                arr_flights = COALESCE(EXCLUDED.arr_flights, eurocontrol_raw.arr_flights), \
                delayed_flights = COALESCE(EXCLUDED.delayed_flights, eurocontrol_raw.delayed_flights), \
                total_atfm_delay_min = COALESCE(EXCLUDED.total_atfm_delay_min, eurocontrol_raw.total_atfm_delay_min), \
                dly_weather_min = COALESCE(EXCLUDED.dly_weather_min, eurocontrol_raw.dly_weather_min), \
                dly_atc_min = COALESCE(EXCLUDED.dly_atc_min, eurocontrol_raw.dly_atc_min), \
                dly_carrier_min = COALESCE(EXCLUDED.dly_carrier_min, eurocontrol_raw.dly_carrier_min), \
                dly_airport_min = COALESCE(EXCLUDED.dly_airport_min, eurocontrol_raw.dly_airport_min), \
                cdo_flights = COALESCE(EXCLUDED.cdo_flights, eurocontrol_raw.cdo_flights), \
                cco_flights = COALESCE(EXCLUDED.cco_flights, eurocontrol_raw.cco_flights), \
                total_flights_vfe = COALESCE(EXCLUDED.total_flights_vfe, eurocontrol_raw.total_flights_vfe), \
                delta_co2_kg_descent = COALESCE(EXCLUDED.delta_co2_kg_descent, eurocontrol_raw.delta_co2_kg_descent), \
                delta_co2_kg_climb = COALESCE(EXCLUDED.delta_co2_kg_climb, eurocontrol_raw.delta_co2_kg_climb), \
                slot_departures = COALESCE(EXCLUDED.slot_departures, eurocontrol_raw.slot_departures), \
                slot_early = COALESCE(EXCLUDED.slot_early, eurocontrol_raw.slot_early), \
                slot_on_time = COALESCE(EXCLUDED.slot_on_time, eurocontrol_raw.slot_on_time), \
                slot_late = COALESCE(EXCLUDED.slot_late, eurocontrol_raw.slot_late), \
                ingested_at = NOW()",
        );

        let mut q = sqlx::query(&query);

        for rec in chunk {
            q = q
                .bind(&rec.dataset)
                .bind(&rec.apt_icao)
                .bind(rec.flight_date)
                .bind(rec.year)
                .bind(rec.month)
                .bind(rec.total_flights)
                .bind(rec.ifr_flights)
                .bind(rec.additional_time_min)
                .bind(rec.reference_time_min)
                .bind(rec.reference_flights)
                .bind(rec.arr_flights)
                .bind(rec.delayed_flights)
                .bind(rec.total_atfm_delay_min)
                .bind(rec.dly_weather_min)
                .bind(rec.dly_atc_min)
                .bind(rec.dly_carrier_min)
                .bind(rec.dly_airport_min)
                .bind(rec.cdo_flights)
                .bind(rec.cco_flights)
                .bind(rec.total_flights_vfe)
                .bind(rec.delta_co2_kg_descent)
                .bind(rec.delta_co2_kg_climb)
                .bind(rec.slot_departures)
                .bind(rec.slot_early)
                .bind(rec.slot_on_time)
                .bind(rec.slot_late);
        }

        let result = q.execute(pool).await?;
        count += result.rows_affected() as usize;
    }

    Ok(count)
}
