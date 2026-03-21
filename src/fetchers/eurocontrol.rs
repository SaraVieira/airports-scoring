use anyhow::Result;
use sqlx::PgPool;

use crate::models::{Airport, FetchResult};

/// Fetch delay and operational statistics from Eurocontrol.
pub async fn fetch(_pool: &PgPool, _airport: &Airport, _full_refresh: bool) -> Result<FetchResult> {
    todo!("Implement Eurocontrol fetcher")
}
