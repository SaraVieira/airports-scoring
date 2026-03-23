use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::Value;
use sqlx::PgPool;
use tracing::info;

use crate::models::{Airport, FetchResult};

#[derive(Debug, Deserialize)]
struct ScrapedLounge {
    name: String,
    terminal: Option<String>,
    opening_hours: Option<String>,
    amenities: Vec<String>,
    url: Option<String>,
}

/// Fetch Priority Pass lounge data by calling the Python scraper as a subprocess.
/// Reads JSON array from stdout and inserts lounge records into the `lounges` table.
pub async fn fetch(pool: &PgPool, airport: &Airport, _full_refresh: bool) -> Result<FetchResult> {
    let iata = airport
        .iata_code
        .as_deref()
        .context("Airport has no IATA code")?;

    // Find python — prefer .venv
    let python = if std::path::Path::new(".venv/bin/python3").exists() {
        ".venv/bin/python3"
    } else {
        "python3"
    };

    info!(airport = iata, "Running Priority Pass lounge scraper");

    let output = tokio::process::Command::new(python)
        .arg("python/priority_pass_scraper.py")
        .arg("--airport")
        .arg(iata)
        .output()
        .await
        .context("Failed to run priority_pass_scraper.py")?;

    if !output.stderr.is_empty() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        for line in stderr.lines() {
            if line.contains("ERROR") || line.contains("Error") {
                tracing::warn!(airport = iata, line = %line, "Priority Pass scraper stderr");
            }
        }
    }

    if !output.status.success() {
        anyhow::bail!(
            "Priority Pass scraper exited with status {} for {}",
            output.status,
            iata
        );
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lounges: Vec<ScrapedLounge> = serde_json::from_str(stdout.trim())
        .context("Failed to parse Priority Pass scraper JSON output")?;

    info!(
        airport = iata,
        count = lounges.len(),
        "Priority Pass scraper returned lounge data"
    );

    // Delete old lounge records for this airport from priority_pass
    sqlx::query(
        "DELETE FROM lounges WHERE airport_id = $1 AND source = 'priority_pass'",
    )
    .bind(airport.id)
    .execute(pool)
    .await
    .context("Failed to delete old Priority Pass lounges")?;

    let mut records: i32 = 0;

    for lounge in &lounges {
        let amenities_json: Value = serde_json::to_value(&lounge.amenities)
            .unwrap_or(Value::Array(vec![]));

        let insert = sqlx::query(
            r#"
            INSERT INTO lounges (
                airport_id, lounge_name, terminal, source,
                opening_hours, amenities, source_url
            ) VALUES (
                $1, $2, $3, 'priority_pass',
                $4, $5, $6
            )
            "#,
        )
        .bind(airport.id)
        .bind(&lounge.name)
        .bind(&lounge.terminal)
        .bind(&lounge.opening_hours)
        .bind(&amenities_json)
        .bind(&lounge.url)
        .execute(pool)
        .await;

        match insert {
            Ok(_) => {
                records += 1;
                info!(airport = iata, lounge = %lounge.name, "Inserted Priority Pass lounge");
            }
            Err(e) => {
                tracing::warn!(
                    airport = iata,
                    lounge = %lounge.name,
                    error = %e,
                    "Failed to insert lounge, skipping"
                );
            }
        }
    }

    info!(airport = iata, records = records, "Priority Pass lounges inserted");

    Ok(FetchResult {
        records_processed: records,
        last_record_date: None,
    })
}
