use axum::extract::{Path, State};
use axum::Json;

use crate::server::AppState;
use super::types::*;

#[utoipa::path(get, path = "/api/v1/airports/{iata}/routes",
    summary = "Airport routes",
    description = "Full list of routes from this airport, including destination details, airline, and monthly flight frequency.",
    params(("iata" = String, Path, description = "IATA airport code")),
    responses(
        (status = 200, description = "Route list", body = Vec<V1Route>),
        (status = 404, description = "Airport not found", body = ApiError),
    ),
    tag = "Airports"
)]
pub async fn get_airport_routes(
    State(state): State<AppState>,
    Path(iata): Path<String>,
) -> ApiResult<Vec<V1Route>> {
    let airport = super::lookup_airport(&state.pool, &iata.to_uppercase()).await?;

    sqlx::query_as::<_, V1Route>(
        "SELECT r.destination_iata, a2.name as destination_name,
                a2.city as destination_city, a2.country as destination_country_code,
                r.airline_name, r.airline_iata, r.flights_per_month
         FROM routes r
         LEFT JOIN all_airports a2 ON a2.icao = r.destination_icao
         WHERE r.origin_id = $1
         ORDER BY r.flights_per_month DESC NULLS LAST",
    )
    .bind(airport.id)
    .fetch_all(&state.pool)
    .await
    .map(Json)
    .map_err(|_| ApiError::internal())
}

#[utoipa::path(get, path = "/api/v1/airports/{iata}/passengers",
    summary = "Passenger history",
    description = "Yearly passenger counts including domestic/international breakdown and aircraft movements.",
    params(("iata" = String, Path, description = "IATA airport code")),
    responses(
        (status = 200, description = "Yearly passenger data", body = Vec<V1PaxYear>),
        (status = 404, description = "Airport not found", body = ApiError),
    ),
    tag = "Airports"
)]
pub async fn get_airport_passengers(
    State(state): State<AppState>,
    Path(iata): Path<String>,
) -> ApiResult<Vec<V1PaxYear>> {
    let airport = super::lookup_airport(&state.pool, &iata.to_uppercase()).await?;

    sqlx::query_as::<_, V1PaxYear>(
        "SELECT year, total_pax, domestic_pax, international_pax, aircraft_movements
         FROM pax_yearly WHERE airport_id = $1 ORDER BY year",
    )
    .bind(airport.id)
    .fetch_all(&state.pool)
    .await
    .map(Json)
    .map_err(|_| ApiError::internal())
}

#[utoipa::path(get, path = "/api/v1/airports/{iata}/operational",
    summary = "Operational statistics",
    description = "Monthly operational data: flight counts, delay percentage, average delay minutes, cancellation rate, and delay cause breakdown (weather, carrier, ATC, airport).",
    params(("iata" = String, Path, description = "IATA airport code")),
    responses(
        (status = 200, description = "Monthly operational stats", body = Vec<V1OperationalStat>),
        (status = 404, description = "Airport not found", body = ApiError),
    ),
    tag = "Airports"
)]
pub async fn get_airport_operational(
    State(state): State<AppState>,
    Path(iata): Path<String>,
) -> ApiResult<Vec<V1OperationalStat>> {
    let airport = super::lookup_airport(&state.pool, &iata.to_uppercase()).await?;

    sqlx::query_as::<_, V1OperationalStat>(
        "SELECT period_year as year, period_month as month,
                total_flights, delayed_flights,
                delay_pct::float8, avg_delay_minutes::float8,
                cancelled_flights, cancellation_pct::float8,
                delay_weather_pct::float8, delay_carrier_pct::float8,
                delay_atc_pct::float8, delay_airport_pct::float8
         FROM operational_stats WHERE airport_id = $1
         ORDER BY period_year, period_month",
    )
    .bind(airport.id)
    .fetch_all(&state.pool)
    .await
    .map(Json)
    .map_err(|_| ApiError::internal())
}

#[utoipa::path(get, path = "/api/v1/airports/{iata}/sentiment",
    summary = "Sentiment history",
    description = "Quarterly sentiment snapshots by source (Google, Skytrax). Includes average rating (1-10 scale), review count, and positive/negative/neutral percentages.",
    params(("iata" = String, Path, description = "IATA airport code")),
    responses(
        (status = 200, description = "Quarterly sentiment data", body = Vec<V1SentimentSnapshot>),
        (status = 404, description = "Airport not found", body = ApiError),
    ),
    tag = "Airports"
)]
pub async fn get_airport_sentiment(
    State(state): State<AppState>,
    Path(iata): Path<String>,
) -> ApiResult<Vec<V1SentimentSnapshot>> {
    let airport = super::lookup_airport(&state.pool, &iata.to_uppercase()).await?;

    sqlx::query_as::<_, V1SentimentSnapshot>(
        "SELECT source, snapshot_year as year, snapshot_quarter as quarter,
                avg_rating::float8, review_count,
                positive_pct::float8, negative_pct::float8, neutral_pct::float8
         FROM sentiment_snapshots WHERE airport_id = $1
         ORDER BY snapshot_year, snapshot_quarter",
    )
    .bind(airport.id)
    .fetch_all(&state.pool)
    .await
    .map(Json)
    .map_err(|_| ApiError::internal())
}
