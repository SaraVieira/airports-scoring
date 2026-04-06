use axum::{extract::State, http::StatusCode, Json};

use crate::fetchers::eurocontrol::sync::run_sync;
use crate::server::jobs::{JobInfo, JobProgress, StartJobRequest};
use crate::server::AppState;

fn error_job(e: String) -> JobInfo {
    JobInfo {
        id: String::new(),
        airports: vec![],
        sources: vec![],
        full_refresh: false,
        score: false,
        status: "failed".to_string(),
        started_at: None,
        completed_at: None,
        progress: JobProgress {
            airports_completed: 0,
            airports_total: 0,
            current_airport: None,
            current_source: None,
        },
        error: Some(e),
    }
}

/// Cron: full refresh of all airports, all sources, then score.
#[utoipa::path(
    post,
    path = "/api/cron/full-refresh",
    responses(
        (status = 201, description = "Job started", body = JobInfo),
    ),
    tag = "cron"
)]
pub async fn cron_full_refresh(State(state): State<AppState>) -> (StatusCode, Json<JobInfo>) {
    let req = StartJobRequest {
        airports: None,
        sources: None,
        full_refresh: Some(true),
        score: Some(true),
    };
    match state.jobs.start_job(req).await {
        Ok(job) => (StatusCode::CREATED, Json(job)),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(error_job(e))),
    }
}

/// Cron: run sentiment pipeline + scoring only.
#[utoipa::path(
    post,
    path = "/api/cron/sentiment",
    responses(
        (status = 201, description = "Job started", body = JobInfo),
    ),
    tag = "cron"
)]
pub async fn cron_sentiment(State(state): State<AppState>) -> (StatusCode, Json<JobInfo>) {
    let req = StartJobRequest {
        airports: None,
        sources: Some(vec!["sentiment".to_string()]),
        full_refresh: Some(false),
        score: Some(true),
    };
    match state.jobs.start_job(req).await {
        Ok(job) => (StatusCode::CREATED, Json(job)),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(error_job(e))),
    }
}

/// Cron: run reviews (Skytrax + Google) only.
#[utoipa::path(
    post,
    path = "/api/cron/reviews",
    responses(
        (status = 201, description = "Job started", body = JobInfo),
    ),
    tag = "cron"
)]
pub async fn cron_reviews(State(state): State<AppState>) -> (StatusCode, Json<JobInfo>) {
    let req = StartJobRequest {
        airports: None,
        sources: Some(vec!["reviews".to_string()]),
        full_refresh: Some(false),
        score: Some(false),
    };
    match state.jobs.start_job(req).await {
        Ok(job) => (StatusCode::CREATED, Json(job)),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(error_job(e))),
    }
}

/// Cron: sync Eurocontrol datasets into local database cache.
/// Downloads all remote CSVs and ingests local apt_dly files.
#[utoipa::path(
    post,
    path = "/api/cron/sync-eurocontrol",
    responses(
        (status = 200, description = "Sync completed"),
        (status = 500, description = "Sync failed"),
    ),
    tag = "cron"
)]
pub async fn cron_sync_eurocontrol(
    State(state): State<AppState>,
) -> (StatusCode, Json<serde_json::Value>) {
    match run_sync(&state.pool, false).await {
        Ok(result) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "datasets_synced": result.datasets_synced,
                "total_rows": result.total_rows,
                "errors": result.errors,
            })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        ),
    }
}
