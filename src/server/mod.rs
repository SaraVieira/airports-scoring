mod auth;
pub mod jobs;
pub mod logs;
mod routes;

use std::sync::Arc;

use anyhow::{Context, Result};
use axum::{middleware, routing::{get, patch, post, put}, Json, Router};
use sqlx::PgPool;
use tokio::sync::broadcast;
use tower_governor::GovernorLayer;
use tower_governor::governor::GovernorConfigBuilder;
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
        routes::cron::cron_sync_eurocontrol,
        routes::live::get_live_pulse,
    ),
)]
struct ApiDoc;

async fn openapi_spec() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

#[derive(OpenApi)]
#[openapi(
    info(title = "Airport Intelligence Public API", version = "1.0.0",
         description = "Free public API for European airport intelligence — scores, rankings, sentiment analysis, operational performance, routes, awards, and operator data.\n\n**No authentication required.** Rate limited to 100 requests/hour per IP (burst of 10).\n\n**Identifiers:** All airports are referenced by IATA code (e.g. MUC, LHR, CDG). Operators use URL-friendly slugs.\n\n**Scores:** Composite scores (0-100) are computed from infrastructure, operational performance, sentiment, connectivity, and operator quality. Higher is better.\n\n**Sentiment:** Ratings are on a 1-10 scale, aggregated from Skytrax and Google reviews. Sub-scores (queuing, cleanliness, staff, etc.) are on a 1-5 scale.\n\n**Data coverage:** ~150 European airports with scores. Operational data from Eurocontrol. Passenger data from national aviation authorities. Awards from Skytrax and ACI ASQ."),
    servers(
        (url = "https://api.airports.report", description = "Production"),
    ),
    tags(
        (name = "Airports", description = "Airport data, scores, rankings, and search"),
        (name = "Awards", description = "Skytrax and ACI ASQ airport awards"),
        (name = "Countries", description = "Country-level aggregated statistics"),
        (name = "Operators", description = "Airport operators and their portfolios"),
    ),
    paths(
        routes::public_v1::airports::list_airports,
        routes::public_v1::airports::search_airports,
        routes::public_v1::airports::get_airport,
        routes::public_v1::rankings::get_rankings,
        routes::public_v1::rankings::get_delays,
        routes::public_v1::rankings::get_busiest,
        routes::public_v1::rankings::get_best_reviewed,
        routes::public_v1::rankings::get_most_connected,
        routes::public_v1::rankings::get_map,
        routes::public_v1::sub_endpoints::get_airport_routes,
        routes::public_v1::sub_endpoints::get_airport_passengers,
        routes::public_v1::sub_endpoints::get_airport_operational,
        routes::public_v1::sub_endpoints::get_airport_sentiment,
        routes::public_v1::countries::list_countries,
        routes::public_v1::countries::airports_by_country,
        routes::public_v1::operators::list_operators,
        routes::public_v1::operators::get_operator,
        routes::public_v1::awards::list_awards,
    ),
)]
struct V1ApiDoc;

async fn v1_openapi_spec() -> Json<utoipa::openapi::OpenApi> {
    use utoipa::openapi::ResponseBuilder;

    let mut spec = V1ApiDoc::openapi();

    // Add 429 response to every operation.
    let rate_limit_response = ResponseBuilder::new()
        .description("Rate limit exceeded — 100 requests/hour per IP. Retry after the cooldown period.")
        .build();

    for (_path, item) in spec.paths.paths.iter_mut() {
        let ops = [
            &mut item.get, &mut item.post, &mut item.put,
            &mut item.delete, &mut item.patch, &mut item.head,
        ];
        for op in ops.into_iter().flatten() {
            op.responses.responses.insert(
                "429".to_string(),
                utoipa::openapi::RefOr::T(rate_limit_response.clone()),
            );
        }
    }

    Json(spec)
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

    let scraper_pool = crate::scraper_pool::ScraperPool::from_env();
    let jobs = Arc::new(JobManager::new(pool.clone(), 3, scraper_pool.clone()));
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
        .route("/reviews", post(routes::cron::cron_reviews))
        .route("/sync-eurocontrol", post(routes::cron::cron_sync_eurocontrol));

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
        .route("/airports/{iata}/live", get(routes::live::get_live_pulse))
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

    // Public API v1 — no auth, IP-rate-limited (100 req/hour).
    let governor_conf = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(36)  // refill rate: ~100/hour (1 token every 36s)
            .burst_size(10)  // allow small bursts
            .finish()
            .expect("governor config"),
    );

    let v1_routes = routes::public_v1::router(state.clone());

    let app = Router::new()
        // Health check sits outside all auth middleware (for load balancers).
        .route("/health", get(health))
        // OpenAPI spec endpoints (no auth).
        .route("/openapi.json", get(openapi_spec))
        .route("/api/v1/openapi.json", get(v1_openapi_spec))
        // SSE logs — outside API key middleware since EventSource can't send headers.
        .nest("/api/admin", logs_route)
        // Public API v1 — no auth, rate-limited.
        .nest("/api/v1", v1_routes.with_state(state.clone()).layer(GovernorLayer {
            config: governor_conf,
        }))
        // Internal API routes nested under /api (requires API key).
        .nest("/api", api_routes)
        .with_state(state)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
    info!(port, "API server listening");
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await?;

    Ok(())
}
