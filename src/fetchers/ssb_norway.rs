//! Statistics Norway (SSB) passenger data fetcher.
//!
//! Downloads monthly passenger counts per airport from the SSB Statbank API.
//! Table 08507: Air transport — passengers by airport, traffic type, etc.
//! Free JSON API, no authentication needed, data from 2009 onwards.

use anyhow::{Context, Result};
use chrono::{Datelike, NaiveDate, Utc};
use serde::Deserialize;
use sqlx::PgPool;
use std::collections::HashMap;
use tracing::{info, warn};

use crate::models::{Airport, FetchResult};

const SSB_API_URL: &str = "https://data.ssb.no/api/v0/en/table/08507";

/// SSB json-stat2 response (simplified).
#[derive(Debug, Deserialize)]
struct SsbResponse {
    dimension: SsbDimensions,
    value: Vec<Option<i64>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct SsbDimensions {
    tid: SsbDimension,
}

#[derive(Debug, Deserialize)]
struct SsbDimension {
    category: SsbCategory,
}

#[derive(Debug, Deserialize)]
struct SsbCategory {
    label: HashMap<String, String>,
}

/// Fetch passenger data from Statistics Norway for a Norwegian airport.
/// Only runs for airports with country_code "NO".
pub async fn fetch(pool: &PgPool, airport: &Airport, _full_refresh: bool) -> Result<FetchResult> {
    let iata = airport
        .iata_code
        .as_deref()
        .context("Airport has no IATA code")?;

    if airport.country_code != "NO" {
        return Ok(FetchResult {
            records_processed: 0,
            last_record_date: None,
        });
    }

    let icao = match airport.icao_code.as_deref() {
        Some(code) if code.starts_with("EN") => code,
        _ => {
            info!(airport = iata, "No Norwegian ICAO code, skipping SSB fetch");
            return Ok(FetchResult {
                records_processed: 0,
                last_record_date: None,
            });
        }
    };

    let current_year = Utc::now().year();
    // Fetch last 5 years of monthly data
    let start_year = current_year - 4;
    let mut time_values: Vec<String> = Vec::new();
    for year in start_year..=current_year {
        for month in 1..=12 {
            time_values.push(format!("{year}M{month:02}"));
        }
    }

    let query = serde_json::json!({
        "query": [
            {
                "code": "Lufthavn",
                "selection": { "filter": "item", "values": [icao] }
            },
            {
                "code": "TrafikkType",
                "selection": { "filter": "item", "values": ["000"] }
            },
            {
                "code": "TrafikkFly",
                "selection": { "filter": "item", "values": ["IU", "I", "U"] }
            },
            {
                "code": "PassasjerType",
                "selection": { "filter": "item", "values": ["AAT"] }
            },
            {
                "code": "ContentsCode",
                "selection": { "filter": "item", "values": ["Passasjerer"] }
            },
            {
                "code": "Tid",
                "selection": { "filter": "item", "values": time_values }
            }
        ],
        "response": { "format": "json-stat2" }
    });

    info!(airport = iata, icao, "Fetching SSB Norway passenger data");

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let resp = client
        .post(SSB_API_URL)
        .json(&query)
        .send()
        .await
        .context("Failed to call SSB API")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        warn!(airport = iata, status = %status, body = %body, "SSB API error");
        anyhow::bail!("SSB API returned {status} for {iata}");
    }

    let data: SsbResponse = resp
        .json()
        .await
        .context("Failed to parse SSB response")?;

    // The response has dimensions: TrafikkFly (IU, I, U) × Tid (months)
    // Values are laid out as: for each TrafikkFly, for each Tid
    let time_labels: Vec<String> = data
        .dimension
        .tid
        .category
        .label
        .values()
        .cloned()
        .collect();
    let time_keys: Vec<String> = data
        .dimension
        .tid
        .category
        .label
        .keys()
        .cloned()
        .collect();

    let n_times = time_labels.len();
    // 3 traffic types: IU (total), I (domestic), U (international)
    // Values layout: [IU_t1, IU_t2, ..., I_t1, I_t2, ..., U_t1, U_t2, ...]

    // Aggregate monthly data into annual totals
    struct YearData {
        total: i64,
        domestic: i64,
        international: i64,
    }
    let mut years: HashMap<i16, YearData> = HashMap::new();

    for (i, time_key) in time_keys.iter().enumerate() {
        // Parse year from "2024M01"
        let year: i16 = match time_key.get(0..4).and_then(|s| s.parse().ok()) {
            Some(y) => y,
            None => continue,
        };

        let entry = years.entry(year).or_insert(YearData {
            total: 0,
            domestic: 0,
            international: 0,
        });

        // Total (IU) is at index i
        if let Some(Some(v)) = data.value.get(i) {
            entry.total += v;
        }
        // Domestic (I) is at index n_times + i
        if let Some(Some(v)) = data.value.get(n_times + i) {
            entry.domestic += v;
        }
        // International (U) is at index 2*n_times + i
        if let Some(Some(v)) = data.value.get(2 * n_times + i) {
            entry.international += v;
        }
    }

    let mut records = 0i32;
    let mut latest_date: Option<NaiveDate> = None;

    for (year, data) in &years {
        // Skip years with no data
        if data.total == 0 {
            continue;
        }

        sqlx::query(
            r#"
            INSERT INTO pax_yearly (airport_id, year, total_pax, domestic_pax, international_pax, source)
            VALUES ($1, $2, $3, $4, $5, 'ssb_norway')
            ON CONFLICT (airport_id, year) DO UPDATE SET
                total_pax = GREATEST(EXCLUDED.total_pax, pax_yearly.total_pax),
                domestic_pax = COALESCE(EXCLUDED.domestic_pax, pax_yearly.domestic_pax),
                international_pax = COALESCE(EXCLUDED.international_pax, pax_yearly.international_pax),
                source = 'ssb_norway'
            "#,
        )
        .bind(airport.id)
        .bind(*year)
        .bind(data.total)
        .bind(if data.domestic > 0 {
            Some(data.domestic)
        } else {
            None
        })
        .bind(if data.international > 0 {
            Some(data.international)
        } else {
            None
        })
        .execute(pool)
        .await
        .with_context(|| format!("Failed to upsert SSB pax for {iata} year {year}"))?;

        records += 1;
        let d = NaiveDate::from_ymd_opt(*year as i32, 12, 31).unwrap();
        latest_date = Some(match latest_date {
            Some(prev) if d > prev => d,
            Some(prev) => prev,
            None => d,
        });
    }

    info!(airport = iata, records, "SSB Norway pax data upserted");

    Ok(FetchResult {
        records_processed: records,
        last_record_date: latest_date,
    })
}
