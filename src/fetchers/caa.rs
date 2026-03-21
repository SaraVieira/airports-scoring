use anyhow::Result;
use sqlx::PgPool;

use crate::models::{Airport, FetchResult};

/// Fetch UK Civil Aviation Authority passenger and performance data.
pub async fn fetch(_pool: &PgPool, _airport: &Airport, _full_refresh: bool) -> Result<FetchResult> {
    todo!("Implement CAA fetcher")
}
