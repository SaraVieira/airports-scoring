use anyhow::Result;
use sqlx::PgPool;

use crate::models::{Airport, FetchResult};

/// Fetch route and flight data from the OpenSky Network API.
pub async fn fetch(_pool: &PgPool, _airport: &Airport, _full_refresh: bool) -> Result<FetchResult> {
    todo!("Implement OpenSky fetcher")
}
