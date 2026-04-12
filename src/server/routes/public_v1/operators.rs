use axum::extract::{Path, State};
use axum::Json;

use crate::server::AppState;
use super::types::*;

#[utoipa::path(get, path = "/api/v1/operators",
    summary = "List operators",
    description = "All airport operators with aggregate stats: airport count, average score, total passengers, and average delay. Use the slug field to fetch operator details.",
    responses((status = 200, description = "Operator list", body = Vec<V1OperatorListItem>)),
    tag = "Operators"
)]
pub async fn list_operators(
    State(state): State<AppState>,
) -> ApiResult<Vec<V1OperatorListItem>> {
    sqlx::query_as::<_, V1OperatorListItem>(
        "SELECT o.slug, o.id, o.name, o.short_name, o.country_code, o.org_type,
                o.ownership_model, o.public_share_pct::float8 as public_share_pct,
                COUNT(DISTINCT a.id)::int8 as airport_count,
                AVG(s.score_total)::float8 as avg_score,
                (SELECT SUM(p.total_pax)::int8
                 FROM pax_yearly p
                 INNER JOIN airports a2 ON a2.id = p.airport_id
                 WHERE a2.operator_id = o.id
                   AND p.year = (SELECT MAX(p2.year) FROM pax_yearly p2
                                 WHERE p2.airport_id = p.airport_id AND p2.total_pax IS NOT NULL)
                ) as total_pax,
                (SELECT AVG(os.delay_pct)::float8
                 FROM operational_stats os
                 INNER JOIN airports a3 ON a3.id = os.airport_id
                 WHERE a3.operator_id = o.id
                   AND os.delay_pct IS NOT NULL
                   AND os.period_year >= EXTRACT(YEAR FROM NOW())::int - 1
                ) as avg_delay_pct
         FROM organisations o
         INNER JOIN airports a ON a.operator_id = o.id AND a.in_seed_set = true
         LEFT JOIN airport_scores s ON s.airport_id = a.id AND s.is_latest = true
         GROUP BY o.id, o.slug, o.name, o.short_name, o.country_code, o.org_type, o.ownership_model, o.public_share_pct
         HAVING COUNT(DISTINCT a.id) > 0
         ORDER BY avg_score DESC NULLS LAST",
    )
    .fetch_all(&state.pool)
    .await
    .map(Json)
    .map_err(|_| ApiError::internal())
}

#[derive(sqlx::FromRow)]
struct OperatorOrgRow {
    id: i32,
    name: String,
    short_name: Option<String>,
    country_code: Option<String>,
    org_type: String,
    ownership_model: Option<String>,
    public_share_pct: Option<f64>,
    notes: Option<String>,
}

#[utoipa::path(get, path = "/api/v1/operators/{slug}",
    summary = "Operator details",
    description = "Full operator profile with ownership info, notes, and all their airports with scores, delays, and passenger data.",
    params(("slug" = String, Path, description = "Operator slug (e.g. fraport-ag, aena-sme-sa)")),
    responses(
        (status = 200, description = "Operator detail", body = V1OperatorDetail),
        (status = 404, description = "Operator not found", body = ApiError),
    ),
    tag = "Operators"
)]
pub async fn get_operator(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> ApiResult<V1OperatorDetail> {
    let pool = &state.pool;

    let org = sqlx::query_as::<_, OperatorOrgRow>(
        "SELECT id, name, short_name, country_code, org_type, ownership_model,
                public_share_pct::float8 as public_share_pct, notes
         FROM organisations WHERE slug = $1",
    )
    .bind(&slug)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal())?
    .ok_or_else(|| ApiError::not_found("Operator not found"))?;

    let airports = sqlx::query_as::<_, V1OperatorAirport>(
        "SELECT a.iata_code, a.name, a.city, a.country_code,
                s.score_total::float8 as score_total,
                (SELECT AVG(os.delay_pct)::float8 FROM operational_stats os
                 WHERE os.airport_id = a.id AND os.delay_pct IS NOT NULL
                 AND os.period_year >= EXTRACT(YEAR FROM NOW())::int - 1) as avg_delay_pct,
                (SELECT p.total_pax FROM pax_yearly p
                 WHERE p.airport_id = a.id AND p.total_pax IS NOT NULL
                 ORDER BY p.year DESC LIMIT 1) as latest_pax
         FROM airports a
         LEFT JOIN airport_scores s ON s.airport_id = a.id AND s.is_latest = true
         WHERE a.operator_id = $1 AND a.in_seed_set = true
         ORDER BY s.score_total DESC NULLS LAST",
    )
    .bind(org.id)
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::internal())?;

    Ok(Json(V1OperatorDetail {
        name: org.name,
        short_name: org.short_name,
        country_code: org.country_code,
        org_type: Some(org.org_type),
        ownership_model: org.ownership_model,
        public_share_pct: org.public_share_pct,
        notes: org.notes,
        airports,
    }))
}
