use chrono::{NaiveDate, DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

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

// ── Runway ────────────────────────────────────────────────────

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Runway {
    pub id: i32,
    pub airport_id: i32,
    pub ident: Option<String>,
    pub le_ident: Option<String>,
    pub he_ident: Option<String>,
    pub length_ft: Option<i32>,
    pub width_ft: Option<i32>,
    pub surface: Option<String>,
    pub lighted: Option<bool>,
    pub closed: Option<bool>,
    pub created_at: Option<DateTime<Utc>>,
}

// ── Frequency ─────────────────────────────────────────────────

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Frequency {
    pub id: i32,
    pub airport_id: i32,
    pub freq_type: Option<String>,
    pub description: Option<String>,
    pub frequency_mhz: rust_decimal::Decimal,
    pub created_at: Option<DateTime<Utc>>,
}

// ── Passenger traffic ─────────────────────────────────────────

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct PaxYearly {
    pub id: i32,
    pub airport_id: i32,
    pub year: i16,
    pub total_pax: Option<i64>,
    pub domestic_pax: Option<i64>,
    pub international_pax: Option<i64>,
    pub aircraft_movements: Option<i32>,
    pub cargo_tonnes: Option<rust_decimal::Decimal>,
    pub source: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
}

// ── Operational stats ─────────────────────────────────────────

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct OperationalStat {
    pub id: i32,
    pub airport_id: i32,
    pub period_year: i16,
    pub period_month: Option<i16>,
    pub period_type: String,
    pub total_flights: Option<i32>,
    pub delayed_flights: Option<i32>,
    pub delay_pct: Option<rust_decimal::Decimal>,
    pub avg_delay_minutes: Option<rust_decimal::Decimal>,
    pub cancelled_flights: Option<i32>,
    pub cancellation_pct: Option<rust_decimal::Decimal>,
    pub source: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
}

// ── METAR daily ───────────────────────────────────────────────

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct MetarDaily {
    pub id: i32,
    pub airport_id: i32,
    pub observation_date: NaiveDate,
    pub avg_temp_c: Option<rust_decimal::Decimal>,
    pub min_temp_c: Option<rust_decimal::Decimal>,
    pub max_temp_c: Option<rust_decimal::Decimal>,
    pub avg_visibility_m: Option<rust_decimal::Decimal>,
    pub min_visibility_m: Option<rust_decimal::Decimal>,
    pub avg_wind_speed_kt: Option<rust_decimal::Decimal>,
    pub max_wind_speed_kt: Option<rust_decimal::Decimal>,
    pub max_wind_gust_kt: Option<rust_decimal::Decimal>,
    pub precipitation_flag: Option<bool>,
    pub thunderstorm_flag: Option<bool>,
    pub fog_flag: Option<bool>,
    pub low_ceiling_flag: Option<bool>,
    pub metar_count: Option<i32>,
    pub created_at: Option<DateTime<Utc>>,
}

// ── Routes ────────────────────────────────────────────────────

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Route {
    pub id: i32,
    pub origin_id: i32,
    pub destination_id: Option<i32>,
    pub destination_icao: Option<String>,
    pub destination_iata: Option<String>,
    pub airline_icao: Option<String>,
    pub airline_iata: Option<String>,
    pub airline_name: Option<String>,
    pub flights_per_month: Option<i32>,
    pub first_observed: Option<NaiveDate>,
    pub last_observed: Option<NaiveDate>,
    pub data_source: String,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

// ── Reviews raw ───────────────────────────────────────────────

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ReviewRaw {
    pub id: i32,
    pub airport_id: i32,
    pub source: String,
    pub review_date: Option<NaiveDate>,
    pub author: Option<String>,
    pub author_country: Option<String>,
    pub overall_rating: Option<i16>,
    pub review_title: Option<String>,
    pub review_text: Option<String>,
    pub source_url: Option<String>,
    pub processed: Option<bool>,
    pub created_at: Option<DateTime<Utc>>,
}

// ── Sentiment snapshots ───────────────────────────────────────

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct SentimentSnapshot {
    pub id: i32,
    pub airport_id: i32,
    pub source: String,
    pub snapshot_year: i16,
    pub snapshot_quarter: Option<i16>,
    pub avg_rating: Option<rust_decimal::Decimal>,
    pub review_count: Option<i32>,
    pub positive_pct: Option<rust_decimal::Decimal>,
    pub negative_pct: Option<rust_decimal::Decimal>,
    pub neutral_pct: Option<rust_decimal::Decimal>,
    pub skytrax_stars: Option<i16>,
    pub notes: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
}

// ── Airport scores ────────────────────────────────────────────

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct AirportScore {
    pub id: i32,
    pub airport_id: i32,
    pub score_version: String,
    pub scored_at: DateTime<Utc>,
    pub reference_year: i16,
    pub score_infrastructure: Option<rust_decimal::Decimal>,
    pub score_operational: Option<rust_decimal::Decimal>,
    pub score_sentiment: Option<rust_decimal::Decimal>,
    pub score_sentiment_velocity: Option<rust_decimal::Decimal>,
    pub score_connectivity: Option<rust_decimal::Decimal>,
    pub score_operator: Option<rust_decimal::Decimal>,
    pub score_total: Option<rust_decimal::Decimal>,
    pub is_latest: Option<bool>,
    pub notes: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
}

// ── Pipeline runs ─────────────────────────────────────────────

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct PipelineRun {
    pub id: i32,
    pub airport_id: Option<i32>,
    pub source: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub status: String,
    pub records_processed: Option<i32>,
    pub last_record_date: Option<NaiveDate>,
    pub error_message: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
}

// ── Airport slugs ─────────────────────────────────────────────

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct AirportSlug {
    pub airport_id: i32,
    pub source: String,
    pub slug: String,
}

// ── Organisations ─────────────────────────────────────────────

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Organisation {
    pub id: i32,
    pub name: String,
    pub short_name: Option<String>,
    pub country_code: Option<String>,
    pub org_type: String,
    pub ownership_model: Option<String>,
    pub public_share_pct: Option<rust_decimal::Decimal>,
    pub founded_year: Option<i16>,
    pub notes: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
}

// ── Fetch result (returned by every fetcher) ──────────────────

#[derive(Debug, Clone)]
pub struct FetchResult {
    pub records_processed: i32,
    pub last_record_date: Option<NaiveDate>,
}
