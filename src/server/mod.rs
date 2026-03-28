mod auth;
pub mod jobs;
pub mod logs;
mod routes;

use std::sync::Arc;

use anyhow::{Context, Result};
use axum::{middleware, routing::{get, patch, post, put}, Json, Router};
use sqlx::PgPool;
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::info;
use utoipa::OpenApi;

use jobs::JobManager;
use logs::LogEntry;

/// Shared application state available to all handlers via `State<AppState>`.
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub jobs: Arc<JobManager>,
    pub log_sender: broadcast::Sender<LogEntry>,
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}

#[derive(OpenApi)]
#[openapi(
    info(title = "Airport Intelligence API", version = "1.0.0"),
    tags(
        (name = "airports", description = "Airport data endpoints"),
        (name = "operators", description = "Operator data endpoints"),
        (name = "admin", description = "Admin management endpoints"),
        (name = "cron", description = "Cron job triggers"),
    ),
    paths(
        routes::airports::get_airport,
        routes::airports::list_airports,
        routes::airports::search_airports,
        routes::airports::get_rankings,
        routes::airports::get_delay_rankings,
        routes::airports::airports_by_country,
        routes::airports::list_countries,
        routes::airports::get_busiest,
        routes::airports::get_best_reviewed,
        routes::airports::get_most_connected,
        routes::airports::get_map_airports,
        routes::airports::list_operators,
        routes::airports::get_operator,
        routes::admin::list_supported_airports,
        routes::admin::create_supported_airport,
        routes::admin::update_supported_airport,
        routes::admin::delete_supported_airport,
        routes::admin::data_gaps,
        routes::admin::start_job,
        routes::admin::list_jobs,
        routes::admin::get_job,
        routes::admin::cancel_job,
        routes::admin::refresh_all,
        routes::admin::trigger_scoring,
        routes::admin::batch_import,
        routes::admin::list_admin_operators,
        routes::admin::update_operator,
        routes::admin::set_operator_airports,
        routes::admin::delete_operator,
        routes::admin::create_operator,
        routes::admin::get_operator_airports,
        routes::cron::cron_full_refresh,
        routes::cron::cron_sentiment,
        routes::cron::cron_reviews,
    ),
)]
struct ApiDoc;

async fn openapi_spec() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

/// Start the API server on the given port.
pub async fn run(port: u16, log_sender: broadcast::Sender<LogEntry>) -> Result<()> {
    let pool = crate::db::get_pool().await?;

    // Run SQL migrations on startup.
    info!("Running database migrations...");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .context("Failed to run database migrations")?;
    info!("Migrations complete");

    let jobs = Arc::new(JobManager::new(pool.clone(), 3));
    let state = AppState { pool, jobs, log_sender };

    // Admin routes: require both API key and admin password.
    let admin_routes = Router::new()
        .route(
            "/supported-airports",
            get(routes::admin::list_supported_airports)
                .post(routes::admin::create_supported_airport),
        )
        .route(
            "/supported-airports/{iata}",
            patch(routes::admin::update_supported_airport)
                .delete(routes::admin::delete_supported_airport),
        )
        .route("/data-gaps", get(routes::admin::data_gaps))
        .route(
            "/jobs",
            get(routes::admin::list_jobs).post(routes::admin::start_job),
        )
        .route("/jobs/{id}", get(routes::admin::get_job))
        .route("/jobs/{id}/cancel", post(routes::admin::cancel_job))
        .route("/refresh", post(routes::admin::refresh_all))
        .route("/score", post(routes::admin::trigger_scoring))
        .route("/batch-import", post(routes::admin::batch_import))
        .route(
            "/operators",
            get(routes::admin::list_admin_operators)
                .post(routes::admin::create_operator),
        )
        .route(
            "/operators/{id}",
            put(routes::admin::update_operator)
                .delete(routes::admin::delete_operator),
        )
        .route(
            "/operators/{id}/airports",
            get(routes::admin::get_operator_airports)
                .post(routes::admin::set_operator_airports),
        )
        .layer(middleware::from_fn(auth::require_admin));

    // SSE logs route — no middleware since EventSource can't send headers.
    // Auth is checked via query param (admin password) inside the handler.
    let logs_route = Router::new()
        .route("/logs/stream", get(logs::stream_logs));

    // Cron routes: API key only (called by Coolify scheduler).
    let cron_routes = Router::new()
        .route("/full-refresh", post(routes::cron::cron_full_refresh))
        .route("/sentiment", post(routes::cron::cron_sentiment))
        .route("/reviews", post(routes::cron::cron_reviews));

    // Public API routes: require API key only.
    let api_routes = Router::new()
        .route("/airports", get(routes::airports::list_airports))
        .route("/airports/search", get(routes::airports::search_airports))
        .route("/airports/rankings", get(routes::airports::get_rankings))
        .route("/airports/delays", get(routes::airports::get_delay_rankings))
        .route("/airports/busiest", get(routes::airports::get_busiest))
        .route("/airports/best-reviewed", get(routes::airports::get_best_reviewed))
        .route("/airports/most-connected", get(routes::airports::get_most_connected))
        .route("/airports/map", get(routes::airports::get_map_airports))
        .route("/airports/{iata}", get(routes::airports::get_airport))
        .route("/countries", get(routes::airports::list_countries))
        .route(
            "/countries/{code}/airports",
            get(routes::airports::airports_by_country),
        )
        .route("/operators", get(routes::airports::list_operators))
        .route("/operators/{id}", get(routes::airports::get_operator))
        .nest("/admin", admin_routes)
        .nest("/cron", cron_routes)
        .layer(middleware::from_fn(auth::require_api_key));

    let app = Router::new()
        // Health check sits outside all auth middleware (for load balancers).
        .route("/health", get(health))
        // OpenAPI spec endpoint (no auth).
        .route("/openapi.json", get(openapi_spec))
        // SSE logs — outside API key middleware since EventSource can't send headers.
        .nest("/api/admin", logs_route)
        // All API routes nested under /api.
        .nest("/api", api_routes)
        .with_state(state)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
    info!(port, "API server listening");
    axum::serve(listener, app).await?;

    Ok(())
}
