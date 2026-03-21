use anyhow::{Context, Result};
use chrono::{NaiveDate, NaiveDateTime, Utc};
use rust_decimal::Decimal;
use sqlx::PgPool;
use std::collections::HashMap;
use tracing::info;

use crate::models::{Airport, FetchResult};

/// Base URL for Iowa Environmental Mesonet ASOS data.
const IEM_BASE: &str = "https://mesonet.agron.iastate.edu/cgi-bin/request/asos.py";

/// Raw METAR observation parsed from IEM CSV.
#[derive(Debug)]
struct MetarObs {
    date: NaiveDate,
    temp_f: Option<f64>,
    dewpoint_f: Option<f64>,
    visibility_miles: Option<f64>,
    wind_speed_kt: Option<f64>,
    wind_gust_kt: Option<f64>,
    precip_in: Option<f64>,
    sky_level1_ft: Option<f64>,
    weather_codes: String,
}

/// Aggregated daily METAR summary.
#[derive(Debug, Default)]
struct DailySummary {
    temps_c: Vec<f64>,
    visibilities_m: Vec<f64>,
    wind_speeds_kt: Vec<f64>,
    max_gust_kt: Option<f64>,
    precipitation_flag: bool,
    fog_flag: bool,
    low_ceiling_flag: bool,
    thunderstorm_flag: bool,
    count: i32,
}

/// Fetch and aggregate METAR weather observations into daily summaries.
pub async fn fetch(pool: &PgPool, airport: &Airport, full_refresh: bool) -> Result<FetchResult> {
    let icao = airport
        .icao_code
        .as_deref()
        .context("Airport has no ICAO code")?;

    // Determine date range
    let end_date = Utc::now().naive_utc().date();
    let start_date = if full_refresh {
        // Last 2 years
        end_date - chrono::Duration::days(730)
    } else {
        // Check last pipeline run for this source
        let last: Option<(NaiveDate,)> = sqlx::query_as(
            r#"
            SELECT last_record_date
            FROM pipeline_runs
            WHERE airport_id = $1 AND source = 'metar' AND status = 'success'
            ORDER BY completed_at DESC
            LIMIT 1
            "#,
        )
        .bind(airport.id)
        .fetch_optional(pool)
        .await?;

        match last {
            Some((d,)) => d,
            None => end_date - chrono::Duration::days(30),
        }
    };

    info!(
        airport = icao,
        start = %start_date,
        end = %end_date,
        "Fetching METAR data from IEM"
    );

    let url = format!(
        "{}?station={}&data=all&sts={}&ets={}&format=comma&latlon=no&elev=no&missing=empty&trace=0.0001&report_type=3",
        IEM_BASE,
        icao,
        start_date.format("%Y-%m-%dT00:00:00Z"),
        end_date.format("%Y-%m-%dT00:00:00Z"),
    );

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()?;

    let resp = client.get(&url).send().await?;
    if !resp.status().is_success() {
        anyhow::bail!(
            "IEM returned HTTP {} for {}",
            resp.status(),
            icao
        );
    }

    let body = resp.text().await?;

    // Parse CSV - IEM outputs with a header row (may have comment lines starting with #)
    let filtered: String = body
        .lines()
        .filter(|l| !l.starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");

    let mut rdr = csv::ReaderBuilder::new()
        .flexible(true)
        .has_headers(true)
        .trim(csv::Trim::All)
        .from_reader(filtered.as_bytes());

    let headers = rdr.headers()?.clone();

    // Map column names to indices
    let col = |name: &str| -> Option<usize> {
        headers.iter().position(|h| h.trim().eq_ignore_ascii_case(name))
    };

    let valid_col = col("valid");
    let tmpf_col = col("tmpf");
    let dwpf_col = col("dwpf");
    let vsby_col = col("vsby");
    let sknt_col = col("sknt");
    let gust_col = col("gust");
    let p01i_col = col("p01i");
    let _skyc1_col = col("skyc1");
    let skyl1_col = col("skyl1");
    let wxcodes_col = col("wxcodes").or_else(|| col("metar"));

    // Parse observations
    let mut daily: HashMap<NaiveDate, DailySummary> = HashMap::new();

    for result in rdr.records() {
        let record = match result {
            Ok(r) => r,
            Err(_) => continue,
        };

        // Parse timestamp
        let valid_str = match valid_col.and_then(|c| record.get(c)) {
            Some(v) if !v.trim().is_empty() => v.trim().to_string(),
            _ => continue,
        };

        let dt = match NaiveDateTime::parse_from_str(&valid_str, "%Y-%m-%d %H:%M") {
            Ok(d) => d,
            Err(_) => match NaiveDateTime::parse_from_str(&valid_str, "%Y-%m-%d %H:%M:%S") {
                Ok(d) => d,
                Err(_) => continue,
            },
        };

        let date = dt.date();

        let parse_f64 = |col: Option<usize>| -> Option<f64> {
            col.and_then(|c| record.get(c))
                .and_then(|v| {
                    let v = v.trim();
                    if v.is_empty() || v == "M" {
                        None
                    } else {
                        v.parse().ok()
                    }
                })
        };

        let obs = MetarObs {
            date,
            temp_f: parse_f64(tmpf_col),
            dewpoint_f: parse_f64(dwpf_col),
            visibility_miles: parse_f64(vsby_col),
            wind_speed_kt: parse_f64(sknt_col),
            wind_gust_kt: parse_f64(gust_col),
            precip_in: parse_f64(p01i_col),
            sky_level1_ft: parse_f64(skyl1_col),
            weather_codes: wxcodes_col
                .and_then(|c| record.get(c))
                .unwrap_or("")
                .to_string(),
        };

        // Aggregate into daily summary
        let summary = daily.entry(obs.date).or_default();
        summary.count += 1;

        // Convert temp F -> C
        if let Some(f) = obs.temp_f {
            let c = (f - 32.0) * 5.0 / 9.0;
            summary.temps_c.push(c);
        }

        // Convert visibility miles -> meters (1 mile = 1609.34 m)
        if let Some(v) = obs.visibility_miles {
            summary.visibilities_m.push(v * 1609.34);
            if v < 1.0 {
                summary.fog_flag = true;
            }
        }

        // Wind speed (already in knots)
        if let Some(w) = obs.wind_speed_kt {
            summary.wind_speeds_kt.push(w);
        }

        // Gust
        if let Some(g) = obs.wind_gust_kt {
            summary.max_gust_kt = Some(
                summary.max_gust_kt.map_or(g, |prev: f64| prev.max(g)),
            );
        }

        // Precipitation flag
        if let Some(p) = obs.precip_in {
            if p > 0.0 {
                summary.precipitation_flag = true;
            }
        }

        // Low ceiling flag (sky level < 1000 ft)
        if let Some(sl) = obs.sky_level1_ft {
            if sl < 1000.0 {
                summary.low_ceiling_flag = true;
            }
        }

        // Thunderstorm flag from weather codes
        let wx = obs.weather_codes.to_uppercase();
        if wx.contains("TS") {
            summary.thunderstorm_flag = true;
        }
    }

    // Upsert daily summaries
    let mut records_processed: i32 = 0;
    let mut latest_date: Option<NaiveDate> = None;

    for (date, summary) in &daily {
        let avg_temp = avg_decimal(&summary.temps_c);
        let min_temp = min_decimal(&summary.temps_c);
        let max_temp = max_decimal(&summary.temps_c);
        let avg_vis = avg_decimal(&summary.visibilities_m);
        let min_vis = min_decimal(&summary.visibilities_m);
        let avg_wind = avg_decimal(&summary.wind_speeds_kt);
        let max_wind = max_decimal(&summary.wind_speeds_kt);
        let max_gust = summary
            .max_gust_kt
            .and_then(Decimal::from_f64_retain);

        sqlx::query(
            r#"
            INSERT INTO metar_daily
                (airport_id, observation_date, avg_temp_c, min_temp_c, max_temp_c,
                 avg_visibility_m, min_visibility_m, avg_wind_speed_kt, max_wind_speed_kt,
                 max_wind_gust_kt, precipitation_flag, thunderstorm_flag, fog_flag,
                 low_ceiling_flag, metar_count)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            ON CONFLICT (airport_id, observation_date) DO UPDATE SET
                avg_temp_c        = EXCLUDED.avg_temp_c,
                min_temp_c        = EXCLUDED.min_temp_c,
                max_temp_c        = EXCLUDED.max_temp_c,
                avg_visibility_m  = EXCLUDED.avg_visibility_m,
                min_visibility_m  = EXCLUDED.min_visibility_m,
                avg_wind_speed_kt = EXCLUDED.avg_wind_speed_kt,
                max_wind_speed_kt = EXCLUDED.max_wind_speed_kt,
                max_wind_gust_kt  = EXCLUDED.max_wind_gust_kt,
                precipitation_flag = EXCLUDED.precipitation_flag,
                thunderstorm_flag  = EXCLUDED.thunderstorm_flag,
                fog_flag           = EXCLUDED.fog_flag,
                low_ceiling_flag   = EXCLUDED.low_ceiling_flag,
                metar_count        = EXCLUDED.metar_count
            "#,
        )
        .bind(airport.id)
        .bind(date)
        .bind(avg_temp)
        .bind(min_temp)
        .bind(max_temp)
        .bind(avg_vis)
        .bind(min_vis)
        .bind(avg_wind)
        .bind(max_wind)
        .bind(max_gust)
        .bind(summary.precipitation_flag)
        .bind(summary.thunderstorm_flag)
        .bind(summary.fog_flag)
        .bind(summary.low_ceiling_flag)
        .bind(summary.count)
        .execute(pool)
        .await
        .with_context(|| format!("Failed to upsert metar_daily for {}", date))?;

        records_processed += 1;
        latest_date = Some(match latest_date {
            Some(prev) if *date > prev => *date,
            Some(prev) => prev,
            None => *date,
        });
    }

    info!(
        airport = icao,
        records = records_processed,
        "METAR fetch complete"
    );

    Ok(FetchResult {
        records_processed,
        last_record_date: latest_date,
    })
}

fn avg_decimal(vals: &[f64]) -> Option<Decimal> {
    if vals.is_empty() {
        return None;
    }
    let sum: f64 = vals.iter().sum();
    Decimal::from_f64_retain(sum / vals.len() as f64)
}

fn min_decimal(vals: &[f64]) -> Option<Decimal> {
    vals.iter()
        .cloned()
        .reduce(f64::min)
        .and_then(Decimal::from_f64_retain)
}

fn max_decimal(vals: &[f64]) -> Option<Decimal> {
    vals.iter()
        .cloned()
        .reduce(f64::max)
        .and_then(Decimal::from_f64_retain)
}
