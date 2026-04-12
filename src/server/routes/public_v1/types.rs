use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// ── Error response ──────────────────────────────────────────

#[derive(Debug, Serialize, ToSchema)]
pub struct ApiError {
    pub error: String,
    pub status: u16,
}

impl ApiError {
    pub fn not_found(msg: &str) -> (axum::http::StatusCode, axum::Json<ApiError>) {
        (
            axum::http::StatusCode::NOT_FOUND,
            axum::Json(ApiError {
                error: msg.to_string(),
                status: 404,
            }),
        )
    }

    pub fn internal() -> (axum::http::StatusCode, axum::Json<ApiError>) {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(ApiError {
                error: "Internal server error".to_string(),
                status: 500,
            }),
        )
    }
}

pub type ApiResult<T> = Result<axum::Json<T>, (axum::http::StatusCode, axum::Json<ApiError>)>;

// ── List / search ───────────────────────────────────────────

#[derive(Debug, Serialize, ToSchema, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct V1AirportListItem {
    pub iata_code: String,
    pub name: String,
    pub city: String,
    pub country_code: String,
    pub score_total: Option<f64>,
    pub score_sentiment_velocity: Option<f64>,
    pub award_count: Option<i64>,
}

#[derive(Debug, Serialize, ToSchema, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct V1SearchResult {
    pub iata_code: String,
    pub name: String,
    pub city: String,
    pub country_code: String,
    pub score_total: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct SearchParams {
    pub q: String,
}

// ── Rankings ────────────────────────────────────────────────

#[derive(Debug, Serialize, ToSchema, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct V1DelayRankingItem {
    pub iata_code: String,
    pub name: String,
    pub city: String,
    pub country_code: String,
    pub avg_delay_pct: f64,
}

#[derive(Debug, Serialize, ToSchema, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct V1BusiestItem {
    pub iata_code: String,
    pub name: String,
    pub city: String,
    pub country_code: String,
    pub year: i16,
    pub total_pax: i64,
}

#[derive(Debug, Serialize, ToSchema, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct V1BestReviewedItem {
    pub iata_code: String,
    pub name: String,
    pub city: String,
    pub country_code: String,
    pub avg_rating: f64,
    pub review_count: i64,
}

#[derive(Debug, Serialize, ToSchema, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct V1ConnectivityItem {
    pub iata_code: String,
    pub name: String,
    pub city: String,
    pub country_code: String,
    pub route_count: i64,
}

#[derive(Debug, Serialize, ToSchema, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct V1MapItem {
    pub iata_code: String,
    pub name: String,
    pub city: String,
    pub country_code: String,
    pub score_total: Option<f64>,
    pub lat: Option<f64>,
    pub lng: Option<f64>,
}

// ── Airport detail ──────────────────────────────────────────

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct V1AirportDetail {
    pub iata_code: String,
    pub icao_code: Option<String>,
    pub name: String,
    pub short_name: Option<String>,
    pub city: String,
    pub country_code: String,
    pub elevation_ft: Option<i32>,
    pub timezone: Option<String>,
    pub terminal_count: Option<i16>,
    pub total_gates: Option<i16>,
    pub opened_year: Option<i16>,
    pub last_major_reno: Option<i16>,
    pub annual_capacity_m: Option<f64>,
    pub annual_pax_latest_m: Option<f64>,
    pub latest_pax_year: Option<i16>,
    pub wikipedia_url: Option<String>,
    pub website_url: Option<String>,

    pub operator: Option<V1Operator>,
    pub country: Option<V1Country>,
    pub scores: Option<V1Scores>,
    pub ranking: V1Ranking,
    pub score_status: String,
    pub sentiment: Option<V1SentimentSummary>,
    pub routes: V1RouteSummary,
    pub runways: Vec<V1Runway>,
    pub awards: Vec<V1Award>,
    pub ground_transport: Option<V1GroundTransport>,
    pub lounges: Vec<V1Lounge>,
    pub hub_status: Vec<V1HubStatus>,
    pub carbon_accreditation: Option<V1CarbonAccreditation>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct V1Operator {
    pub name: String,
    pub short_name: Option<String>,
    pub country_code: Option<String>,
    pub org_type: Option<String>,
    pub ownership_model: Option<String>,
    pub public_share_pct: Option<f64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct V1Country {
    pub code: String,
    pub name: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct V1Scores {
    pub total: f64,
    pub infrastructure: Option<f64>,
    pub operational: Option<f64>,
    pub sentiment: Option<f64>,
    pub sentiment_velocity: Option<f64>,
    pub connectivity: Option<f64>,
    pub operator: Option<f64>,
    pub commentary: Option<String>,
    pub scored_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct V1Ranking {
    pub position: Option<i64>,
    pub total: i64,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct V1SentimentSummary {
    pub avg_rating: Option<f64>,
    pub review_count: i64,
    pub positive_pct: Option<f64>,
    pub negative_pct: Option<f64>,
    pub neutral_pct: Option<f64>,
    pub sub_scores: V1SubScores,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct V1SubScores {
    pub queuing: Option<f64>,
    pub cleanliness: Option<f64>,
    pub staff: Option<f64>,
    pub food_bev: Option<f64>,
    pub shopping: Option<f64>,
    pub wifi: Option<f64>,
    pub wayfinding: Option<f64>,
    pub transport: Option<f64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct V1RouteSummary {
    pub count: i64,
    pub airlines: i64,
}

#[derive(Debug, Serialize, ToSchema, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct V1Runway {
    pub ident: Option<String>,
    pub length_ft: Option<i32>,
    pub width_ft: Option<i32>,
    pub surface: Option<String>,
    pub lighted: Option<bool>,
}

#[derive(Debug, Serialize, ToSchema, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct V1Award {
    pub source: String,
    pub year: i16,
    pub category: String,
    pub region: Option<String>,
    pub size_bucket: Option<String>,
    pub rank: Option<i16>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct V1GroundTransport {
    pub transport_modes_count: Option<i16>,
    pub has_direct_rail: Option<bool>,
    pub has_metro: Option<bool>,
    pub has_bus: Option<bool>,
    pub has_rail: Option<bool>,
    pub has_tram: Option<bool>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct V1Lounge {
    pub lounge_name: String,
    pub terminal: Option<String>,
    pub source: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct V1HubStatus {
    pub airline_name: String,
    pub status_type: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct V1CarbonAccreditation {
    pub level: i16,
    pub level_name: String,
    pub report_year: Option<i16>,
}

// ── Sub-endpoints ───────────────────────────────────────────

#[derive(Debug, Serialize, ToSchema, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct V1Route {
    pub destination_iata: Option<String>,
    pub destination_name: Option<String>,
    pub destination_city: Option<String>,
    pub destination_country_code: Option<String>,
    pub airline_name: Option<String>,
    pub airline_iata: Option<String>,
    pub flights_per_month: Option<i32>,
}

#[derive(Debug, Serialize, ToSchema, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct V1PaxYear {
    pub year: i16,
    pub total_pax: Option<i64>,
    pub domestic_pax: Option<i64>,
    pub international_pax: Option<i64>,
    pub aircraft_movements: Option<i32>,
}

#[derive(Debug, Serialize, ToSchema, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct V1OperationalStat {
    pub year: i16,
    pub month: Option<i16>,
    pub total_flights: Option<i32>,
    pub delayed_flights: Option<i32>,
    pub delay_pct: Option<f64>,
    pub avg_delay_minutes: Option<f64>,
    pub cancelled_flights: Option<i32>,
    pub cancellation_pct: Option<f64>,
    pub delay_weather_pct: Option<f64>,
    pub delay_carrier_pct: Option<f64>,
    pub delay_atc_pct: Option<f64>,
    pub delay_airport_pct: Option<f64>,
}

#[derive(Debug, Serialize, ToSchema, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct V1SentimentSnapshot {
    pub source: String,
    pub year: i16,
    pub quarter: Option<i16>,
    pub avg_rating: Option<f64>,
    pub review_count: Option<i32>,
    pub positive_pct: Option<f64>,
    pub negative_pct: Option<f64>,
    pub neutral_pct: Option<f64>,
}

// ── Countries ───────────────────────────────────────────────

#[derive(Debug, Serialize, ToSchema, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct V1CountrySummary {
    pub code: String,
    pub name: String,
    pub airport_count: i64,
    pub avg_score: Option<f64>,
    pub best_score: Option<f64>,
    pub worst_score: Option<f64>,
    pub total_pax: Option<i64>,
    pub avg_sentiment_positive: Option<f64>,
    pub avg_on_time: Option<f64>,
    pub total_routes: Option<i64>,
    pub lat: Option<f64>,
    pub lng: Option<f64>,
}

// ── Operators ───────────────────────────────────────────────

#[derive(Debug, Serialize, ToSchema, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct V1OperatorListItem {
    pub slug: String,
    #[serde(skip)]
    #[allow(dead_code)]
    pub id: i32,
    pub name: String,
    pub short_name: Option<String>,
    pub country_code: Option<String>,
    pub org_type: Option<String>,
    pub ownership_model: Option<String>,
    pub public_share_pct: Option<f64>,
    pub airport_count: i64,
    pub avg_score: Option<f64>,
    pub total_pax: Option<i64>,
    pub avg_delay_pct: Option<f64>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct V1OperatorDetail {
    pub name: String,
    pub short_name: Option<String>,
    pub country_code: Option<String>,
    pub org_type: Option<String>,
    pub ownership_model: Option<String>,
    pub public_share_pct: Option<f64>,
    pub notes: Option<String>,
    pub airports: Vec<V1OperatorAirport>,
}

#[derive(Debug, Serialize, ToSchema, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct V1OperatorAirport {
    pub iata_code: String,
    pub name: String,
    pub city: String,
    pub country_code: String,
    pub score_total: Option<f64>,
    pub avg_delay_pct: Option<f64>,
    pub latest_pax: Option<i64>,
}

// ── Awards ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct AwardsFilter {
    pub year: Option<i16>,
    pub source: Option<String>,
    pub region: Option<String>,
    pub iata: Option<String>,
}

#[derive(Debug, Serialize, ToSchema, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct V1AwardWithAirport {
    pub iata_code: String,
    pub source: String,
    pub year: i16,
    pub category: String,
    pub region: Option<String>,
    pub size_bucket: Option<String>,
    pub rank: Option<i16>,
}
