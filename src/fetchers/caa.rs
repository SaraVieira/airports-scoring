use anyhow::{Context, Result};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use sqlx::PgPool;
use tracing::{info, warn};

use crate::models::{Airport, FetchResult};

/// CAA is the UK Civil Aviation Authority — only runs for GB airports.
const CAA_COUNTRY: &str = "GB";

/// CAA data download URL pattern.
/// The CAA publishes CSV punctuality data on their website.
const CAA_BASE_URL: &str =
    "https://www.caa.co.uk/data-and-analysis/uk-aviation-market/flight-punctuality/uk-flight-punctuality-data";

/// Fetch UK Civil Aviation Authority passenger and performance data.
pub async fn fetch(pool: &PgPool, airport: &Airport, _full_refresh: bool) -> Result<FetchResult> {
    let iata = airport
        .iata_code
        .as_deref()
        .context("Airport has no IATA code")?;

    // Only process UK airports.
    if airport.country_code != CAA_COUNTRY {
        info!(airport = iata, "Not a UK airport, skipping CAA");
        return Ok(FetchResult {
            records_processed: 0,
            last_record_date: None,
        });
    }

    let icao = airport
        .icao_code
        .as_deref()
        .unwrap_or("????");

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()?;

    // Try to download the CSV data from CAA
    // The CAA website may serve CSV directly or require navigating to a download link.
    // We attempt a direct CSV download from known patterns.
    let csv_url = format!("{}/download", CAA_BASE_URL);

    info!(url = %csv_url, airport = iata, "Fetching CAA punctuality data");

    let resp = match client
        .get(&csv_url)
        .header("Accept", "text/csv, application/octet-stream, */*")
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            warn!(error = %e, "Failed to download CAA data");
            return Ok(FetchResult {
                records_processed: 0,
                last_record_date: None,
            });
        }
    };

    if !resp.status().is_success() {
        warn!(
            status = %resp.status(),
            "CAA returned non-success status"
        );
        return Ok(FetchResult {
            records_processed: 0,
            last_record_date: None,
        });
    }

    let body = resp.text().await?;

    let mut rdr = csv::ReaderBuilder::new()
        .flexible(true)
        .has_headers(true)
        .trim(csv::Trim::All)
        .from_reader(body.as_bytes());

    let headers = rdr.headers()?.clone();

    // Find relevant columns (CAA CSVs vary but typically include these)
    let col = |candidates: &[&str]| -> Option<usize> {
        for (i, h) in headers.iter().enumerate() {
            let lower = h.trim().to_lowercase();
            for &c in candidates {
                if lower.contains(c) {
                    return Some(i);
                }
            }
        }
        None
    };

    let airport_col = col(&["reporting_airport", "airport"]);
    let year_col = col(&["year", "report_year"]);
    let month_col = col(&["month", "report_month", "month_num"]);
    let total_col = col(&["total_flights", "number_flights", "flights"]);
    let delayed_col = col(&["delayed", "flights_delayed", "late"]);
    let delay_pct_col = col(&["pct_delayed", "delay_pct", "percent_late", "on_time"]);
    let cancelled_col = col(&["cancelled", "cancellations"]);
    let cancel_pct_col = col(&["cancel_pct", "pct_cancelled", "cancellation_rate"]);
    let avg_delay_col = col(&["avg_delay", "average_delay", "mean_delay"]);

    let airport_col = match airport_col {
        Some(c) => c,
        None => {
            warn!("No airport column found in CAA CSV");
            return Ok(FetchResult {
                records_processed: 0,
                last_record_date: None,
            });
        }
    };

    let mut records_processed: i32 = 0;
    let mut latest_date: Option<NaiveDate> = None;

    for result in rdr.records() {
        let record = match result {
            Ok(r) => r,
            Err(_) => continue,
        };

        // Check if this record matches our airport (by IATA or ICAO)
        let rec_airport = record.get(airport_col).unwrap_or("").trim();
        if !rec_airport.eq_ignore_ascii_case(iata) && !rec_airport.eq_ignore_ascii_case(icao) {
            continue;
        }

        let year: i16 = match year_col.and_then(|c| record.get(c)) {
            Some(v) => match v.trim().parse() {
                Ok(y) => y,
                Err(_) => continue,
            },
            None => continue,
        };

        let month: Option<i16> = month_col
            .and_then(|c| record.get(c))
            .and_then(|v| v.trim().parse().ok())
            .filter(|m: &i16| (1..=12).contains(m));

        let parse_i32 = |col: Option<usize>| -> Option<i32> {
            col.and_then(|c| record.get(c))
                .and_then(|v| v.trim().replace(',', "").parse().ok())
        };

        let parse_dec = |col: Option<usize>| -> Option<Decimal> {
            col.and_then(|c| record.get(c))
                .and_then(|v| v.trim().replace('%', "").parse().ok())
        };

        let total_flights = parse_i32(total_col);
        let delayed_flights = parse_i32(delayed_col);
        let delay_pct = parse_dec(delay_pct_col);
        let cancelled_flights = parse_i32(cancelled_col);
        let cancellation_pct = parse_dec(cancel_pct_col);
        let avg_delay_minutes = parse_dec(avg_delay_col);

        let period_type = if month.is_some() { "monthly" } else { "annual" };

        sqlx::query(
            r#"
            INSERT INTO operational_stats
                (airport_id, period_year, period_month, period_type,
                 total_flights, delayed_flights, delay_pct, avg_delay_minutes,
                 cancelled_flights, cancellation_pct, source)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 'caa')
            ON CONFLICT (airport_id, period_year, period_month, source)
            DO UPDATE SET
                total_flights     = EXCLUDED.total_flights,
                delayed_flights   = EXCLUDED.delayed_flights,
                delay_pct         = EXCLUDED.delay_pct,
                avg_delay_minutes = EXCLUDED.avg_delay_minutes,
                cancelled_flights = EXCLUDED.cancelled_flights,
                cancellation_pct  = EXCLUDED.cancellation_pct
            "#,
        )
        .bind(airport.id)
        .bind(year)
        .bind(month)
        .bind(period_type)
        .bind(total_flights)
        .bind(delayed_flights)
        .bind(delay_pct)
        .bind(avg_delay_minutes)
        .bind(cancelled_flights)
        .bind(cancellation_pct)
        .execute(pool)
        .await
        .with_context(|| {
            format!(
                "Failed to upsert CAA operational_stats for {}-{:?}",
                year, month
            )
        })?;

        records_processed += 1;

        let record_date = NaiveDate::from_ymd_opt(
            year as i32,
            month.unwrap_or(12) as u32,
            1,
        )
        .unwrap();
        latest_date = Some(match latest_date {
            Some(prev) if record_date > prev => record_date,
            Some(prev) => prev,
            None => record_date,
        });
    }

    info!(
        airport = iata,
        records = records_processed,
        "CAA fetch complete"
    );

    Ok(FetchResult {
        records_processed,
        last_record_date: latest_date,
    })
}
