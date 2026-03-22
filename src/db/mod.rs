use anyhow::{Context, Result};
use chrono::NaiveDate;
use sqlx::postgres::{PgPool, PgPoolOptions};

use crate::models::Airport;

/// Column list for the airports table, shared across queries.
const AIRPORT_COLUMNS: &str = r#"
    id, iata_code, icao_code, ourairports_id,
    name, short_name, city, country_code, region_code,
    elevation_ft, timezone, airport_type, scheduled_service,
    terminal_count, total_gates, opened_year, last_major_reno,
    operator_id, owner_id, ownership_notes,
    annual_capacity_m, annual_pax_2019_m, annual_pax_latest_m, latest_pax_year,
    wikipedia_url, website_url, skytrax_url,
    in_seed_set, created_at, updated_at
"#;

/// Create a connection pool from the DATABASE_URL environment variable.
pub async fn get_pool() -> Result<PgPool> {
    let database_url =
        std::env::var("DATABASE_URL").context("DATABASE_URL environment variable not set")?;

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(std::time::Duration::from_secs(30))
        .idle_timeout(std::time::Duration::from_secs(600))
        .max_lifetime(std::time::Duration::from_secs(1800))
        .connect(&database_url)
        .await
        .context("Failed to connect to Postgres")?;

    Ok(pool)
}

/// Look up a single airport by its IATA code.
pub async fn get_airport_by_iata(pool: &PgPool, iata: &str) -> Result<Airport> {
    let query = format!("SELECT {} FROM airports WHERE iata_code = $1", AIRPORT_COLUMNS);
    let airport = sqlx::query_as::<_, Airport>(&query)
    .bind(iata)
    .fetch_one(pool)
    .await
    .with_context(|| format!("Airport with IATA code '{}' not found", iata))?;

    Ok(airport)
}

/// Return all airports that are part of the seed set.
pub async fn get_seed_airports(pool: &PgPool) -> Result<Vec<Airport>> {
    let query = format!(
        "SELECT {} FROM airports WHERE in_seed_set = TRUE ORDER BY iata_code",
        AIRPORT_COLUMNS
    );
    let airports = sqlx::query_as::<_, Airport>(&query)
    .fetch_all(pool)
    .await
    .context("Failed to fetch seed airports")?;

    Ok(airports)
}

/// Insert a new pipeline_run row with status = 'running'. Returns the run id.
pub async fn start_pipeline_run(pool: &PgPool, airport_id: i32, source: &str) -> Result<i32> {
    let row: (i32,) = sqlx::query_as(
        r#"
        INSERT INTO pipeline_runs (airport_id, source, status)
        VALUES ($1, $2, 'running')
        RETURNING id
        "#,
    )
    .bind(airport_id)
    .bind(source)
    .fetch_one(pool)
    .await
    .context("Failed to start pipeline run")?;

    Ok(row.0)
}

/// Mark a pipeline run as complete (success or failed).
pub async fn complete_pipeline_run(
    pool: &PgPool,
    run_id: i32,
    status: &str,
    records: i32,
    last_date: Option<NaiveDate>,
    error: Option<&str>,
) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE pipeline_runs
        SET completed_at      = NOW(),
            status            = $1,
            records_processed = $2,
            last_record_date  = $3,
            error_message     = $4
        WHERE id = $5
        "#,
    )
    .bind(status)
    .bind(records)
    .bind(last_date)
    .bind(error)
    .bind(run_id)
    .execute(pool)
    .await
    .context("Failed to complete pipeline run")?;

    Ok(())
}
