use chrono::{NaiveDate, DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[allow(unused_imports)]
mod entities;
#[allow(unused_imports)]
pub use entities::*;

// ── Core airport ──────────────────────────────────────────────

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Airport {
    pub id: i32,
    pub iata_code: Option<String>,
    pub icao_code: Option<String>,
    pub ourairports_id: Option<i32>,
    pub name: String,
    pub short_name: Option<String>,
    pub city: String,
    pub country_code: String,
    pub region_code: Option<String>,
    pub elevation_ft: Option<i32>,
    pub timezone: Option<String>,
    pub airport_type: String,
    pub scheduled_service: Option<bool>,
    pub terminal_count: Option<i16>,
    pub total_gates: Option<i16>,
    pub opened_year: Option<i16>,
    pub last_major_reno: Option<i16>,
    pub operator_id: Option<i32>,
    pub owner_id: Option<i32>,
    pub ownership_notes: Option<String>,
    pub annual_capacity_m: Option<rust_decimal::Decimal>,
    pub annual_pax_2019_m: Option<rust_decimal::Decimal>,
    pub annual_pax_latest_m: Option<rust_decimal::Decimal>,
    pub latest_pax_year: Option<i16>,
    pub wikipedia_url: Option<String>,
    pub website_url: Option<String>,
    pub skytrax_url: Option<String>,
    pub in_seed_set: Option<bool>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

// ── Fetch result (returned by every fetcher) ──────────────────

#[derive(Debug, Clone)]
pub struct FetchResult {
    pub records_processed: i32,
    pub last_record_date: Option<NaiveDate>,
}
