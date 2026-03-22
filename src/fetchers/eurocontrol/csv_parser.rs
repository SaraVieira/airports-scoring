use chrono::NaiveDate;
use rust_decimal::Decimal;
use std::collections::HashMap;

use super::MonthBucket;

/// Process a CSV string (from remote download or local bz2) into the buckets.
pub(super) fn process_csv(
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

pub(super) fn find_col(headers: &csv::StringRecord, candidates: &[&str]) -> Option<usize> {
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

pub(super) fn find_col_exact(headers: &csv::StringRecord, name: &str) -> Option<usize> {
    for (i, h) in headers.iter().enumerate() {
        if h.trim().eq_ignore_ascii_case(name) {
            return Some(i);
        }
    }
    None
}

pub(super) fn parse_f64_col(record: &csv::StringRecord, col: Option<usize>) -> f64 {
    col.and_then(|c| record.get(c))
        .and_then(|v| v.trim().parse::<f64>().ok())
        .unwrap_or(0.0)
}

pub(super) fn compute_cause_pct(cause_min: f64, total_min: f64) -> Option<Decimal> {
    if total_min > 0.0 && cause_min > 0.0 {
        let pct = (cause_min / total_min) * 100.0;
        let mut d = Decimal::from_f64_retain(pct).unwrap_or_default();
        d.rescale(2);
        Some(d)
    } else {
        None
    }
}
