use anyhow::Result;
use sqlx::PgPool;

use crate::models::{Airport, FetchResult};

/// Fetch passenger traffic statistics from Eurostat.
pub async fn fetch(_pool: &PgPool, _airport: &Airport, _full_refresh: bool) -> Result<FetchResult> {
    todo!("Implement Eurostat fetcher")
}
