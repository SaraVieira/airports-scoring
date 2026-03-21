use anyhow::Result;
use sqlx::PgPool;

use crate::models::{Airport, FetchResult};

/// Fetch airport, runway, and frequency data from OurAirports CSV files.
pub async fn fetch(_pool: &PgPool, _airport: &Airport, _full_refresh: bool) -> Result<FetchResult> {
    todo!("Implement OurAirports fetcher")
}
