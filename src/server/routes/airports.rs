use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::prelude::ToPrimitive;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use utoipa::ToSchema;

use crate::server::AppState;

// ── Response types ──────────────────────────────────────────────

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AirportDetailResponse {
    // Core fields
    pub id: i32,
    pub iata_code: Option<String>,
    pub icao_code: Option<String>,
    pub name: String,
    pub short_name: Option<String>,
    pub city: String,
    pub country_code: String,
    pub elevation_ft: Option<i32>,
    pub timezone: Option<String>,
    pub airport_type: String,
    pub terminal_count: Option<i16>,
    pub total_gates: Option<i16>,
    pub opened_year: Option<i16>,
    pub last_major_reno: Option<i16>,
    pub annual_capacity_m: Option<f64>,
    pub annual_pax_latest_m: Option<f64>,
    pub latest_pax_year: Option<i16>,
    pub wikipedia_url: Option<String>,
    pub website_url: Option<String>,
    pub skytrax_url: Option<String>,

    // Relations
    pub operator: Option<OrganisationResponse>,
    pub owner: Option<OrganisationResponse>,
    pub country: Option<CountryResponse>,
    pub runways: Vec<RunwayResponse>,
    pub pax_yearly: Vec<PaxYearlyResponse>,
    pub operational_stats: Vec<OperationalStatResponse>,
    pub sentiment_snapshots: Vec<SentimentSnapshotResponse>,
    pub scores: Vec<ScoreResponse>,
    pub routes_out: Vec<RouteResponse>,
    pub wikipedia_snapshots: Vec<WikipediaSnapshotResponse>,
    pub carbon_accreditation: Vec<CarbonAccreditationResponse>,
    pub ground_transport: Vec<GroundTransportResponse>,
    pub lounges: Vec<LoungeResponse>,
    pub hub_status: Vec<HubStatusResponse>,

    // Derived
    pub recent_reviews: Vec<RecentReviewResponse>,
    pub ranking: RankingResponse,
    pub google_agg: Option<GoogleAggResponse>,
    pub source_breakdown: Vec<SourceBreakdownResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OrganisationResponse {
    pub id: i32,
    pub name: String,
    pub short_name: Option<String>,
    pub country_code: Option<String>,
    pub org_type: Option<String>,
    pub ownership_model: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CountryResponse {
    pub code: String,
    pub name: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RunwayResponse {
    pub id: i32,
    pub ident: Option<String>,
    pub length_ft: Option<i32>,
    pub width_ft: Option<i32>,
    pub surface: Option<String>,
    pub lighted: Option<bool>,
    pub closed: Option<bool>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PaxYearlyResponse {
    pub year: i16,
    pub total_pax: Option<i64>,
    pub domestic_pax: Option<i64>,
    pub international_pax: Option<i64>,
    pub aircraft_movements: Option<i32>,
    pub source: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OperationalStatResponse {
    pub period_year: i16,
    pub period_month: Option<i16>,
    pub total_flights: Option<i32>,
    pub delayed_flights: Option<i32>,
    pub delay_pct: Option<f64>,
    pub avg_delay_minutes: Option<f64>,
    pub cancelled_flights: Option<i32>,
    pub cancellation_pct: Option<f64>,
    pub delay_weather_pct: Option<f64>,
    pub delay_carrier_pct: Option<f64>,
    pub delay_atc_pct: Option<f64>,
    pub delay_security_pct: Option<f64>,
    pub delay_airport_pct: Option<f64>,
    pub source: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SentimentSnapshotResponse {
    pub source: Option<String>,
    pub snapshot_year: Option<i16>,
    pub snapshot_quarter: Option<i16>,
    pub avg_rating: Option<f64>,
    pub review_count: Option<i32>,
    pub positive_pct: Option<f64>,
    pub negative_pct: Option<f64>,
    pub neutral_pct: Option<f64>,
    pub score_queuing: Option<f64>,
    pub score_cleanliness: Option<f64>,
    pub score_staff: Option<f64>,
    pub score_food_bev: Option<f64>,
    pub score_shopping: Option<f64>,
    pub score_wifi: Option<f64>,
    pub score_wayfinding: Option<f64>,
    pub score_transport: Option<f64>,
    pub skytrax_stars: Option<i16>,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ScoreResponse {
    pub score_version: Option<String>,
    pub scored_at: Option<String>,
    pub score_infrastructure: Option<f64>,
    pub score_operational: Option<f64>,
    pub score_sentiment: Option<f64>,
    pub score_sentiment_velocity: Option<f64>,
    pub score_connectivity: Option<f64>,
    pub score_operator: Option<f64>,
    pub score_total: Option<f64>,
    pub commentary: Option<String>,
    pub is_latest: Option<bool>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RouteDestinationResponse {
    pub name: Option<String>,
    pub city: Option<String>,
    pub country_code: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RouteResponse {
    pub destination_iata: Option<String>,
    pub destination_icao: Option<String>,
    pub airline_name: Option<String>,
    pub airline_iata: Option<String>,
    pub flights_per_month: Option<i32>,
    pub destination: Option<RouteDestinationResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WikipediaSnapshotResponse {
    pub fetched_at: Option<String>,
    pub opened_year: Option<i16>,
    pub operator_raw: Option<String>,
    pub owner_raw: Option<String>,
    pub terminal_count: Option<i16>,
    pub terminal_names: Option<Vec<String>>,
    pub renovation_notes: Option<String>,
    pub ownership_notes: Option<String>,
    pub milestone_notes: Option<String>,
    pub skytrax_history: Option<serde_json::Value>,
    pub aci_awards: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CarbonAccreditationResponse {
    pub level: Option<i16>,
    pub level_name: Option<String>,
    pub report_year: Option<i16>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GroundTransportResponse {
    pub transport_modes_count: Option<i16>,
    pub has_direct_rail: Option<bool>,
    pub has_metro: Option<bool>,
    pub has_bus: Option<bool>,
    pub has_rail: Option<bool>,
    pub has_tram: Option<bool>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LoungeResponse {
    pub lounge_name: Option<String>,
    pub terminal: Option<String>,
    pub source: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HubStatusResponse {
    pub airline_name: Option<String>,
    pub status_type: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RecentReviewResponse {
    pub review_date: Option<NaiveDate>,
    pub overall_rating: Option<i16>,
    pub review_text: Option<String>,
    pub source: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RankingResponse {
    pub position: Option<i64>,
    pub total: i64,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GoogleAggResponse {
    pub rating: Option<f64>,
    pub count: i64,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SourceBreakdownResponse {
    pub source: String,
    pub count: i64,
}

// ── Row types for sqlx queries ──────────────────────────────────

#[derive(FromRow)]
struct OrgRow {
    id: i32,
    name: String,
    short_name: Option<String>,
    country_code: Option<String>,
    org_type: Option<String>,
    ownership_model: Option<String>,
}

#[derive(FromRow)]
struct CountryRow {
    code: String,
    name: String,
}

#[derive(FromRow)]
struct RunwayRow {
    id: i32,
    ident: Option<String>,
    length_ft: Option<i32>,
    width_ft: Option<i32>,
    surface: Option<String>,
    lighted: Option<bool>,
    closed: Option<bool>,
}

#[derive(FromRow)]
struct PaxYearlyRow {
    year: i16,
    total_pax: Option<i64>,
    domestic_pax: Option<i64>,
    international_pax: Option<i64>,
    aircraft_movements: Option<i32>,
    source: Option<String>,
}

#[derive(FromRow)]
struct OperationalStatRow {
    period_year: i16,
    period_month: Option<i16>,
    total_flights: Option<i32>,
    delayed_flights: Option<i32>,
    delay_pct: Option<rust_decimal::Decimal>,
    avg_delay_minutes: Option<rust_decimal::Decimal>,
    cancelled_flights: Option<i32>,
    cancellation_pct: Option<rust_decimal::Decimal>,
    delay_weather_pct: Option<rust_decimal::Decimal>,
    delay_carrier_pct: Option<rust_decimal::Decimal>,
    delay_atc_pct: Option<rust_decimal::Decimal>,
    delay_security_pct: Option<rust_decimal::Decimal>,
    delay_airport_pct: Option<rust_decimal::Decimal>,
    source: Option<String>,
}

#[derive(FromRow)]
struct SentimentSnapshotRow {
    source: Option<String>,
    snapshot_year: Option<i16>,
    snapshot_quarter: Option<i16>,
    avg_rating: Option<rust_decimal::Decimal>,
    review_count: Option<i32>,
    positive_pct: Option<rust_decimal::Decimal>,
    negative_pct: Option<rust_decimal::Decimal>,
    neutral_pct: Option<rust_decimal::Decimal>,
    score_queuing: Option<rust_decimal::Decimal>,
    score_cleanliness: Option<rust_decimal::Decimal>,
    score_staff: Option<rust_decimal::Decimal>,
    score_food_bev: Option<rust_decimal::Decimal>,
    score_shopping: Option<rust_decimal::Decimal>,
    score_wifi: Option<rust_decimal::Decimal>,
    score_wayfinding: Option<rust_decimal::Decimal>,
    score_transport: Option<rust_decimal::Decimal>,
    skytrax_stars: Option<i16>,
    notes: Option<String>,
}

#[derive(FromRow)]
struct ScoreRow {
    score_version: Option<String>,
    scored_at: Option<DateTime<Utc>>,
    score_infrastructure: Option<rust_decimal::Decimal>,
    score_operational: Option<rust_decimal::Decimal>,
    score_sentiment: Option<rust_decimal::Decimal>,
    score_sentiment_velocity: Option<rust_decimal::Decimal>,
    score_connectivity: Option<rust_decimal::Decimal>,
    score_operator: Option<rust_decimal::Decimal>,
    score_total: Option<rust_decimal::Decimal>,
    commentary: Option<String>,
    is_latest: Option<bool>,
}

#[derive(FromRow)]
struct RouteRow {
    destination_iata: Option<String>,
    destination_icao: Option<String>,
    airline_name: Option<String>,
    airline_iata: Option<String>,
    flights_per_month: Option<i32>,
    dest_name: Option<String>,
    dest_city: Option<String>,
    dest_country_code: Option<String>,
}

#[derive(FromRow)]
struct WikipediaSnapshotRow {
    fetched_at: Option<DateTime<Utc>>,
    opened_year: Option<i16>,
    operator_raw: Option<String>,
    owner_raw: Option<String>,
    terminal_count: Option<i16>,
    terminal_names: Option<Vec<String>>,
    renovation_notes: Option<String>,
    ownership_notes: Option<String>,
    milestone_notes: Option<String>,
    skytrax_history: Option<serde_json::Value>,
    aci_awards: Option<serde_json::Value>,
}

#[derive(FromRow)]
struct CarbonAccreditationRow {
    level: Option<i16>,
    level_name: Option<String>,
    report_year: Option<i16>,
}

#[derive(FromRow)]
struct GroundTransportRow {
    transport_modes_count: Option<i16>,
    has_direct_rail: Option<bool>,
    has_metro: Option<bool>,
    has_bus: Option<bool>,
    has_rail: Option<bool>,
    has_tram: Option<bool>,
}

#[derive(FromRow)]
struct LoungeRow {
    lounge_name: Option<String>,
    terminal: Option<String>,
    source: Option<String>,
}

#[derive(FromRow)]
struct HubStatusRow {
    airline_name: Option<String>,
    status_type: Option<String>,
}

#[derive(FromRow)]
struct RecentReviewRow {
    review_date: Option<NaiveDate>,
    overall_rating: Option<i16>,
    review_text: Option<String>,
    source: Option<String>,
}

#[derive(FromRow)]
struct RankingRow {
    position: Option<i64>,
    total: i64,
}

#[derive(FromRow)]
struct GoogleAggRow {
    rating: Option<f64>,
    count: Option<i64>,
}

#[derive(FromRow)]
struct SourceBreakdownRow {
    source: String,
    count: Option<i64>,
}

// ── Helper: Decimal -> f64 ──────────────────────────────────────

fn dec_to_f64(d: Option<rust_decimal::Decimal>) -> Option<f64> {
    d.and_then(|v| v.to_f64())
}

// ── Fetcher helpers ─────────────────────────────────────────────

async fn fetch_org(pool: &PgPool, org_id: Option<i32>) -> Option<OrganisationResponse> {
    let id = org_id?;
    sqlx::query_as::<_, OrgRow>(
        "SELECT id, name, short_name, country_code, org_type, ownership_model
         FROM organisations WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .map(|r| OrganisationResponse {
        id: r.id,
        name: r.name,
        short_name: r.short_name,
        country_code: r.country_code,
        org_type: r.org_type,
        ownership_model: r.ownership_model,
    })
}

async fn fetch_country(pool: &PgPool, code: &str) -> Option<CountryResponse> {
    sqlx::query_as::<_, CountryRow>("SELECT iso_code AS code, name FROM countries WHERE iso_code = $1")
        .bind(code)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .map(|r| CountryResponse {
            code: r.code,
            name: r.name,
        })
}

async fn fetch_runways(pool: &PgPool, airport_id: i32) -> Vec<RunwayResponse> {
    sqlx::query_as::<_, RunwayRow>(
        "SELECT id, ident, length_ft, width_ft, surface, lighted, closed
         FROM runways WHERE airport_id = $1 ORDER BY ident",
    )
    .bind(airport_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|r| RunwayResponse {
        id: r.id,
        ident: r.ident,
        length_ft: r.length_ft,
        width_ft: r.width_ft,
        surface: r.surface,
        lighted: r.lighted,
        closed: r.closed,
    })
    .collect()
}

async fn fetch_pax_yearly(pool: &PgPool, airport_id: i32) -> Vec<PaxYearlyResponse> {
    sqlx::query_as::<_, PaxYearlyRow>(
        "SELECT year, total_pax, domestic_pax, international_pax, aircraft_movements, source
         FROM pax_yearly WHERE airport_id = $1 ORDER BY year",
    )
    .bind(airport_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|r| PaxYearlyResponse {
        year: r.year,
        total_pax: r.total_pax,
        domestic_pax: r.domestic_pax,
        international_pax: r.international_pax,
        aircraft_movements: r.aircraft_movements,
        source: r.source,
    })
    .collect()
}

async fn fetch_operational_stats(pool: &PgPool, airport_id: i32) -> Vec<OperationalStatResponse> {
    sqlx::query_as::<_, OperationalStatRow>(
        "SELECT period_year, period_month, total_flights, delayed_flights,
                delay_pct, avg_delay_minutes, cancelled_flights, cancellation_pct,
                delay_weather_pct, delay_carrier_pct, delay_atc_pct,
                delay_security_pct, delay_airport_pct, source
         FROM operational_stats WHERE airport_id = $1
         ORDER BY period_year, period_month",
    )
    .bind(airport_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|r| OperationalStatResponse {
        period_year: r.period_year,
        period_month: r.period_month,
        total_flights: r.total_flights,
        delayed_flights: r.delayed_flights,
        delay_pct: dec_to_f64(r.delay_pct),
        avg_delay_minutes: dec_to_f64(r.avg_delay_minutes),
        cancelled_flights: r.cancelled_flights,
        cancellation_pct: dec_to_f64(r.cancellation_pct),
        delay_weather_pct: dec_to_f64(r.delay_weather_pct),
        delay_carrier_pct: dec_to_f64(r.delay_carrier_pct),
        delay_atc_pct: dec_to_f64(r.delay_atc_pct),
        delay_security_pct: dec_to_f64(r.delay_security_pct),
        delay_airport_pct: dec_to_f64(r.delay_airport_pct),
        source: r.source,
    })
    .collect()
}

async fn fetch_sentiment_snapshots(
    pool: &PgPool,
    airport_id: i32,
) -> Vec<SentimentSnapshotResponse> {
    sqlx::query_as::<_, SentimentSnapshotRow>(
        "SELECT source, snapshot_year, snapshot_quarter, avg_rating, review_count,
                positive_pct, negative_pct, neutral_pct,
                score_queuing, score_cleanliness, score_staff, score_food_bev,
                score_shopping, score_wifi, score_wayfinding, score_transport,
                skytrax_stars, notes
         FROM sentiment_snapshots WHERE airport_id = $1
         ORDER BY snapshot_year, snapshot_quarter",
    )
    .bind(airport_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|r| SentimentSnapshotResponse {
        source: r.source,
        snapshot_year: r.snapshot_year,
        snapshot_quarter: r.snapshot_quarter,
        avg_rating: dec_to_f64(r.avg_rating),
        review_count: r.review_count,
        positive_pct: dec_to_f64(r.positive_pct),
        negative_pct: dec_to_f64(r.negative_pct),
        neutral_pct: dec_to_f64(r.neutral_pct),
        score_queuing: dec_to_f64(r.score_queuing),
        score_cleanliness: dec_to_f64(r.score_cleanliness),
        score_staff: dec_to_f64(r.score_staff),
        score_food_bev: dec_to_f64(r.score_food_bev),
        score_shopping: dec_to_f64(r.score_shopping),
        score_wifi: dec_to_f64(r.score_wifi),
        score_wayfinding: dec_to_f64(r.score_wayfinding),
        score_transport: dec_to_f64(r.score_transport),
        skytrax_stars: r.skytrax_stars,
        notes: r.notes,
    })
    .collect()
}

async fn fetch_scores(pool: &PgPool, airport_id: i32) -> Vec<ScoreResponse> {
    sqlx::query_as::<_, ScoreRow>(
        "SELECT score_version, scored_at, score_infrastructure, score_operational,
                score_sentiment, score_sentiment_velocity, score_connectivity,
                score_operator, score_total, commentary, is_latest
         FROM airport_scores WHERE airport_id = $1
         ORDER BY scored_at DESC",
    )
    .bind(airport_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|r| ScoreResponse {
        score_version: r.score_version,
        scored_at: r.scored_at.map(|dt| dt.to_rfc3339()),
        score_infrastructure: dec_to_f64(r.score_infrastructure),
        score_operational: dec_to_f64(r.score_operational),
        score_sentiment: dec_to_f64(r.score_sentiment),
        score_sentiment_velocity: dec_to_f64(r.score_sentiment_velocity),
        score_connectivity: dec_to_f64(r.score_connectivity),
        score_operator: dec_to_f64(r.score_operator),
        score_total: dec_to_f64(r.score_total),
        commentary: r.commentary,
        is_latest: r.is_latest,
    })
    .collect()
}

async fn fetch_routes(pool: &PgPool, airport_id: i32) -> Vec<RouteResponse> {
    sqlx::query_as::<_, RouteRow>(
        "SELECT r.destination_iata, r.destination_icao, r.airline_name, r.airline_iata,
                r.flights_per_month,
                a.name AS dest_name, a.city AS dest_city, a.country AS dest_country_code
         FROM routes r
         LEFT JOIN all_airports a ON a.icao = r.destination_icao
         WHERE r.origin_id = $1
         ORDER BY r.flights_per_month DESC NULLS LAST",
    )
    .bind(airport_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|r| {
        let destination = if r.dest_name.is_some() || r.dest_city.is_some() {
            Some(RouteDestinationResponse {
                name: r.dest_name,
                city: r.dest_city,
                country_code: r.dest_country_code,
            })
        } else {
            None
        };
        RouteResponse {
            destination_iata: r.destination_iata,
            destination_icao: r.destination_icao,
            airline_name: r.airline_name,
            airline_iata: r.airline_iata,
            flights_per_month: r.flights_per_month,
            destination,
        }
    })
    .collect()
}

async fn fetch_wikipedia_snapshots(
    pool: &PgPool,
    airport_id: i32,
) -> Vec<WikipediaSnapshotResponse> {
    sqlx::query_as::<_, WikipediaSnapshotRow>(
        "SELECT fetched_at, opened_year, operator_raw, owner_raw, terminal_count,
                terminal_names, renovation_notes, ownership_notes, milestone_notes,
                skytrax_history, aci_awards
         FROM wikipedia_snapshots WHERE airport_id = $1
         ORDER BY fetched_at DESC",
    )
    .bind(airport_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|r| WikipediaSnapshotResponse {
        fetched_at: r.fetched_at.map(|dt| dt.to_rfc3339()),
        opened_year: r.opened_year,
        operator_raw: r.operator_raw,
        owner_raw: r.owner_raw,
        terminal_count: r.terminal_count,
        terminal_names: r.terminal_names,
        renovation_notes: r.renovation_notes,
        ownership_notes: r.ownership_notes,
        milestone_notes: r.milestone_notes,
        skytrax_history: r.skytrax_history,
        aci_awards: r.aci_awards,
    })
    .collect()
}

async fn fetch_carbon_accreditation(
    pool: &PgPool,
    airport_id: i32,
) -> Vec<CarbonAccreditationResponse> {
    sqlx::query_as::<_, CarbonAccreditationRow>(
        "SELECT level, level_name, report_year
         FROM carbon_accreditation WHERE airport_id = $1
         ORDER BY report_year DESC",
    )
    .bind(airport_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|r| CarbonAccreditationResponse {
        level: r.level,
        level_name: r.level_name,
        report_year: r.report_year,
    })
    .collect()
}

async fn fetch_ground_transport(pool: &PgPool, airport_id: i32) -> Vec<GroundTransportResponse> {
    sqlx::query_as::<_, GroundTransportRow>(
        "SELECT transport_modes_count, has_direct_rail, has_metro, has_bus, has_rail, has_tram
         FROM ground_transport WHERE airport_id = $1",
    )
    .bind(airport_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|r| GroundTransportResponse {
        transport_modes_count: r.transport_modes_count,
        has_direct_rail: r.has_direct_rail,
        has_metro: r.has_metro,
        has_bus: r.has_bus,
        has_rail: r.has_rail,
        has_tram: r.has_tram,
    })
    .collect()
}

async fn fetch_lounges(pool: &PgPool, airport_id: i32) -> Vec<LoungeResponse> {
    sqlx::query_as::<_, LoungeRow>(
        "SELECT lounge_name, terminal, source
         FROM lounges WHERE airport_id = $1
         ORDER BY lounge_name",
    )
    .bind(airport_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|r| LoungeResponse {
        lounge_name: r.lounge_name,
        terminal: r.terminal,
        source: r.source,
    })
    .collect()
}

async fn fetch_hub_status(pool: &PgPool, airport_id: i32) -> Vec<HubStatusResponse> {
    sqlx::query_as::<_, HubStatusRow>(
        "SELECT airline_name, status_type
         FROM hub_status WHERE airport_id = $1
         ORDER BY airline_name",
    )
    .bind(airport_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|r| HubStatusResponse {
        airline_name: r.airline_name,
        status_type: r.status_type,
    })
    .collect()
}

async fn fetch_recent_reviews(pool: &PgPool, airport_id: i32) -> Vec<RecentReviewResponse> {
    sqlx::query_as::<_, RecentReviewRow>(
        "SELECT review_date, overall_rating, review_text, source
         FROM reviews_raw WHERE airport_id = $1
         AND review_text IS NOT NULL AND review_text != ''
         ORDER BY review_date DESC NULLS LAST
         LIMIT 5",
    )
    .bind(airport_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|r| RecentReviewResponse {
        review_date: r.review_date,
        overall_rating: r.overall_rating,
        review_text: r.review_text,
        source: r.source,
    })
    .collect()
}

async fn fetch_ranking(pool: &PgPool, airport_id: i32) -> RankingResponse {
    let row = sqlx::query_as::<_, RankingRow>(
        "WITH ranked AS (
            SELECT airport_id,
                   RANK() OVER (ORDER BY score_total DESC) AS position,
                   COUNT(*) OVER () AS total
            FROM airport_scores
            WHERE is_latest = TRUE
         )
         SELECT position, total FROM ranked WHERE airport_id = $1",
    )
    .bind(airport_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    match row {
        Some(r) => RankingResponse {
            position: r.position,
            total: r.total,
        },
        None => RankingResponse {
            position: None,
            total: 0,
        },
    }
}

async fn fetch_google_agg(pool: &PgPool, airport_id: i32) -> Option<GoogleAggResponse> {
    let row = sqlx::query_as::<_, GoogleAggRow>(
        "SELECT AVG(overall_rating::float8) AS rating, COUNT(*) AS count
         FROM reviews_raw
         WHERE airport_id = $1 AND source = 'google'",
    )
    .bind(airport_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()?;

    let count = row.count.unwrap_or(0);
    if count == 0 {
        return None;
    }

    Some(GoogleAggResponse {
        rating: row.rating,
        count,
    })
}

async fn fetch_source_breakdown(pool: &PgPool, airport_id: i32) -> Vec<SourceBreakdownResponse> {
    sqlx::query_as::<_, SourceBreakdownRow>(
        "SELECT source, COUNT(*) AS count
         FROM reviews_raw WHERE airport_id = $1
         GROUP BY source
         ORDER BY source",
    )
    .bind(airport_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|r| SourceBreakdownResponse {
        source: r.source,
        count: r.count.unwrap_or(0),
    })
    .collect()
}

// ── Handler ─────────────────────────────────────────────────────

/// Get full airport detail by IATA code.
#[utoipa::path(
    get,
    path = "/api/airports/{iata}",
    params(("iata" = String, Path, description = "IATA airport code")),
    responses(
        (status = 200, description = "Airport detail", body = AirportDetailResponse),
        (status = 404, description = "Airport not found"),
    ),
    tag = "airports"
)]
pub async fn get_airport(
    State(state): State<AppState>,
    Path(iata): Path<String>,
) -> Result<Json<AirportDetailResponse>, StatusCode> {
    let iata_upper = iata.to_uppercase();
    let pool = &state.pool;

    // Fetch the core airport record.
    let airport = crate::db::get_airport_by_iata(pool, &iata_upper)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let aid = airport.id;

    // Fetch all relations sequentially.
    let operator = fetch_org(pool, airport.operator_id).await;
    let owner = fetch_org(pool, airport.owner_id).await;
    let country = fetch_country(pool, &airport.country_code).await;
    let runways = fetch_runways(pool, aid).await;
    let pax_yearly = fetch_pax_yearly(pool, aid).await;
    let operational_stats = fetch_operational_stats(pool, aid).await;
    let sentiment_snapshots = fetch_sentiment_snapshots(pool, aid).await;
    let scores = fetch_scores(pool, aid).await;
    let routes_out = fetch_routes(pool, aid).await;
    let wikipedia_snapshots = fetch_wikipedia_snapshots(pool, aid).await;
    let carbon_accreditation = fetch_carbon_accreditation(pool, aid).await;
    let ground_transport = fetch_ground_transport(pool, aid).await;
    let lounges = fetch_lounges(pool, aid).await;
    let hub_status = fetch_hub_status(pool, aid).await;
    let recent_reviews = fetch_recent_reviews(pool, aid).await;
    let ranking = fetch_ranking(pool, aid).await;
    let google_agg = fetch_google_agg(pool, aid).await;
    let source_breakdown = fetch_source_breakdown(pool, aid).await;

    let response = AirportDetailResponse {
        id: airport.id,
        iata_code: airport.iata_code,
        icao_code: airport.icao_code,
        name: airport.name,
        short_name: airport.short_name,
        city: airport.city,
        country_code: airport.country_code,
        elevation_ft: airport.elevation_ft,
        timezone: airport.timezone,
        airport_type: airport.airport_type,
        terminal_count: airport.terminal_count,
        total_gates: airport.total_gates,
        opened_year: airport.opened_year,
        last_major_reno: airport.last_major_reno,
        annual_capacity_m: dec_to_f64(airport.annual_capacity_m),
        annual_pax_latest_m: dec_to_f64(airport.annual_pax_latest_m),
        latest_pax_year: airport.latest_pax_year,
        wikipedia_url: airport.wikipedia_url,
        website_url: airport.website_url,
        skytrax_url: airport.skytrax_url,
        operator,
        owner,
        country,
        runways,
        pax_yearly,
        operational_stats,
        sentiment_snapshots,
        scores,
        routes_out,
        wikipedia_snapshots,
        carbon_accreditation,
        ground_transport,
        lounges,
        hub_status,
        recent_reviews,
        ranking,
        google_agg,
        source_breakdown,
    };

    Ok(Json(response))
}

// ── Search / List / Rankings / Country endpoints ─────────────

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: String,
}

#[derive(Debug, Serialize, FromRow, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub iata_code: String,
    pub name: String,
    pub city: String,
    pub country_code: String,
    pub score_total: Option<f64>,
}

#[derive(Debug, Serialize, FromRow, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AirportListItem {
    pub iata_code: String,
    pub name: String,
    pub city: String,
    pub country_code: String,
    pub score_total: Option<f64>,
    pub score_sentiment_velocity: Option<f64>,
}

/// Search airports by IATA code, name, or city.
#[utoipa::path(
    get,
    path = "/api/airports/search",
    params(("q" = String, Query, description = "Search query")),
    responses(
        (status = 200, description = "Search results", body = Vec<SearchResult>),
    ),
    tag = "airports"
)]
pub async fn search_airports(
    State(state): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> Result<Json<Vec<SearchResult>>, StatusCode> {
    let escaped_q = params.q.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
    let pattern = format!("%{}%", escaped_q);

    let results = sqlx::query_as::<_, SearchResult>(
        "SELECT a.iata_code, a.name, a.city, a.country_code,
                s.score_total::float8 as score_total
         FROM airports a
         LEFT JOIN airport_scores s ON s.airport_id = a.id AND s.is_latest = TRUE
         WHERE a.iata_code ILIKE $1 OR a.name ILIKE $1 OR a.city ILIKE $1
         ORDER BY s.score_total DESC NULLS LAST
         LIMIT 8",
    )
    .bind(&pattern)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(results))
}

/// List all airports with their latest scores.
#[utoipa::path(
    get,
    path = "/api/airports",
    responses(
        (status = 200, description = "All scored airports", body = Vec<AirportListItem>),
    ),
    tag = "airports"
)]
pub async fn list_airports(
    State(state): State<AppState>,
) -> Result<Json<Vec<AirportListItem>>, StatusCode> {
    let results = sqlx::query_as::<_, AirportListItem>(
        "SELECT a.iata_code, a.name, a.city, a.country_code,
                s.score_total::float8 as score_total,
                s.score_sentiment_velocity::float8 as score_sentiment_velocity
         FROM airport_scores s
         INNER JOIN airports a ON a.id = s.airport_id
         WHERE s.is_latest = TRUE
         ORDER BY s.score_total DESC",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(results))
}

/// Get airport rankings (semantic alias for list).
#[utoipa::path(
    get,
    path = "/api/airports/rankings",
    responses(
        (status = 200, description = "Airport rankings", body = Vec<AirportListItem>),
    ),
    tag = "airports"
)]
pub async fn get_rankings(
    State(state): State<AppState>,
) -> Result<Json<Vec<AirportListItem>>, StatusCode> {
    list_airports(State(state)).await
}

/// List airports in a specific country.
#[utoipa::path(
    get,
    path = "/api/countries/{code}/airports",
    params(("code" = String, Path, description = "ISO country code")),
    responses(
        (status = 200, description = "Airports in country", body = Vec<AirportListItem>),
    ),
    tag = "airports"
)]
pub async fn airports_by_country(
    State(state): State<AppState>,
    Path(code): Path<String>,
) -> Result<Json<Vec<AirportListItem>>, StatusCode> {
    let code_upper = code.to_uppercase();

    let results = sqlx::query_as::<_, AirportListItem>(
        "SELECT a.iata_code, a.name, a.city, a.country_code,
                s.score_total::float8 as score_total,
                s.score_sentiment_velocity::float8 as score_sentiment_velocity
         FROM airports a
         LEFT JOIN airport_scores s ON s.airport_id = a.id AND s.is_latest = TRUE
         WHERE a.country_code = $1 AND a.in_seed_set = TRUE
         ORDER BY s.score_total DESC NULLS LAST",
    )
    .bind(&code_upper)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(results))
}
