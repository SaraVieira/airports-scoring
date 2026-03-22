use anyhow::{Context, Result};
use chrono::NaiveDate;
use serde_json::Value;
use sqlx::PgPool;
use std::collections::HashMap;
use tracing::{info, warn};

use crate::models::{Airport, FetchResult};

/// Eurostat REST API base for aviation passenger data.
const EUROSTAT_API: &str =
    "https://ec.europa.eu/eurostat/api/dissemination/statistics/1.0/data/avia_paoa";

/// Fetch passenger traffic statistics from Eurostat.
/// Pulls total pax, international pax, and domestic pax separately.
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
    let rep_airp = format!("{}_{}", country, icao);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()?;

    // Step 1: Fetch total passengers (existing behavior)
    let totals = fetch_eurostat_data(&client, &rep_airp, "TOTAL").await?;

    let mut records_processed: i32 = 0;
    let mut latest_date: Option<NaiveDate> = None;

    for (&year, &total_pax) in &totals {
        sqlx::query(
            r#"
            INSERT INTO pax_yearly (airport_id, year, total_pax, source)
            VALUES ($1, $2, $3, 'eurostat')
            ON CONFLICT (airport_id, year) DO UPDATE SET
                total_pax = COALESCE(EXCLUDED.total_pax, pax_yearly.total_pax)
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

    // Step 2: Fetch international passengers
    let international = fetch_eurostat_data(&client, &rep_airp, "INTL").await?;
    let mut int_updated = 0;
    for (&year, &int_pax) in &international {
        let result = sqlx::query(
            "UPDATE pax_yearly SET international_pax = $1 WHERE airport_id = $2 AND year = $3 AND international_pax IS NULL",
        )
        .bind(int_pax)
        .bind(airport.id)
        .bind(year)
        .execute(pool)
        .await;

        if let Ok(r) = result {
            if r.rows_affected() > 0 {
                int_updated += 1;
            }
        }
    }

    // Step 3: Fetch domestic (national) passengers
    let domestic = fetch_eurostat_data(&client, &rep_airp, "NAT").await?;
    let mut dom_updated = 0;
    for (&year, &dom_pax) in &domestic {
        let result = sqlx::query(
            "UPDATE pax_yearly SET domestic_pax = $1 WHERE airport_id = $2 AND year = $3 AND domestic_pax IS NULL",
        )
        .bind(dom_pax)
        .bind(airport.id)
        .bind(year)
        .execute(pool)
        .await;

        if let Ok(r) = result {
            if r.rows_affected() > 0 {
                dom_updated += 1;
            }
        }
    }

    info!(
        airport = iata,
        records = records_processed,
        international_years = int_updated,
        domestic_years = dom_updated,
        "Eurostat fetch complete"
    );

    Ok(FetchResult {
        records_processed,
        last_record_date: latest_date,
    })
}

/// Query a single Eurostat tra_cov dimension (TOTAL, INT, or NAT) and return year→pax map.
async fn fetch_eurostat_data(
    client: &reqwest::Client,
    rep_airp: &str,
    tra_cov: &str,
) -> Result<HashMap<i16, i64>> {
    let url = format!(
        "{}?format=JSON&lang=en&freq=A&unit=PAS&tra_meas=PAS_CRD&schedule=TOT&tra_cov={}&rep_airp={}&sinceTimePeriod=2000",
        EUROSTAT_API, tra_cov, rep_airp
    );

    info!(url = %url, "Querying Eurostat API");

    let resp = match client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => {
            warn!(error = %e, tra_cov = tra_cov, "Eurostat request failed");
            return Ok(HashMap::new());
        }
    };

    if !resp.status().is_success() {
        warn!(status = %resp.status(), tra_cov = tra_cov, "Eurostat non-success response");
        return Ok(HashMap::new());
    }

    let body: Value = match resp.json().await {
        Ok(v) => v,
        Err(e) => {
            warn!(error = %e, tra_cov = tra_cov, "Failed to parse Eurostat JSON");
            return Ok(HashMap::new());
        }
    };

    let time_index = body
        .pointer("/dimension/time/category/index")
        .and_then(|v| v.as_object());

    let values = body.get("value").and_then(|v| v.as_object());

    let (time_index, values) = match (time_index, values) {
        (Some(t), Some(v)) => (t, v),
        _ => return Ok(HashMap::new()),
    };

    let mut index_to_year: HashMap<String, i16> = HashMap::new();
    for (year_str, idx_val) in time_index {
        if let Ok(year) = year_str.parse::<i16>() {
            if let Some(idx) = idx_val.as_u64() {
                index_to_year.insert(idx.to_string(), year);
            }
        }
    }

    let mut result = HashMap::new();
    for (idx_str, pax_val) in values {
        if let (Some(&year), Some(pax)) = (index_to_year.get(idx_str), pax_val.as_f64()) {
            result.insert(year, pax as i64);
        }
    }

    Ok(result)
}
