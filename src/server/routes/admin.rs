use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

use crate::models::{
    CreateSupportedAirport, SourceStatus, SupportedAirport, UpdateSupportedAirport,
};
use crate::server::jobs::{JobInfo, StartJobRequest};
use crate::server::AppState;

// ── Response types ──────────────────────────────────────────────

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SupportedAirportWithStatus {
    pub iata_code: String,
    pub country_code: String,
    pub name: String,
    pub skytrax_review_slug: Option<String>,
    pub skytrax_rating_slug: Option<String>,
    pub google_maps_url: Option<String>,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
    pub sources: Vec<SourceStatusResponse>,
    pub has_score: bool,
    /// "scored", "too_small", or "pending"
    pub score_status: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SourceStatusResponse {
    pub source: String,
    pub last_fetched_at: Option<String>,
    pub last_status: String,
    pub last_record_count: Option<i32>,
    pub last_error: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DataGapResponse {
    pub iata_code: String,
    pub name: String,
    pub source: String,
    pub last_fetched_at: Option<String>,
    pub last_status: String,
}

// ── Helpers ─────────────────────────────────────────────────────

fn airport_to_response(
    airport: &SupportedAirport,
    statuses: Vec<SourceStatus>,
    has_score: bool,
    route_count: i64,
) -> SupportedAirportWithStatus {
    let score_status = if has_score {
        "scored".to_string()
    } else if route_count < crate::scoring::MIN_ROUTES_FOR_SCORING {
        "too_small".to_string()
    } else {
        "pending".to_string()
    };

    SupportedAirportWithStatus {
        iata_code: airport.iata_code.clone(),
        country_code: airport.country_code.clone(),
        name: airport.name.clone(),
        skytrax_review_slug: airport.skytrax_review_slug.clone(),
        skytrax_rating_slug: airport.skytrax_rating_slug.clone(),
        google_maps_url: airport.google_maps_url.clone(),
        enabled: airport.enabled,
        created_at: airport.created_at.to_rfc3339(),
        updated_at: airport.updated_at.to_rfc3339(),
        sources: statuses
            .into_iter()
            .map(|s| SourceStatusResponse {
                source: s.source,
                last_fetched_at: s.last_fetched_at.map(|dt: DateTime<Utc>| dt.to_rfc3339()),
                last_status: s.last_status,
                last_record_count: s.last_record_count,
                last_error: s.last_error,
            })
            .collect(),
        has_score,
        score_status,
    }
}

// ── Handlers ────────────────────────────────────────────────────

/// List all supported airports with their source statuses.
#[utoipa::path(
    get,
    path = "/api/admin/supported-airports",
    responses(
        (status = 200, description = "All supported airports with source status", body = Vec<SupportedAirportWithStatus>),
    ),
    tag = "admin"
)]
pub async fn list_supported_airports(
    State(state): State<AppState>,
) -> Result<Json<Vec<SupportedAirportWithStatus>>, StatusCode> {
    let airports = sqlx::query_as::<sqlx::Postgres, SupportedAirport>(
        "SELECT * FROM supported_airports ORDER BY iata_code",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| { tracing::error!("admin query error: {e}"); StatusCode::INTERNAL_SERVER_ERROR })?;

    let statuses = sqlx::query_as::<sqlx::Postgres, SourceStatus>(
        "SELECT * FROM source_status ORDER BY iata_code, source",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| { tracing::error!("admin query error: {e}"); StatusCode::INTERNAL_SERVER_ERROR })?;

    // Group statuses by iata_code
    let mut status_map: std::collections::HashMap<String, Vec<SourceStatus>> =
        std::collections::HashMap::new();
    for s in statuses {
        status_map
            .entry(s.iata_code.clone())
            .or_default()
            .push(s);
    }

    // Check which airports have scores
    let scored_iatas: std::collections::HashSet<String> = sqlx::query_scalar::<_, String>(
        "SELECT a.iata_code FROM airports a \
         INNER JOIN airport_scores s ON s.airport_id = a.id AND s.is_latest = TRUE \
         WHERE a.in_seed_set = TRUE AND s.score_total IS NOT NULL",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .into_iter()
    .collect();

    // Route counts per airport (for score_status)
    let route_counts: Vec<(String, i64)> = sqlx::query_as(
        "SELECT a.iata_code, COUNT(r.id) \
         FROM airports a \
         LEFT JOIN routes r ON r.origin_id = a.id \
         WHERE a.in_seed_set = TRUE \
         GROUP BY a.iata_code",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let route_map: std::collections::HashMap<String, i64> =
        route_counts.into_iter().collect();

    let result: Vec<SupportedAirportWithStatus> = airports
        .iter()
        .map(|a| {
            let statuses = status_map.remove(&a.iata_code).unwrap_or_default();
            let has_score = scored_iatas.contains(&a.iata_code);
            let routes = route_map.get(&a.iata_code).copied().unwrap_or(0);
            airport_to_response(a, statuses, has_score, routes)
        })
        .collect();

    Ok(Json(result))
}

/// Create a new supported airport.
#[utoipa::path(
    post,
    path = "/api/admin/supported-airports",
    request_body = CreateSupportedAirport,
    responses(
        (status = 201, description = "Airport created", body = SupportedAirportWithStatus),
        (status = 409, description = "Airport already exists"),
    ),
    tag = "admin"
)]
pub async fn create_supported_airport(
    State(state): State<AppState>,
    Json(body): Json<CreateSupportedAirport>,
) -> Result<(StatusCode, Json<SupportedAirportWithStatus>), StatusCode> {
    let airport = sqlx::query_as::<sqlx::Postgres, SupportedAirport>(
        r#"
        INSERT INTO supported_airports (iata_code, country_code, name, skytrax_review_slug, skytrax_rating_slug, google_maps_url)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING *
        "#,
    )
    .bind(&body.iata_code)
    .bind(&body.country_code)
    .bind(&body.name)
    .bind(&body.skytrax_review_slug)
    .bind(&body.skytrax_rating_slug)
    .bind(&body.google_maps_url)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(ref db_err) if db_err.is_unique_violation() => StatusCode::CONFLICT,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    })?;

    let response = airport_to_response(&airport, vec![], false, 0);
    Ok((StatusCode::CREATED, Json(response)))
}

/// Update a supported airport (partial update).
#[utoipa::path(
    patch,
    path = "/api/admin/supported-airports/{iata}",
    params(("iata" = String, Path, description = "IATA airport code")),
    request_body = UpdateSupportedAirport,
    responses(
        (status = 200, description = "Airport updated", body = SupportedAirportWithStatus),
        (status = 404, description = "Airport not found"),
    ),
    tag = "admin"
)]
pub async fn update_supported_airport(
    State(state): State<AppState>,
    Path(iata): Path<String>,
    Json(body): Json<UpdateSupportedAirport>,
) -> Result<Json<SupportedAirportWithStatus>, StatusCode> {
    let airport = sqlx::query_as::<sqlx::Postgres, SupportedAirport>(
        r#"
        UPDATE supported_airports SET
            name = COALESCE($1, name),
            country_code = COALESCE($2, country_code),
            skytrax_review_slug = COALESCE($3, skytrax_review_slug),
            skytrax_rating_slug = COALESCE($4, skytrax_rating_slug),
            google_maps_url = COALESCE($5, google_maps_url),
            enabled = COALESCE($6, enabled),
            updated_at = now()
        WHERE iata_code = $7
        RETURNING *
        "#,
    )
    .bind(&body.name)
    .bind(&body.country_code)
    .bind(&body.skytrax_review_slug)
    .bind(&body.skytrax_rating_slug)
    .bind(&body.google_maps_url)
    .bind(&body.enabled)
    .bind(&iata)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    // Sync in_seed_set with enabled status
    if let Some(enabled) = body.enabled {
        sqlx::query("UPDATE airports SET in_seed_set = $1 WHERE iata_code = $2")
            .bind(enabled)
            .bind(&iata)
            .execute(&state.pool)
            .await
            .map_err(|e| { tracing::error!("admin query error: {e}"); StatusCode::INTERNAL_SERVER_ERROR })?;
    }

    let statuses = sqlx::query_as::<sqlx::Postgres, SourceStatus>(
        "SELECT * FROM source_status WHERE iata_code = $1 ORDER BY source",
    )
    .bind(&iata)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| { tracing::error!("admin query error: {e}"); StatusCode::INTERNAL_SERVER_ERROR })?;

    let has_score: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM airport_scores s \
         INNER JOIN airports a ON a.id = s.airport_id \
         WHERE a.iata_code = $1 AND s.is_latest = TRUE AND s.score_total IS NOT NULL)",
    )
    .bind(&iata)
    .fetch_one(&state.pool)
    .await
    .unwrap_or(false);

    let route_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM routes r \
         INNER JOIN airports a ON a.id = r.origin_id \
         WHERE a.iata_code = $1",
    )
    .bind(&iata)
    .fetch_one(&state.pool)
    .await
    .unwrap_or((0,));

    Ok(Json(airport_to_response(&airport, statuses, has_score, route_count.0)))
}

/// Delete a supported airport.
#[utoipa::path(
    delete,
    path = "/api/admin/supported-airports/{iata}",
    params(("iata" = String, Path, description = "IATA airport code")),
    responses(
        (status = 204, description = "Airport deleted"),
        (status = 404, description = "Airport not found"),
    ),
    tag = "admin"
)]
pub async fn delete_supported_airport(
    State(state): State<AppState>,
    Path(iata): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let result = sqlx::query("DELETE FROM supported_airports WHERE iata_code = $1")
        .bind(&iata)
        .execute(&state.pool)
        .await
        .map_err(|e| { tracing::error!("admin query error: {e}"); StatusCode::INTERNAL_SERVER_ERROR })?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    // Remove from public lists and operator aggregations
    sqlx::query("UPDATE airports SET in_seed_set = false WHERE iata_code = $1")
        .bind(&iata)
        .execute(&state.pool)
        .await
        .map_err(|e| { tracing::error!("admin query error: {e}"); StatusCode::INTERNAL_SERVER_ERROR })?;

    Ok(StatusCode::NO_CONTENT)
}

/// Find airports with missing or stale data sources.
#[utoipa::path(
    get,
    path = "/api/admin/data-gaps",
    responses(
        (status = 200, description = "Airports with missing or stale data", body = Vec<DataGapResponse>),
    ),
    tag = "admin"
)]
pub async fn data_gaps(
    State(state): State<AppState>,
) -> Result<Json<Vec<DataGapResponse>>, StatusCode> {
    // Stale, failed, or never-fetched source_status rows
    let stale = sqlx::query_as::<sqlx::Postgres, (String, String, String, Option<DateTime<Utc>>, String)>(
        r#"
        SELECT sa.iata_code, sa.name, ss.source, ss.last_fetched_at, ss.last_status
        FROM source_status ss
        JOIN supported_airports sa ON sa.iata_code = ss.iata_code
        WHERE sa.enabled = true
          AND ss.source != 'operator'
          AND (ss.last_fetched_at IS NULL
               OR ss.last_status = 'failed'
               OR ss.last_fetched_at < now() - interval '30 days')
        ORDER BY sa.iata_code, ss.source
        "#,
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| { tracing::error!("admin query error: {e}"); StatusCode::INTERNAL_SERVER_ERROR })?;

    // Find airports missing expected sources entirely (no source_status row for that source).
    // Cross-join enabled airports with the expected pipeline sources, then LEFT JOIN
    // to find combinations that have never been run.
    let never_fetched = sqlx::query_as::<sqlx::Postgres, (String, String, String)>(
        r#"
        SELECT sa.iata_code, sa.name, expected.source
        FROM supported_airports sa
        CROSS JOIN (
            VALUES ('eurocontrol'), ('metar'), ('opensky'), ('routes'),
                   ('eurostat'), ('caa'), ('aena'), ('ssb_norway'),
                   ('dst_denmark'), ('finavia'), ('wikipedia'),
                   ('reviews'), ('sentiment'), ('carbon_accreditation'), ('priority_pass')
        ) AS expected(source)
        LEFT JOIN source_status ss
            ON ss.iata_code = sa.iata_code AND ss.source = expected.source
        WHERE sa.enabled = true
          AND ss.iata_code IS NULL
        ORDER BY sa.iata_code, expected.source
        "#,
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| { tracing::error!("admin query error: {e}"); StatusCode::INTERNAL_SERVER_ERROR })?;

    let mut results: Vec<DataGapResponse> = stale
        .into_iter()
        .map(|(iata_code, name, source, last_fetched_at, last_status)| DataGapResponse {
            iata_code,
            name,
            source,
            last_fetched_at: last_fetched_at.map(|dt| dt.to_rfc3339()),
            last_status,
        })
        .collect();

    for (iata_code, name, source) in never_fetched {
        results.push(DataGapResponse {
            iata_code,
            name,
            source,
            last_fetched_at: None,
            last_status: "never_fetched".to_string(),
        });
    }

    // Airports with no operator assigned
    let no_operator = sqlx::query_as::<sqlx::Postgres, (String, String)>(
        r#"
        SELECT sa.iata_code, sa.name
        FROM supported_airports sa
        JOIN airports a ON a.iata_code = sa.iata_code
        WHERE sa.enabled = true
          AND a.operator_id IS NULL
        ORDER BY sa.iata_code
        "#,
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| { tracing::error!("admin query error: {e}"); StatusCode::INTERNAL_SERVER_ERROR })?;

    for (iata_code, name) in no_operator {
        results.push(DataGapResponse {
            iata_code,
            name,
            source: "operator".to_string(),
            last_fetched_at: None,
            last_status: "missing".to_string(),
        });
    }

    // Airports with reviews but no sentiment snapshots for that source
    // Airports with any reviews lacking sentiment snapshots — collapsed per airport
    // regardless of source (google/skytrax), since the sentiment pipeline handles both.
    let unprocessed_reviews = sqlx::query_as::<sqlx::Postgres, (String, String, i64)>(
        r#"
        SELECT sa.iata_code, sa.name, COUNT(*) as review_count
        FROM reviews_raw r
        JOIN airports a ON a.id = r.airport_id
        JOIN supported_airports sa ON sa.iata_code = a.iata_code
        LEFT JOIN sentiment_snapshots ss
            ON ss.airport_id = a.id AND ss.source = r.source
        WHERE sa.enabled = true
          AND ss.id IS NULL
        GROUP BY sa.iata_code, sa.name
        HAVING COUNT(*) > 0
        ORDER BY COUNT(*) DESC
        "#,
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| { tracing::error!("admin query error: {e}"); StatusCode::INTERNAL_SERVER_ERROR })?;

    for (iata_code, name, count) in unprocessed_reviews {
        results.push(DataGapResponse {
            iata_code,
            name,
            source: "sentiment".to_string(),
            last_fetched_at: None,
            last_status: format!("{count} reviews unprocessed"),
        });
    }

    Ok(Json(results))
}

// ── Job endpoints ──────────────────────────────────────────────

/// Start a pipeline job.
#[utoipa::path(
    post,
    path = "/api/admin/jobs",
    request_body = StartJobRequest,
    responses(
        (status = 202, description = "Job started", body = JobInfo),
        (status = 500, description = "Failed to start job"),
    ),
    tag = "admin"
)]
pub async fn start_job(
    State(state): State<AppState>,
    Json(body): Json<StartJobRequest>,
) -> Result<(StatusCode, Json<JobInfo>), StatusCode> {
    let job = state
        .jobs
        .start_job(body)
        .await
        .map_err(|e| { tracing::error!("admin query error: {e}"); StatusCode::INTERNAL_SERVER_ERROR })?;
    Ok((StatusCode::ACCEPTED, Json(job)))
}

/// List all jobs.
#[utoipa::path(
    get,
    path = "/api/admin/jobs",
    responses(
        (status = 200, description = "All jobs", body = Vec<JobInfo>),
    ),
    tag = "admin"
)]
pub async fn list_jobs(State(state): State<AppState>) -> Json<Vec<JobInfo>> {
    Json(state.jobs.list_jobs().await)
}

/// Get a single job by ID.
#[utoipa::path(
    get,
    path = "/api/admin/jobs/{id}",
    params(("id" = String, Path, description = "Job ID")),
    responses(
        (status = 200, description = "Job details", body = JobInfo),
        (status = 404, description = "Job not found"),
    ),
    tag = "admin"
)]
pub async fn get_job(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<JobInfo>, StatusCode> {
    state
        .jobs
        .get_job(&id)
        .await
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

/// Cancel a running job.
#[utoipa::path(
    post,
    path = "/api/admin/jobs/{id}/cancel",
    params(("id" = String, Path, description = "Job ID")),
    responses(
        (status = 202, description = "Cancellation requested"),
        (status = 404, description = "Job not found"),
    ),
    tag = "admin"
)]
pub async fn cancel_job(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> StatusCode {
    if state.jobs.cancel_job(&id).await {
        StatusCode::ACCEPTED
    } else {
        StatusCode::NOT_FOUND
    }
}

/// Incremental refresh all enabled airports (all sources, no full refresh).
#[utoipa::path(
    post,
    path = "/api/admin/refresh",
    responses(
        (status = 202, description = "Refresh job started", body = JobInfo),
        (status = 500, description = "Failed to start refresh"),
    ),
    tag = "admin"
)]
pub async fn refresh_all(
    State(state): State<AppState>,
) -> Result<(StatusCode, Json<JobInfo>), StatusCode> {
    let request = StartJobRequest {
        airports: None,
        sources: None,
        full_refresh: Some(false),
        score: Some(true),
    };
    let job = state
        .jobs
        .start_job(request)
        .await
        .map_err(|e| { tracing::error!("admin query error: {e}"); StatusCode::INTERNAL_SERVER_ERROR })?;
    Ok((StatusCode::ACCEPTED, Json(job)))
}

/// Trigger scoring only (no data fetching).
#[utoipa::path(
    post,
    path = "/api/admin/score",
    responses(
        (status = 200, description = "Scoring completed"),
        (status = 500, description = "Scoring failed"),
    ),
    tag = "admin"
)]
pub async fn trigger_scoring(State(state): State<AppState>) -> StatusCode {
    match state.jobs.trigger_scoring().await {
        Ok(()) => StatusCode::OK,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

// ── Batch import ────────────────────────────────────────────────

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BatchImportRequest {
    pub iata_codes: Vec<String>,
    #[serde(default)]
    pub run_pipeline: bool,
    #[serde(default)]
    pub score: bool,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BatchResolvedAirport {
    pub iata_code: String,
    pub name: String,
    pub country_code: String,
    pub icao_code: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BatchImportResponse {
    pub resolved: Vec<BatchResolvedAirport>,
    pub failed: Vec<String>,
    pub job_id: Option<String>,
}

/// Row type for reading from the all_airports table.
#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct AllAirportRow {
    pub iata: Option<String>,
    pub icao: String,
    pub name: String,
    pub city: String,
    pub country: String,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
}

/// Batch import airports by IATA codes, resolving from all_airports.
#[utoipa::path(
    post,
    path = "/api/admin/batch-import",
    request_body = BatchImportRequest,
    responses(
        (status = 200, description = "Batch import results", body = BatchImportResponse),
        (status = 500, description = "Internal error"),
    ),
    tag = "admin"
)]
pub async fn batch_import(
    State(state): State<AppState>,
    Json(body): Json<BatchImportRequest>,
) -> Result<Json<BatchImportResponse>, StatusCode> {
    let upper_codes: Vec<String> = body.iata_codes.iter().map(|c| c.to_uppercase()).collect();

    // 1) Single SELECT to resolve all IATA codes at once.
    let rows = sqlx::query_as::<sqlx::Postgres, AllAirportRow>(
        "SELECT iata, icao, name, city, country, lat, lon FROM all_airports WHERE iata = ANY($1)",
    )
    .bind(&upper_codes)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| { tracing::error!("admin query error: {e}"); StatusCode::INTERNAL_SERVER_ERROR })?;

    // Partition into resolved vs failed.
    let found_set: std::collections::HashSet<String> = rows
        .iter()
        .filter_map(|r| r.iata.clone())
        .collect();

    let failed: Vec<String> = upper_codes
        .iter()
        .filter(|c| !found_set.contains(c.as_str()))
        .cloned()
        .collect();

    let resolved: Vec<BatchResolvedAirport> = rows
        .iter()
        .filter_map(|r| {
            let iata = r.iata.as_ref()?;
            Some(BatchResolvedAirport {
                iata_code: iata.clone(),
                name: r.name.clone(),
                country_code: r.country.clone(),
                icao_code: r.icao.clone(),
            })
        })
        .collect();

    if !resolved.is_empty() {
        // Prepare column vectors for batch inserts.
        let iata_codes: Vec<&str> = resolved.iter().map(|r| r.iata_code.as_str()).collect();
        let names: Vec<&str> = resolved.iter().map(|r| r.name.as_str()).collect();
        let country_codes: Vec<&str> = resolved.iter().map(|r| r.country_code.as_str()).collect();
        let icao_codes: Vec<&str> = resolved.iter().map(|r| r.icao_code.as_str()).collect();
        let cities: Vec<&str> = rows
            .iter()
            .filter(|r| r.iata.is_some())
            .map(|r| r.city.as_str())
            .collect();
        let lats: Vec<f64> = rows
            .iter()
            .filter(|r| r.iata.is_some())
            .map(|r| r.lat.unwrap_or(0.0))
            .collect();
        let lons: Vec<f64> = rows
            .iter()
            .filter(|r| r.iata.is_some())
            .map(|r| r.lon.unwrap_or(0.0))
            .collect();

        // 2) & 3) Batch inserts inside a transaction.
        let mut tx = state
            .pool
            .begin()
            .await
            .map_err(|e| { tracing::error!("admin query error: {e}"); StatusCode::INTERNAL_SERVER_ERROR })?;

        sqlx::query(
            r#"
            INSERT INTO supported_airports (iata_code, country_code, name)
            SELECT * FROM UNNEST($1::text[], $2::text[], $3::text[])
            ON CONFLICT (iata_code) DO NOTHING
            "#,
        )
        .bind(&iata_codes)
        .bind(&country_codes)
        .bind(&names)
        .execute(&mut *tx)
        .await
        .map_err(|e| { tracing::error!("admin query error: {e}"); StatusCode::INTERNAL_SERVER_ERROR })?;

        sqlx::query(
            r#"
            INSERT INTO airports (iata_code, icao_code, name, city, country_code, airport_type, in_seed_set, location)
            SELECT i, ic, n, ci, cc, 'large_airport', true, ST_SetSRID(ST_MakePoint(lo, la), 4326)
            FROM UNNEST($1::text[], $2::text[], $3::text[], $4::text[], $5::text[], $6::float8[], $7::float8[])
              AS t(i, ic, n, ci, cc, la, lo)
            ON CONFLICT (iata_code) DO UPDATE SET in_seed_set = true
            "#,
        )
        .bind(&iata_codes)
        .bind(&icao_codes)
        .bind(&names)
        .bind(&cities)
        .bind(&country_codes)
        .bind(&lats)
        .bind(&lons)
        .execute(&mut *tx)
        .await
        .map_err(|e| { tracing::error!("admin query error: {e}"); StatusCode::INTERNAL_SERVER_ERROR })?;

        tx.commit()
            .await
            .map_err(|e| { tracing::error!("admin query error: {e}"); StatusCode::INTERNAL_SERVER_ERROR })?;
    }

    // Optionally start a pipeline job for the resolved airports.
    let job_id = if body.run_pipeline && !resolved.is_empty() {
        let request = StartJobRequest {
            airports: Some(resolved.iter().map(|r| r.iata_code.clone()).collect()),
            sources: None,
            full_refresh: Some(false),
            score: Some(body.score),
        };
        match state.jobs.start_job(request).await {
            Ok(job) => Some(job.id),
            Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
        }
    } else {
        None
    };

    Ok(Json(BatchImportResponse {
        resolved,
        failed,
        job_id,
    }))
}

// ── Operator CRUD ───────────────────────────────────────────────

#[derive(Debug, Serialize, FromRow, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AdminOperatorItem {
    pub id: i32,
    pub name: String,
    pub short_name: Option<String>,
    pub country_code: Option<String>,
    pub org_type: String,
    pub ownership_model: Option<String>,
    pub public_share_pct: Option<f64>,
    pub notes: Option<String>,
    pub airport_count: i64,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateOperatorRequest {
    pub name: Option<String>,
    pub short_name: Option<String>,
    pub country_code: Option<String>,
    pub org_type: Option<String>,
    pub ownership_model: Option<String>,
    pub public_share_pct: Option<f64>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateOperatorRequest {
    pub name: String,
    pub short_name: Option<String>,
    pub country_code: Option<String>,
    pub org_type: String,
    pub ownership_model: Option<String>,
    pub public_share_pct: Option<f64>,
    pub notes: Option<String>,
    pub iata_codes: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SetOperatorAirportsRequest {
    pub iata_codes: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateOperatorResponse {
    pub id: i32,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SetOperatorAirportsResponse {
    pub mapped_count: u64,
}

/// List all operators with their airport mappings.
#[utoipa::path(
    get,
    path = "/api/admin/operators",
    responses(
        (status = 200, description = "All operators with airport counts", body = Vec<AdminOperatorItem>),
    ),
    tag = "admin"
)]
pub async fn list_admin_operators(
    State(state): State<AppState>,
) -> Result<Json<Vec<AdminOperatorItem>>, StatusCode> {
    let operators = sqlx::query_as::<sqlx::Postgres, AdminOperatorItem>(
        r#"
        SELECT o.id, o.name, o.short_name, o.country_code, o.org_type,
               o.ownership_model, o.public_share_pct::float8 as public_share_pct, o.notes,
               (SELECT COUNT(*) FROM airports a WHERE a.operator_id = o.id)::int8 as airport_count
        FROM organisations o
        ORDER BY o.name
        "#,
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("list_admin_operators error: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(operators))
}

/// Update an operator's details.
#[utoipa::path(
    put,
    path = "/api/admin/operators/{id}",
    params(("id" = i32, Path, description = "Operator ID")),
    request_body = UpdateOperatorRequest,
    responses(
        (status = 200, description = "Operator updated"),
        (status = 404, description = "Operator not found"),
    ),
    tag = "admin"
)]
pub async fn update_operator(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(body): Json<UpdateOperatorRequest>,
) -> Result<StatusCode, StatusCode> {
    let result = sqlx::query(
        r#"
        UPDATE organisations SET
            name = COALESCE($2, name),
            short_name = COALESCE($3, short_name),
            country_code = COALESCE($4, country_code),
            org_type = COALESCE($5, org_type),
            ownership_model = COALESCE($6, ownership_model),
            public_share_pct = COALESCE($7, public_share_pct),
            notes = COALESCE($8, notes)
        WHERE id = $1
        "#,
    )
    .bind(id)
    .bind(&body.name)
    .bind(&body.short_name)
    .bind(&body.country_code)
    .bind(&body.org_type)
    .bind(&body.ownership_model)
    .bind(&body.public_share_pct)
    .bind(&body.notes)
    .execute(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("update_operator error: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(StatusCode::OK)
}

/// Set the airport mappings for an operator.
#[utoipa::path(
    post,
    path = "/api/admin/operators/{id}/airports",
    params(("id" = i32, Path, description = "Operator ID")),
    request_body = SetOperatorAirportsRequest,
    responses(
        (status = 200, description = "Airports mapped", body = SetOperatorAirportsResponse),
    ),
    tag = "admin"
)]
pub async fn set_operator_airports(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(body): Json<SetOperatorAirportsRequest>,
) -> Result<Json<SetOperatorAirportsResponse>, StatusCode> {
    let mut tx = state.pool.begin().await.map_err(|e| {
        tracing::error!("set_operator_airports tx error: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Clear existing mappings
    sqlx::query("UPDATE airports SET operator_id = NULL WHERE operator_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!("set_operator_airports clear error: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Set new mappings
    let result =
        sqlx::query("UPDATE airports SET operator_id = $1 WHERE iata_code = ANY($2)")
            .bind(id)
            .bind(&body.iata_codes)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                tracing::error!("set_operator_airports set error: {e}");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

    tx.commit().await.map_err(|e| {
        tracing::error!("set_operator_airports commit error: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(SetOperatorAirportsResponse {
        mapped_count: result.rows_affected(),
    }))
}

/// Delete an operator.
#[utoipa::path(
    delete,
    path = "/api/admin/operators/{id}",
    params(("id" = i32, Path, description = "Operator ID")),
    responses(
        (status = 204, description = "Operator deleted"),
        (status = 404, description = "Operator not found"),
    ),
    tag = "admin"
)]
pub async fn delete_operator(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<StatusCode, StatusCode> {
    let mut tx = state.pool.begin().await.map_err(|e| {
        tracing::error!("delete_operator tx error: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Clear airport mappings first
    sqlx::query("UPDATE airports SET operator_id = NULL WHERE operator_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!("delete_operator clear error: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let result = sqlx::query("DELETE FROM organisations WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!("delete_operator delete error: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    tx.commit().await.map_err(|e| {
        tracing::error!("delete_operator commit error: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(StatusCode::NO_CONTENT)
}

/// Create a new operator.
#[utoipa::path(
    post,
    path = "/api/admin/operators",
    request_body = CreateOperatorRequest,
    responses(
        (status = 201, description = "Operator created", body = CreateOperatorResponse),
        (status = 500, description = "Internal error"),
    ),
    tag = "admin"
)]
pub async fn create_operator(
    State(state): State<AppState>,
    Json(body): Json<CreateOperatorRequest>,
) -> Result<(StatusCode, Json<CreateOperatorResponse>), StatusCode> {
    let mut tx = state.pool.begin().await.map_err(|e| {
        tracing::error!("create_operator tx error: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let id: i32 = sqlx::query_scalar(
        r#"
        INSERT INTO organisations (name, short_name, country_code, org_type, ownership_model, public_share_pct, notes)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id
        "#,
    )
    .bind(&body.name)
    .bind(&body.short_name)
    .bind(&body.country_code)
    .bind(&body.org_type)
    .bind(&body.ownership_model)
    .bind(&body.public_share_pct)
    .bind(&body.notes)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!("create_operator insert error: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Map airports if iata_codes provided
    if let Some(ref iata_codes) = body.iata_codes {
        if !iata_codes.is_empty() {
            sqlx::query("UPDATE airports SET operator_id = $1 WHERE iata_code = ANY($2)")
                .bind(id)
                .bind(iata_codes)
                .execute(&mut *tx)
                .await
                .map_err(|e| {
                    tracing::error!("create_operator map airports error: {e}");
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;
        }
    }

    tx.commit().await.map_err(|e| {
        tracing::error!("create_operator commit error: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok((StatusCode::CREATED, Json(CreateOperatorResponse { id })))
}

/// Get the list of IATA codes currently mapped to an operator.
#[utoipa::path(
    get,
    path = "/api/admin/operators/{id}/airports",
    params(("id" = i32, Path, description = "Operator ID")),
    responses(
        (status = 200, description = "List of mapped IATA codes", body = Vec<String>),
    ),
    tag = "admin"
)]
pub async fn get_operator_airports(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<Vec<String>>, StatusCode> {
    let codes = sqlx::query_scalar::<_, String>(
        "SELECT iata_code FROM airports WHERE operator_id = $1 AND iata_code IS NOT NULL ORDER BY iata_code",
    )
    .bind(id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("get_operator_airports error: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(codes))
}
