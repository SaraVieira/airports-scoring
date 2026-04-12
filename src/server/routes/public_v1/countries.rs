use axum::extract::{Path, State};
use axum::Json;

use crate::server::AppState;
use super::types::*;

#[utoipa::path(get, path = "/api/v1/countries",
    summary = "Country summaries",
    description = "Aggregated stats per country: average score, best/worst airport, total passengers, sentiment, on-time performance, route count, and geographic center.",
    responses((status = 200, description = "Country summaries", body = Vec<V1CountrySummary>)),
    tag = "Countries"
)]
pub async fn list_countries(
    State(state): State<AppState>,
) -> ApiResult<Vec<V1CountrySummary>> {
    sqlx::query_as::<_, V1CountrySummary>(
        "SELECT
            c.iso_code                          AS code,
            c.name                              AS name,
            COUNT(DISTINCT a.id)::bigint        AS airport_count,
            AVG(s.score_total::float8)          AS avg_score,
            MAX(s.score_total::float8)          AS best_score,
            MIN(s.score_total::float8)          AS worst_score,
            (
                SELECT SUM(py.total_pax)::bigint
                FROM pax_yearly py
                WHERE py.airport_id = ANY(ARRAY_AGG(a.id))
                  AND py.year = (
                      SELECT MAX(py2.year)
                      FROM pax_yearly py2
                      WHERE py2.airport_id = py.airport_id
                  )
            )                                   AS total_pax,
            (
                SELECT AVG(ss.positive_pct::float8)
                FROM sentiment_snapshots ss
                WHERE ss.airport_id = ANY(ARRAY_AGG(a.id))
            )                                   AS avg_sentiment_positive,
            (
                SELECT AVG(100.0 - os.delay_pct::float8)
                FROM operational_stats os
                WHERE os.airport_id = ANY(ARRAY_AGG(a.id))
                  AND os.delay_pct IS NOT NULL
            )                                   AS avg_on_time,
            (
                SELECT COUNT(*)::bigint
                FROM routes ro
                WHERE ro.origin_id = ANY(ARRAY_AGG(a.id))
            )                                   AS total_routes,
            (
                SELECT AVG(aa.lat::float8)
                FROM all_airports aa
                WHERE aa.iata = ANY(ARRAY_AGG(a.iata_code))
            )                                   AS lat,
            (
                SELECT AVG(aa.lon::float8)
                FROM all_airports aa
                WHERE aa.iata = ANY(ARRAY_AGG(a.iata_code))
            )                                   AS lng
         FROM airports a
         INNER JOIN countries c ON c.iso_code = a.country_code
         LEFT JOIN airport_scores s ON s.airport_id = a.id AND s.is_latest = TRUE
         WHERE a.in_seed_set = TRUE
         GROUP BY c.iso_code, c.name
         ORDER BY avg_score DESC NULLS LAST",
    )
    .fetch_all(&state.pool)
    .await
    .map(Json)
    .map_err(|_| ApiError::internal())
}

#[utoipa::path(get, path = "/api/v1/countries/{code}/airports",
    summary = "Airports in a country",
    description = "All tracked airports in a country, ordered by score.",
    params(("code" = String, Path, description = "ISO 3166-1 alpha-2 country code (e.g. DE, GB, FR)")),
    responses((status = 200, description = "Airports in country", body = Vec<V1AirportListItem>)),
    tag = "Countries"
)]
pub async fn airports_by_country(
    State(state): State<AppState>,
    Path(code): Path<String>,
) -> ApiResult<Vec<V1AirportListItem>> {
    let code_upper = code.to_uppercase();

    sqlx::query_as::<_, V1AirportListItem>(
        "SELECT a.iata_code, a.name, a.city, a.country_code,
                s.score_total::float8 as score_total,
                s.score_sentiment_velocity::float8 as score_sentiment_velocity,
                (SELECT COUNT(*) FROM airport_awards aw WHERE aw.iata_code = a.iata_code) as award_count
         FROM airports a
         LEFT JOIN airport_scores s ON s.airport_id = a.id AND s.is_latest = TRUE
         WHERE a.country_code = $1 AND a.in_seed_set = TRUE
         ORDER BY s.score_total DESC NULLS LAST",
    )
    .bind(&code_upper)
    .fetch_all(&state.pool)
    .await
    .map(Json)
    .map_err(|_| ApiError::internal())
}
