use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct SupportedAirport {
    pub iata_code: String,
    pub country_code: String,
    pub name: String,
    pub skytrax_review_slug: Option<String>,
    pub skytrax_rating_slug: Option<String>,
    pub google_maps_url: Option<String>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct SourceStatus {
    pub iata_code: String,
    pub source: String,
    pub last_fetched_at: Option<DateTime<Utc>>,
    pub last_status: String,
    pub last_record_count: Option<i32>,
    pub last_error: Option<String>,
    pub updated_at: DateTime<Utc>,
}

/// For creating a supported airport via the admin API.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateSupportedAirport {
    pub iata_code: String,
    pub country_code: String,
    pub name: String,
    pub skytrax_review_slug: Option<String>,
    pub skytrax_rating_slug: Option<String>,
    pub google_maps_url: Option<String>,
}

/// For updating a supported airport via the admin API.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateSupportedAirport {
    pub name: Option<String>,
    pub country_code: Option<String>,
    pub skytrax_review_slug: Option<String>,
    pub skytrax_rating_slug: Option<String>,
    pub google_maps_url: Option<String>,
    pub enabled: Option<bool>,
}
