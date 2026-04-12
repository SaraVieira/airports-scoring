pub mod airports;
pub mod awards;
pub mod countries;
pub mod operators;
pub mod rankings;
pub mod sub_endpoints;
pub mod types;

use axum::{routing::get, Json, Router};
use sqlx::PgPool;

use crate::server::AppState;
use types::ApiError;

/// Look up an airport by IATA code, returning a 404 ApiError if not found.
pub(crate) async fn lookup_airport(
    pool: &PgPool,
    iata: &str,
) -> Result<crate::models::Airport, (axum::http::StatusCode, Json<ApiError>)> {
    crate::db::get_airport_by_iata(pool, iata)
        .await
        .map_err(|_| ApiError::not_found("Airport not found"))
}

/// Build the public API v1 router. No auth, rate limiting applied externally.
pub fn router(_state: AppState) -> Router<AppState> {
    Router::new()
        .route("/health", get(v1_health))
        .route("/docs", get(v1_docs))
        // Airports
        .route("/airports", get(airports::list_airports))
        .route("/airports/search", get(airports::search_airports))
        .route("/airports/rankings", get(rankings::get_rankings))
        .route("/airports/delays", get(rankings::get_delays))
        .route("/airports/busiest", get(rankings::get_busiest))
        .route(
            "/airports/best-reviewed",
            get(rankings::get_best_reviewed),
        )
        .route(
            "/airports/most-connected",
            get(rankings::get_most_connected),
        )
        .route("/airports/map", get(rankings::get_map))
        // Airport sub-endpoints (must come before /{iata} catch-all)
        .route(
            "/airports/{iata}/routes",
            get(sub_endpoints::get_airport_routes),
        )
        .route(
            "/airports/{iata}/passengers",
            get(sub_endpoints::get_airport_passengers),
        )
        .route(
            "/airports/{iata}/operational",
            get(sub_endpoints::get_airport_operational),
        )
        .route(
            "/airports/{iata}/sentiment",
            get(sub_endpoints::get_airport_sentiment),
        )
        // Airport detail (after specific routes)
        .route("/airports/{iata}", get(airports::get_airport))
        // Countries
        .route("/countries", get(countries::list_countries))
        .route(
            "/countries/{code}/airports",
            get(countries::airports_by_country),
        )
        // Operators
        .route("/operators", get(operators::list_operators))
        .route("/operators/{slug}", get(operators::get_operator))
        // Awards
        .route("/awards", get(awards::list_awards))
}

async fn v1_health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "version": "v1",
        "rateLimit": "100 requests/hour"
    }))
}

async fn v1_docs() -> axum::response::Html<&'static str> {
    axum::response::Html(
        r#"<!doctype html>
<html>
  <head>
    <title>Airport Intelligence API</title>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
  </head>
  <body>
    <div id="app"></div>
    <script src="https://cdn.jsdelivr.net/npm/@scalar/api-reference"></script>
    <script>
      Scalar.createApiReference('#app', {
        url: '/api/v1/openapi.json',
        agent: {
          disabled: true,
        },
        baseServerURL: 'https://api.airports.report',
        metaData: {
          title: 'Airport Intelligence API — Free European Airport Data',
          description: 'Free public API for European airport scores, rankings, sentiment, delays, routes, awards, and operator data. No authentication required.',
          ogTitle: 'Airport Intelligence API',
          ogDescription: 'Free API for European airport scores, rankings, sentiment, and operational data. 150+ airports, no auth required.',
          ogImage: 'https://airports.report/og-api.png',
          twitterCard: 'summary_large_image',
        },
        mcp: {
            disabled: true,
        }
      })
    </script>
  </body>
</html>"#,
    )
}
