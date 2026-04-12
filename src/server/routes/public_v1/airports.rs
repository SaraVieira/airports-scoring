use axum::extract::{Path, Query, State};
use axum::Json;

use crate::server::AppState;
use super::types::*;

#[utoipa::path(get, path = "/api/v1/airports",
    summary = "List all scored airports",
    description = "Returns all airports that have been scored, ordered by total score descending. Only includes airports with enough routes to qualify for scoring.",
    responses((status = 200, description = "Scored airports", body = Vec<V1AirportListItem>)),
    tag = "Airports"
)]
pub async fn list_airports(
    State(state): State<AppState>,
) -> ApiResult<Vec<V1AirportListItem>> {
    sqlx::query_as::<_, V1AirportListItem>(
        "SELECT a.iata_code, a.name, a.city, a.country_code,
                s.score_total::float8 as score_total,
                s.score_sentiment_velocity::float8 as score_sentiment_velocity,
                (SELECT COUNT(*) FROM airport_awards aw WHERE aw.iata_code = a.iata_code) as award_count
         FROM airport_scores s
         INNER JOIN airports a ON a.id = s.airport_id
         WHERE s.is_latest = TRUE AND a.in_seed_set = TRUE
           AND (SELECT COUNT(*) FROM routes r WHERE r.origin_id = a.id) >= $1
         ORDER BY s.score_total DESC",
    )
    .bind(crate::scoring::MIN_ROUTES_FOR_SCORING)
    .fetch_all(&state.pool)
    .await
    .map(Json)
    .map_err(|_| ApiError::internal())
}

#[utoipa::path(get, path = "/api/v1/airports/search",
    summary = "Search airports",
    description = "Search airports by IATA code, name, or city. Returns up to 8 matches ordered by score.",
    params(("q" = String, Query, description = "Search query (e.g. 'munich', 'MUC', 'berlin')")),
    responses((status = 200, description = "Search results", body = Vec<V1SearchResult>)),
    tag = "Airports"
)]
pub async fn search_airports(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> ApiResult<Vec<V1SearchResult>> {
    let escaped_q = params
        .q
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    let pattern = format!("%{}%", escaped_q);

    sqlx::query_as::<_, V1SearchResult>(
        "SELECT a.iata_code, a.name, a.city, a.country_code,
                s.score_total::float8 as score_total
         FROM airports a
         LEFT JOIN airport_scores s ON s.airport_id = a.id AND s.is_latest = TRUE
         WHERE a.iata_code ILIKE $1 OR a.name ILIKE $1 OR a.city ILIKE $1
         ORDER BY s.score_total DESC NULLS LAST
         LIMIT 8",
    )
    .bind(&pattern)
    .fetch_all(&state.pool)
    .await
    .map(Json)
    .map_err(|_| ApiError::internal())
}

// ── Detail helpers ──────────────────────────────────────────

fn dec_to_f64(d: Option<rust_decimal::Decimal>) -> Option<f64> {
    d.and_then(|v| v.to_string().parse::<f64>().ok())
}

#[derive(sqlx::FromRow)]
struct OrgRow {
    name: String,
    short_name: Option<String>,
    country_code: Option<String>,
    org_type: Option<String>,
    ownership_model: Option<String>,
    public_share_pct: Option<f64>,
}

#[derive(sqlx::FromRow)]
struct CountryRow {
    code: String,
    name: String,
}

#[derive(sqlx::FromRow)]
struct ScoreRow {
    score_total: Option<rust_decimal::Decimal>,
    score_infrastructure: Option<rust_decimal::Decimal>,
    score_operational: Option<rust_decimal::Decimal>,
    score_sentiment: Option<rust_decimal::Decimal>,
    score_sentiment_velocity: Option<rust_decimal::Decimal>,
    score_connectivity: Option<rust_decimal::Decimal>,
    score_operator: Option<rust_decimal::Decimal>,
    commentary: Option<String>,
    scored_at: chrono::DateTime<chrono::Utc>,
}

#[derive(sqlx::FromRow)]
struct RankingRow {
    position: Option<i64>,
    total: i64,
}

#[derive(sqlx::FromRow)]
struct SentimentRow {
    avg_rating: Option<rust_decimal::Decimal>,
    review_count: Option<i32>,
    positive_pct: Option<rust_decimal::Decimal>,
    negative_pct: Option<rust_decimal::Decimal>,
    neutral_pct: Option<rust_decimal::Decimal>,
    score_queuing: Option<rust_decimal::Decimal>,
    score_cleanliness: Option<rust_decimal::Decimal>,
    score_staff: Option<rust_decimal::Decimal>,
    score_food_bev: Option<rust_decimal::Decimal>,
    score_shopping: Option<rust_decimal::Decimal>,
    score_wifi: Option<rust_decimal::Decimal>,
    score_wayfinding: Option<rust_decimal::Decimal>,
    score_transport: Option<rust_decimal::Decimal>,
}

#[derive(sqlx::FromRow)]
struct RouteSummaryRow {
    count: i64,
    airlines: i64,
}

#[derive(sqlx::FromRow)]
struct GroundTransportRow {
    transport_modes_count: Option<i16>,
    has_direct_rail: Option<bool>,
    has_metro: Option<bool>,
    has_bus: Option<bool>,
    has_rail: Option<bool>,
    has_tram: Option<bool>,
}

#[derive(sqlx::FromRow)]
struct LoungeRow {
    lounge_name: String,
    terminal: Option<String>,
    source: Option<String>,
}

#[derive(sqlx::FromRow)]
struct HubStatusRow {
    airline_name: String,
    status_type: String,
}

#[derive(sqlx::FromRow)]
struct CarbonRow {
    level: Option<i16>,
    level_name: Option<String>,
    report_year: Option<i16>,
}

#[utoipa::path(get, path = "/api/v1/airports/{iata}",
    summary = "Get airport details",
    description = "Full airport profile including scores, sentiment summary, operator, runways, awards, ground transport, lounges, hub status, and carbon accreditation. Scores are flattened to the latest version. Routes are summarized as counts — use /airports/{iata}/routes for the full list.",
    params(("iata" = String, Path, description = "IATA airport code (e.g. MUC, LHR, CDG)")),
    responses(
        (status = 200, description = "Airport detail", body = V1AirportDetail),
        (status = 404, description = "Airport not found", body = ApiError),
    ),
    tag = "Airports"
)]
pub async fn get_airport(
    State(state): State<AppState>,
    Path(iata): Path<String>,
) -> ApiResult<V1AirportDetail> {
    let iata_upper = iata.to_uppercase();
    let pool = &state.pool;

    let airport = super::lookup_airport(pool, &iata_upper).await?;
    let aid = airport.id;

    // Operator
    let operator = if let Some(op_id) = airport.operator_id {
        sqlx::query_as::<_, OrgRow>(
            "SELECT name, short_name, country_code, org_type, ownership_model,
                    public_share_pct::float8 as public_share_pct
             FROM organisations WHERE id = $1",
        )
        .bind(op_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .map(|r| V1Operator {
            name: r.name,
            short_name: r.short_name,
            country_code: r.country_code,
            org_type: r.org_type,
            ownership_model: r.ownership_model,
            public_share_pct: r.public_share_pct,
        })
    } else {
        None
    };

    // Country
    let country = sqlx::query_as::<_, CountryRow>(
        "SELECT iso_code AS code, name FROM countries WHERE iso_code = $1",
    )
    .bind(&airport.country_code)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .map(|r| V1Country {
        code: r.code,
        name: r.name,
    });

    // Latest score
    let scores = sqlx::query_as::<_, ScoreRow>(
        "SELECT score_total, score_infrastructure, score_operational,
                score_sentiment, score_sentiment_velocity, score_connectivity,
                score_operator, commentary, scored_at
         FROM airport_scores WHERE airport_id = $1 AND is_latest = TRUE
         LIMIT 1",
    )
    .bind(aid)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .map(|r| V1Scores {
        total: dec_to_f64(r.score_total).unwrap_or(0.0),
        infrastructure: dec_to_f64(r.score_infrastructure),
        operational: dec_to_f64(r.score_operational),
        sentiment: dec_to_f64(r.score_sentiment),
        sentiment_velocity: dec_to_f64(r.score_sentiment_velocity),
        connectivity: dec_to_f64(r.score_connectivity),
        operator: dec_to_f64(r.score_operator),
        commentary: r.commentary,
        scored_at: r.scored_at.to_rfc3339(),
    });

    // Ranking
    let ranking = sqlx::query_as::<_, RankingRow>(
        "SELECT
            (SELECT COUNT(*) + 1 FROM airport_scores s2
             WHERE s2.is_latest = TRUE AND s2.score_total > s.score_total) as position,
            (SELECT COUNT(*) FROM airport_scores WHERE is_latest = TRUE) as total
         FROM airport_scores s
         WHERE s.airport_id = $1 AND s.is_latest = TRUE",
    )
    .bind(aid)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .map(|r| V1Ranking {
        position: r.position,
        total: r.total,
    })
    .unwrap_or(V1Ranking {
        position: None,
        total: 0,
    });

    // Aggregated sentiment (latest quarter per source)
    let sentiment_rows = sqlx::query_as::<_, SentimentRow>(
        "SELECT avg_rating, review_count, positive_pct, negative_pct, neutral_pct,
                score_queuing, score_cleanliness, score_staff, score_food_bev,
                score_shopping, score_wifi, score_wayfinding, score_transport
         FROM sentiment_snapshots
         WHERE airport_id = $1
         ORDER BY snapshot_year DESC, snapshot_quarter DESC NULLS LAST",
    )
    .bind(aid)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let sentiment = if sentiment_rows.is_empty() {
        None
    } else {
        let total_reviews: i64 = sentiment_rows
            .iter()
            .map(|r| r.review_count.unwrap_or(0) as i64)
            .sum();
        // Use the first (most recent) row for ratings/percentages
        let latest = &sentiment_rows[0];
        Some(V1SentimentSummary {
            avg_rating: dec_to_f64(latest.avg_rating),
            review_count: total_reviews,
            positive_pct: dec_to_f64(latest.positive_pct),
            negative_pct: dec_to_f64(latest.negative_pct),
            neutral_pct: dec_to_f64(latest.neutral_pct),
            sub_scores: V1SubScores {
                queuing: dec_to_f64(latest.score_queuing),
                cleanliness: dec_to_f64(latest.score_cleanliness),
                staff: dec_to_f64(latest.score_staff),
                food_bev: dec_to_f64(latest.score_food_bev),
                shopping: dec_to_f64(latest.score_shopping),
                wifi: dec_to_f64(latest.score_wifi),
                wayfinding: dec_to_f64(latest.score_wayfinding),
                transport: dec_to_f64(latest.score_transport),
            },
        })
    };

    // Route summary
    let routes = sqlx::query_as::<_, RouteSummaryRow>(
        "SELECT COUNT(DISTINCT destination_iata)::int8 as count,
                COUNT(DISTINCT airline_iata)::int8 as airlines
         FROM routes WHERE origin_id = $1",
    )
    .bind(aid)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .map(|r| V1RouteSummary {
        count: r.count,
        airlines: r.airlines,
    })
    .unwrap_or(V1RouteSummary {
        count: 0,
        airlines: 0,
    });

    // Runways
    let runways = sqlx::query_as::<_, V1Runway>(
        "SELECT ident, length_ft, width_ft, surface, lighted
         FROM runways WHERE airport_id = $1 ORDER BY length_ft DESC NULLS LAST",
    )
    .bind(aid)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    // Awards
    let awards = sqlx::query_as::<_, V1Award>(
        "SELECT source, year, category, region, size_bucket, rank
         FROM airport_awards WHERE iata_code = $1
         ORDER BY year DESC, source",
    )
    .bind(&iata_upper)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    // Ground transport
    let ground_transport = sqlx::query_as::<_, GroundTransportRow>(
        "SELECT transport_modes_count, has_direct_rail, has_metro, has_bus, has_rail, has_tram
         FROM ground_transport WHERE airport_id = $1 LIMIT 1",
    )
    .bind(aid)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .map(|r| V1GroundTransport {
        transport_modes_count: r.transport_modes_count,
        has_direct_rail: r.has_direct_rail,
        has_metro: r.has_metro,
        has_bus: r.has_bus,
        has_rail: r.has_rail,
        has_tram: r.has_tram,
    });

    // Lounges
    let lounges: Vec<V1Lounge> = sqlx::query_as::<_, LoungeRow>(
        "SELECT lounge_name, terminal, source FROM lounges WHERE airport_id = $1",
    )
    .bind(aid)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|r| V1Lounge {
        lounge_name: r.lounge_name,
        terminal: r.terminal,
        source: r.source,
    })
    .collect();

    // Hub status
    let hub_status: Vec<V1HubStatus> = sqlx::query_as::<_, HubStatusRow>(
        "SELECT airline_name, status_type FROM hub_status WHERE airport_id = $1",
    )
    .bind(aid)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|r| V1HubStatus {
        airline_name: r.airline_name,
        status_type: r.status_type,
    })
    .collect();

    // Carbon accreditation (latest)
    let carbon_accreditation = sqlx::query_as::<_, CarbonRow>(
        "SELECT level, level_name, report_year FROM carbon_accreditation
         WHERE airport_id = $1 ORDER BY report_year DESC NULLS LAST LIMIT 1",
    )
    .bind(aid)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .and_then(|r| {
        Some(V1CarbonAccreditation {
            level: r.level?,
            level_name: r.level_name?,
            report_year: r.report_year,
        })
    });

    // Score status
    let score_status = if scores.is_some() {
        "scored"
    } else if (routes.count) < crate::scoring::MIN_ROUTES_FOR_SCORING as i64 {
        "too_small"
    } else {
        "pending"
    };

    Ok(Json(V1AirportDetail {
        iata_code: airport.iata_code.unwrap_or_default(),
        icao_code: airport.icao_code,
        name: airport.name,
        short_name: airport.short_name,
        city: airport.city,
        country_code: airport.country_code,
        elevation_ft: airport.elevation_ft,
        timezone: airport.timezone,
        terminal_count: airport.terminal_count,
        total_gates: airport.total_gates,
        opened_year: airport.opened_year,
        last_major_reno: airport.last_major_reno,
        annual_capacity_m: airport.annual_capacity_m.and_then(|d| d.to_string().parse().ok()),
        annual_pax_latest_m: airport.annual_pax_latest_m.and_then(|d| d.to_string().parse().ok()),
        latest_pax_year: airport.latest_pax_year,
        wikipedia_url: airport.wikipedia_url,
        website_url: airport.website_url,
        operator,
        country,
        scores,
        ranking,
        score_status: score_status.to_string(),
        sentiment,
        routes,
        runways,
        awards,
        ground_transport,
        lounges,
        hub_status,
        carbon_accreditation,
    }))
}
