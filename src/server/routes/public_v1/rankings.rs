use axum::extract::State;
use axum::Json;

use crate::server::AppState;
use super::types::*;

#[utoipa::path(get, path = "/api/v1/airports/rankings",
    summary = "Airport rankings",
    description = "All scored airports ranked by composite score. Same data as /airports, provided as a semantic alias.",
    responses((status = 200, description = "Ranked airports", body = Vec<V1AirportListItem>)),
    tag = "Airports"
)]
pub async fn get_rankings(
    state: State<AppState>,
) -> ApiResult<Vec<V1AirportListItem>> {
    super::airports::list_airports(state).await
}

#[utoipa::path(get, path = "/api/v1/airports/delays",
    summary = "Delay rankings",
    description = "Airports ranked by average delay percentage over the last 12 months. Higher values mean more delays. Only includes airports with at least 3 months of data.",
    responses((status = 200, description = "Airports by delay", body = Vec<V1DelayRankingItem>)),
    tag = "Airports"
)]
pub async fn get_delays(
    State(state): State<AppState>,
) -> ApiResult<Vec<V1DelayRankingItem>> {
    sqlx::query_as::<_, V1DelayRankingItem>(
        "SELECT a.iata_code, a.name, a.city, a.country_code,
                AVG(os.delay_pct)::float8 as avg_delay_pct
         FROM operational_stats os
         INNER JOIN airports a ON a.id = os.airport_id
         WHERE os.delay_pct IS NOT NULL
           AND os.period_year >= EXTRACT(YEAR FROM NOW())::int - 1
         GROUP BY a.iata_code, a.name, a.city, a.country_code
         HAVING COUNT(*) >= 3
         ORDER BY avg_delay_pct DESC",
    )
    .fetch_all(&state.pool)
    .await
    .map(Json)
    .map_err(|_| ApiError::internal())
}

#[utoipa::path(get, path = "/api/v1/airports/busiest",
    summary = "Busiest airports",
    description = "Top 10 airports by annual passenger count (most recent year with data).",
    responses((status = 200, description = "Busiest airports", body = Vec<V1BusiestItem>)),
    tag = "Airports"
)]
pub async fn get_busiest(
    State(state): State<AppState>,
) -> ApiResult<Vec<V1BusiestItem>> {
    sqlx::query_as::<_, V1BusiestItem>(
        "SELECT a.iata_code, a.name, a.city, a.country_code,
                p.year::int2 AS year, p.total_pax
         FROM pax_yearly p
         INNER JOIN airports a ON a.id = p.airport_id
         WHERE p.total_pax IS NOT NULL
           AND a.in_seed_set = true
           AND p.year = (SELECT MAX(p2.year) FROM pax_yearly p2
                         WHERE p2.airport_id = p.airport_id AND p2.total_pax IS NOT NULL)
         ORDER BY p.total_pax DESC
         LIMIT 10",
    )
    .fetch_all(&state.pool)
    .await
    .map(Json)
    .map_err(|_| ApiError::internal())
}

#[utoipa::path(get, path = "/api/v1/airports/best-reviewed",
    summary = "Best reviewed airports",
    description = "Top 10 airports by average sentiment rating (1-10 scale) from the last 2 years. Requires at least 100 reviews to qualify.",
    responses((status = 200, description = "Best reviewed airports", body = Vec<V1BestReviewedItem>)),
    tag = "Airports"
)]
pub async fn get_best_reviewed(
    State(state): State<AppState>,
) -> ApiResult<Vec<V1BestReviewedItem>> {
    sqlx::query_as::<_, V1BestReviewedItem>(
        "SELECT a.iata_code, a.name, a.city, a.country_code,
                AVG(ss.avg_rating)::float8 AS avg_rating,
                SUM(ss.review_count)::int8 AS review_count
         FROM sentiment_snapshots ss
         INNER JOIN airports a ON a.id = ss.airport_id
         WHERE ss.avg_rating IS NOT NULL
           AND a.in_seed_set = true
           AND ss.snapshot_year >= EXTRACT(YEAR FROM NOW())::int - 2
         GROUP BY a.iata_code, a.name, a.city, a.country_code
         HAVING SUM(ss.review_count) >= 100
         ORDER BY avg_rating DESC
         LIMIT 10",
    )
    .fetch_all(&state.pool)
    .await
    .map(Json)
    .map_err(|_| ApiError::internal())
}

#[utoipa::path(get, path = "/api/v1/airports/most-connected",
    summary = "Most connected airports",
    description = "Airports ranked by number of unique route destinations.",
    responses((status = 200, description = "Most connected airports", body = Vec<V1ConnectivityItem>)),
    tag = "Airports"
)]
pub async fn get_most_connected(
    State(state): State<AppState>,
) -> ApiResult<Vec<V1ConnectivityItem>> {
    sqlx::query_as::<_, V1ConnectivityItem>(
        "SELECT a.iata_code, a.name, a.city, a.country_code,
                COUNT(DISTINCT r.destination_iata)::int8 AS route_count
         FROM routes r
         INNER JOIN airports a ON a.id = r.origin_id
         WHERE a.in_seed_set = true
         GROUP BY a.iata_code, a.name, a.city, a.country_code
         ORDER BY route_count DESC",
    )
    .fetch_all(&state.pool)
    .await
    .map(Json)
    .map_err(|_| ApiError::internal())
}

#[utoipa::path(get, path = "/api/v1/airports/map",
    summary = "Map data",
    description = "All tracked airports with latitude, longitude, and score for map visualizations.",
    responses((status = 200, description = "Airports with coordinates", body = Vec<V1MapItem>)),
    tag = "Airports"
)]
pub async fn get_map(
    State(state): State<AppState>,
) -> ApiResult<Vec<V1MapItem>> {
    sqlx::query_as::<_, V1MapItem>(
        "SELECT a.iata_code, a.name, a.city, a.country_code,
                s.score_total::float8 as score_total,
                ST_Y(a.location::geometry)::float8 as lat,
                ST_X(a.location::geometry)::float8 as lng
         FROM airports a
         LEFT JOIN airport_scores s ON s.airport_id = a.id AND s.is_latest = true
         WHERE a.in_seed_set = true AND a.location IS NOT NULL
         ORDER BY s.score_total DESC NULLS LAST",
    )
    .fetch_all(&state.pool)
    .await
    .map(Json)
    .map_err(|_| ApiError::internal())
}
