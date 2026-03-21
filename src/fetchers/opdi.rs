use anyhow::{Context, Result};
use chrono::{Datelike, NaiveDate};
use serde::Deserialize;
use sqlx::PgPool;
use std::collections::HashMap;
use tracing::{info, warn};

use crate::models::{Airport, FetchResult};

/// OPDI flight record as output by the Python helper.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct OpdiRecord {
    origin_icao: Option<String>,
    destination_icao: Option<String>,
    airline: Option<String>,
    flight_date: Option<String>,
}

/// Aggregation key for route counts.
#[derive(Debug, Hash, Eq, PartialEq, Clone)]
struct RouteKey {
    destination_icao: String,
    airline_icao: Option<String>,
}

/// Fetch route data from the Open Performance Data Initiative (OPDI).
///
/// OPDI publishes monthly Parquet files. We shell out to Python to convert
/// the relevant rows to JSON, then parse and upsert into the routes table.
pub async fn fetch(pool: &PgPool, airport: &Airport, full_refresh: bool) -> Result<FetchResult> {
    let icao = airport
        .icao_code
        .as_deref()
        .context("Airport has no ICAO code")?;

    // Determine which months to fetch
    let now = chrono::Utc::now().naive_utc().date();
    // OPDI parquet files are ~35MB each. Default to last 3 months
    // to avoid downloading 50+ files. Use --full-refresh for all history.
    let start_date = if full_refresh {
        NaiveDate::from_ymd_opt(2022, 1, 1).unwrap()
    } else {
        let last: Option<(Option<NaiveDate>,)> = sqlx::query_as(
            r#"
            SELECT last_record_date
            FROM pipeline_runs
            WHERE airport_id = $1 AND source = 'opdi' AND status = 'success'
            ORDER BY completed_at DESC
            LIMIT 1
            "#,
        )
        .bind(airport.id)
        .fetch_optional(pool)
        .await?;

        match last {
            Some((Some(d),)) => d,
            // Default to 3 months ago (not 2022) to avoid huge downloads
            _ => now - chrono::Months::new(3),
        }
    };

    // Generate monthly file URLs from start_date to now
    let mut route_counts: HashMap<RouteKey, (i64, NaiveDate, NaiveDate)> = HashMap::new();
    let mut latest_date: Option<NaiveDate> = None;

    let mut year = start_date.year() as i32;
    let mut month = start_date.month() as u32;

    loop {
        if year > now.year() as i32 || (year == now.year() as i32 && month > now.month() as u32) {
            break;
        }

        let url = format!(
            "https://www.eurocontrol.int/performance/data/download/OPDI/v002/flight_list/flight_list_{:04}{:02}.parquet",
            year, month
        );

        info!(url = %url, airport = icao, "Fetching OPDI parquet via Python helper");

        // Shell out to Python to download and filter the parquet file
        // Pure pyarrow — no pandas dependency needed
        let python_script = format!(
            r#"
import sys, json, urllib.request, tempfile, os
try:
    import pyarrow.parquet as pq
    import pyarrow.compute as pc
except ImportError:
    print("[]")
    sys.exit(0)

url = "{url}"
icao = "{icao}"
try:
    tmp = tempfile.NamedTemporaryFile(suffix=".parquet", delete=False)
    urllib.request.urlretrieve(url, tmp.name)
    table = pq.read_table(tmp.name)
    os.unlink(tmp.name)

    cols = {{c.lower(): c for c in table.column_names}}
    origin_col = cols.get("adep", cols.get("origin_icao", cols.get("departure_icao")))
    dest_col = cols.get("ades", cols.get("destination_icao", cols.get("arrival_icao")))
    airline_col = cols.get("airline", cols.get("airline_icao", cols.get("operator")))
    date_col = cols.get("filing_date", cols.get("flight_date", cols.get("date")))

    if origin_col is None or dest_col is None:
        print("[]")
        sys.exit(0)

    mask = pc.or_(
        pc.equal(table.column(origin_col), icao),
        pc.equal(table.column(dest_col), icao)
    )
    filtered = table.filter(mask)

    results = []
    for i in range(filtered.num_rows):
        rec = {{
            "origin_icao": str(filtered.column(origin_col)[i].as_py()) if origin_col else None,
            "destination_icao": str(filtered.column(dest_col)[i].as_py()) if dest_col else None,
            "airline": str(filtered.column(airline_col)[i].as_py()) if airline_col else None,
            "flight_date": str(filtered.column(date_col)[i].as_py()) if date_col else None,
        }}
        results.append(rec)

    print(json.dumps(results))
except Exception as e:
    print("[]", file=sys.stdout)
    print(f"Error: {{e}}", file=sys.stderr)
"#,
        );

        // Use venv python if available, otherwise system python3
        let python = if std::path::Path::new(".venv/bin/python3").exists() {
            ".venv/bin/python3"
        } else {
            "python3"
        };

        let output = tokio::process::Command::new(python)
            .arg("-c")
            .arg(&python_script)
            .output()
            .await;

        match output {
            Ok(out) => {
                if !out.stderr.is_empty() {
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    warn!(month = format!("{:04}-{:02}", year, month), stderr = %stderr, "Python stderr");
                }

                let stdout = String::from_utf8_lossy(&out.stdout);
                let records: Vec<OpdiRecord> = match serde_json::from_str(stdout.trim()) {
                    Ok(r) => r,
                    Err(e) => {
                        warn!(error = %e, "Failed to parse OPDI Python output");
                        Vec::new()
                    }
                };

                for rec in &records {
                    let dest = match &rec.destination_icao {
                        Some(d) if !d.is_empty() && d != "None" => d.clone(),
                        _ => continue,
                    };

                    let airline = rec.airline.as_ref().and_then(|a| {
                        let a = a.trim();
                        if a.is_empty() || a == "None" {
                            None
                        } else {
                            Some(a.to_string())
                        }
                    });

                    let flight_date = rec
                        .flight_date
                        .as_ref()
                        .and_then(|d| NaiveDate::parse_from_str(d.trim(), "%Y-%m-%d").ok())
                        .unwrap_or_else(|| {
                            NaiveDate::from_ymd_opt(year, month, 1).unwrap()
                        });

                    let key = RouteKey {
                        destination_icao: dest,
                        airline_icao: airline,
                    };

                    let entry = route_counts
                        .entry(key)
                        .or_insert((0, flight_date, flight_date));
                    entry.0 += 1;
                    if flight_date < entry.1 {
                        entry.1 = flight_date;
                    }
                    if flight_date > entry.2 {
                        entry.2 = flight_date;
                    }

                    latest_date = Some(match latest_date {
                        Some(prev) if flight_date > prev => flight_date,
                        Some(prev) => prev,
                        None => flight_date,
                    });
                }
            }
            Err(e) => {
                warn!(error = %e, "Failed to run Python helper for OPDI");
            }
        }

        // Advance to next month
        month += 1;
        if month > 12 {
            month = 1;
            year += 1;
        }
    }

    // Upsert routes
    let mut records_processed: i32 = 0;

    for (key, (count, first_seen, last_seen)) in &route_counts {
        sqlx::query(
            r#"
            INSERT INTO routes
                (origin_id, destination_icao, airline_icao,
                 flights_per_month, first_observed, last_observed, data_source)
            VALUES ($1, $2, $3, $4, $5, $6, 'opdi')
            ON CONFLICT (origin_id, destination_icao, airline_icao, data_source)
                WHERE data_source IN ('opdi', 'opensky')
            DO UPDATE SET
                flights_per_month = EXCLUDED.flights_per_month,
                first_observed    = LEAST(routes.first_observed, EXCLUDED.first_observed),
                last_observed     = GREATEST(routes.last_observed, EXCLUDED.last_observed),
                updated_at        = NOW()
            "#,
        )
        .bind(airport.id)
        .bind(&key.destination_icao)
        .bind(&key.airline_icao)
        .bind(*count as i32)
        .bind(first_seen)
        .bind(last_seen)
        .execute(pool)
        .await
        .with_context(|| format!("Failed to upsert OPDI route to {}", key.destination_icao))?;

        records_processed += 1;
    }

    info!(
        airport = icao,
        routes = records_processed,
        "OPDI fetch complete"
    );

    Ok(FetchResult {
        records_processed,
        last_record_date: latest_date,
    })
}
