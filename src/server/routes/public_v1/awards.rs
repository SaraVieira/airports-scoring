use axum::extract::{Query, State};
use axum::Json;

use crate::server::AppState;
use super::types::*;

#[utoipa::path(get, path = "/api/v1/awards",
    summary = "Browse awards",
    description = "All Skytrax World Airport Awards and ACI ASQ Awards. Filter by year, source, region, or airport IATA code. Skytrax data covers 2018-2026, ACI data covers 2006-2024.",
    params(
        ("year" = Option<i16>, Query, description = "Filter by year (e.g. 2024)"),
        ("source" = Option<String>, Query, description = "Filter by source: skytrax or aci_asq"),
        ("region" = Option<String>, Query, description = "Filter by region (e.g. europe, asia)"),
        ("iata" = Option<String>, Query, description = "Filter by airport IATA code (e.g. MUC)"),
    ),
    responses((status = 200, description = "Awards list", body = Vec<V1AwardWithAirport>)),
    tag = "Awards"
)]
pub async fn list_awards(
    State(state): State<AppState>,
    Query(filter): Query<AwardsFilter>,
) -> ApiResult<Vec<V1AwardWithAirport>> {
    // Dataset is small (<1000 rows) — fetch all and filter in Rust
    // to avoid dynamic SQL building.
    let all = sqlx::query_as::<_, V1AwardWithAirport>(
        "SELECT iata_code, source, year, category, region, size_bucket, rank
         FROM airport_awards ORDER BY year DESC, source, iata_code",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|_| ApiError::internal())?;

    let filtered: Vec<V1AwardWithAirport> = all
        .into_iter()
        .filter(|a| {
            if let Some(y) = filter.year {
                if a.year != y {
                    return false;
                }
            }
            if let Some(ref s) = filter.source {
                if a.source != *s {
                    return false;
                }
            }
            if let Some(ref r) = filter.region {
                if a.region.as_deref() != Some(r.as_str()) {
                    return false;
                }
            }
            if let Some(ref i) = filter.iata {
                if a.iata_code != i.to_uppercase() {
                    return false;
                }
            }
            true
        })
        .collect();

    Ok(Json(filtered))
}
