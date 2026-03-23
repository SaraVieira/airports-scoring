use anyhow::{Context, Result};
use serde::Deserialize;
use sqlx::PgPool;
use tracing::info;

use crate::models::{Airport, FetchResult};

#[derive(Debug, Deserialize)]
struct AccreditationOutput {
    level_name: String,
    report_year: i32,
}

/// Map ACI Airport Carbon Accreditation level name to a numeric level (1–7).
fn level_name_to_int(name: &str) -> i16 {
    match name {
        "Mapping" => 1,
        "Reduction" => 2,
        "Optimisation" => 3,
        "Neutrality" => 4,
        "Transformation" => 5,
        "Transition" => 6,
        "Net Zero" => 7,
        _ => 0,
    }
}

/// Fetch ACI Airport Carbon Accreditation data by calling the Python lookup
/// script as a subprocess.  Data is read from `data/carbon_accreditation.json`
/// and upserted into the `carbon_accreditation` table.
pub async fn fetch(pool: &PgPool, airport: &Airport, _full_refresh: bool) -> Result<FetchResult> {
    let iata = airport
        .iata_code
        .as_deref()
        .context("Airport has no IATA code")?;

    let python = if std::path::Path::new(".venv/bin/python3").exists() {
        ".venv/bin/python3"
    } else {
        "python3"
    };

    info!(airport = iata, "Looking up carbon accreditation");

    let output = tokio::process::Command::new(python)
        .arg("python/carbon_accreditation.py")
        .arg("--airport")
        .arg(iata)
        .output()
        .await
        .context("Failed to run carbon_accreditation.py")?;

    // A non-zero exit means the airport was not found in the data file — not
    // an error worth failing the whole pipeline run.
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        info!(airport = iata, reason = %stderr.trim(), "Airport not in carbon accreditation data, skipping");
        return Ok(FetchResult {
            records_processed: 0,
            last_record_date: None,
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: AccreditationOutput = serde_json::from_str(stdout.trim())
        .context("Failed to parse carbon_accreditation.py JSON output")?;

    let level = level_name_to_int(&result.level_name);

    info!(
        airport = iata,
        level_name = %result.level_name,
        level = level,
        report_year = result.report_year,
        "Carbon accreditation data received"
    );

    sqlx::query(
        r#"
        INSERT INTO carbon_accreditation (airport_id, level, level_name, report_year)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (airport_id) DO UPDATE SET
            level       = EXCLUDED.level,
            level_name  = EXCLUDED.level_name,
            report_year = EXCLUDED.report_year
        "#,
    )
    .bind(airport.id)
    .bind(level)
    .bind(&result.level_name)
    .bind(result.report_year)
    .execute(pool)
    .await
    .context("Failed to upsert carbon_accreditation row")?;

    info!(airport = iata, "Carbon accreditation upserted");

    Ok(FetchResult {
        records_processed: 1,
        last_record_date: None,
    })
}
