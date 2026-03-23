use anyhow::Result;
use chrono::Datelike;
use rust_decimal::prelude::*;
use sqlx::PgPool;

use crate::models::Airport;

/// Raw data pulled from DB for scoring an airport.
#[derive(Debug)]
pub(crate) struct ScoringData {
    // Infrastructure
    pub runway_count: i64,
    pub max_runway_length_ft: Option<i32>,
    pub annual_capacity_m: Option<f64>,
    pub annual_pax_latest_m: Option<f64>,
    pub opened_year: Option<i16>,
    pub last_major_reno: Option<i16>,
    // Operational (weighted average across all years)
    pub avg_delay_pct: Option<f64>,
    pub avg_cancellation_pct: Option<f64>,
    pub avg_delay_minutes: Option<f64>,
    pub delay_airport_pct: Option<f64>,
    pub taxi_out_additional_min: Option<f64>,
    // Sentiment (weighted average across all snapshots)
    pub weighted_avg_rating: Option<f64>,
    pub total_review_count: Option<i32>,
    pub sub_score_count: i32,
    pub sub_score_sum: f64,
    // Velocity (8-quarter rolling averages for longer arc)
    pub avg_rating_last_8q: Option<f64>,
    pub avg_rating_prior_8q: Option<f64>,
    // Connectivity
    pub destination_count: i64,
    pub airline_count: i64,
    pub international_pax: Option<i64>,
    pub total_pax: Option<i64>,
    // Operator portfolio
    pub operator_avg_sentiment: Option<f64>,
    pub operator_avg_operational: Option<f64>,
    pub operator_airport_count: i64,
    // Ground transport
    pub transport_modes_count: i16,
    pub has_direct_rail: bool,
    // Hub status
    pub hub_airline_count: i64,
    pub focus_city_count: i64,
    pub operating_base_count: i64,
    // Lounges
    pub lounge_count: i64,
    // Carbon accreditation
    pub carbon_level: Option<i16>, // 1-7 or None
}

/// Year-based recency weight: recent years count more.
/// weight(year) = 1 + max(0, (year - 2015) * 0.3)
/// 2015: 1.0, 2020: 2.5, 2025: 4.0
pub(crate) fn year_weight(year: i16) -> f64 {
    1.0 + (year as f64 - 2015.0).max(0.0) * 0.3
}

/// Operational stats for a single year, used for weighted averaging.
#[derive(Debug, sqlx::FromRow)]
pub(crate) struct YearlyOps {
    pub period_year: i16,
    pub avg_delay_pct: Option<Decimal>,
    pub avg_cancellation_pct: Option<Decimal>,
    pub avg_delay_minutes: Option<Decimal>,
    pub avg_delay_airport_pct: Option<Decimal>,
}

/// Sentiment snapshot for weighted averaging across all time.
#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
pub(crate) struct SentimentRow {
    pub snapshot_year: i16,
    pub snapshot_quarter: Option<i16>,
    pub avg_rating: Option<Decimal>,
    pub review_count: Option<i32>,
    pub score_queuing: Option<Decimal>,
    pub score_cleanliness: Option<Decimal>,
    pub score_staff: Option<Decimal>,
    pub score_food_bev: Option<Decimal>,
    pub score_shopping: Option<Decimal>,
    pub score_wifi: Option<Decimal>,
    pub score_wayfinding: Option<Decimal>,
    pub score_transport: Option<Decimal>,
}

pub(crate) async fn gather_scoring_data(
    pool: &PgPool,
    airport: &Airport,
) -> Result<ScoringData> {
    // Runway stats (current state — no time filtering)
    let runway_stats: (i64, Option<i32>) = sqlx::query_as(
        "SELECT COUNT(*), MAX(length_ft) FROM runways WHERE airport_id = $1 AND closed = FALSE",
    )
    .bind(airport.id)
    .fetch_one(pool)
    .await?;

    // Operational stats — ALL years, grouped by year for weighted averaging in Rust
    let yearly_ops: Vec<YearlyOps> = sqlx::query_as(
        "SELECT period_year, \
                AVG(delay_pct) as avg_delay_pct, \
                AVG(cancellation_pct) as avg_cancellation_pct, \
                AVG(avg_delay_minutes) as avg_delay_minutes, \
                AVG(delay_airport_pct) as avg_delay_airport_pct \
         FROM operational_stats WHERE airport_id = $1 \
         GROUP BY period_year ORDER BY period_year",
    )
    .bind(airport.id)
    .fetch_all(pool)
    .await?;

    // Compute weighted averages for operational data
    let (avg_delay_pct, avg_cancellation_pct, avg_delay_minutes, delay_airport_pct) =
        weighted_avg_ops(&yearly_ops);

    // Sentiment — ALL snapshots for weighted averaging
    let all_sentiment: Vec<SentimentRow> = sqlx::query_as(
        "SELECT snapshot_year, snapshot_quarter, avg_rating, review_count, \
                score_queuing, score_cleanliness, score_staff, score_food_bev, \
                score_shopping, score_wifi, score_wayfinding, score_transport \
         FROM sentiment_snapshots \
         WHERE airport_id = $1 \
         ORDER BY snapshot_year, snapshot_quarter",
    )
    .bind(airport.id)
    .fetch_all(pool)
    .await?;

    // Compute weighted sentiment averages (by recency AND review count)
    let (weighted_avg_rating, total_review_count, sub_score_sum, sub_score_count) =
        weighted_avg_sentiment(&all_sentiment);

    // Velocity: compare last 8 quarters vs prior 8 quarters (longer arc)
    let current_year = chrono::Utc::now().naive_utc().date().year() as i32;
    let last_8q: Option<(Option<Decimal>,)> = sqlx::query_as(
        "SELECT AVG(avg_rating) \
         FROM sentiment_snapshots \
         WHERE airport_id = $1 \
         AND (snapshot_year * 4 + COALESCE(snapshot_quarter, 4)) > ($2 * 4 - 8) \
         AND (snapshot_year * 4 + COALESCE(snapshot_quarter, 4)) <= ($2 * 4)",
    )
    .bind(airport.id)
    .bind(current_year)
    .fetch_optional(pool)
    .await?;

    let prior_8q: Option<(Option<Decimal>,)> = sqlx::query_as(
        "SELECT AVG(avg_rating) \
         FROM sentiment_snapshots \
         WHERE airport_id = $1 \
         AND (snapshot_year * 4 + COALESCE(snapshot_quarter, 4)) > ($2 * 4 - 16) \
         AND (snapshot_year * 4 + COALESCE(snapshot_quarter, 4)) <= ($2 * 4 - 8)",
    )
    .bind(airport.id)
    .bind(current_year)
    .fetch_optional(pool)
    .await?;

    // Route connectivity — current state only
    let connectivity: (i64, i64) = sqlx::query_as(
        "SELECT COUNT(DISTINCT COALESCE(destination_icao, destination_iata)), \
                COUNT(DISTINCT COALESCE(airline_icao, airline_iata)) \
         FROM routes WHERE origin_id = $1",
    )
    .bind(airport.id)
    .fetch_one(pool)
    .await?;

    // Latest pax for international ratio
    let pax: Option<(Option<i64>, Option<i64>)> = sqlx::query_as(
        "SELECT international_pax, total_pax FROM pax_yearly \
         WHERE airport_id = $1 ORDER BY year DESC LIMIT 1",
    )
    .bind(airport.id)
    .fetch_optional(pool)
    .await?;

    // Operator portfolio
    let operator_portfolio: Option<(Option<Decimal>, Option<Decimal>, i64)> = sqlx::query_as(
        "SELECT AVG(s.score_sentiment), AVG(s.score_operational), COUNT(DISTINCT a.id) \
         FROM airport_scores s \
         JOIN airports a ON a.id = s.airport_id \
         WHERE a.operator_id = $1 AND s.is_latest = TRUE",
    )
    .bind(airport.operator_id)
    .fetch_optional(pool)
    .await?;

    // Ground transport
    let transport: Option<(i16, bool)> = sqlx::query_as(
        "SELECT transport_modes_count, has_direct_rail \
         FROM ground_transport WHERE airport_id = $1 \
         ORDER BY fetched_at DESC LIMIT 1"
    )
    .bind(airport.id)
    .fetch_optional(pool)
    .await?;

    // Hub status counts
    let hub_counts: (i64, i64, i64) = sqlx::query_as(
        "SELECT \
             COUNT(*) FILTER (WHERE status_type = 'hub'), \
             COUNT(*) FILTER (WHERE status_type = 'focus_city'), \
             COUNT(*) FILTER (WHERE status_type = 'operating_base') \
         FROM hub_status WHERE airport_id = $1"
    )
    .bind(airport.id)
    .fetch_one(pool)
    .await?;

    // Lounge count
    let lounge_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM lounges WHERE airport_id = $1"
    )
    .bind(airport.id)
    .fetch_one(pool)
    .await?;

    // Carbon accreditation
    let carbon: Option<(i16,)> = sqlx::query_as(
        "SELECT level FROM carbon_accreditation WHERE airport_id = $1 \
         ORDER BY report_year DESC LIMIT 1"
    )
    .bind(airport.id)
    .fetch_optional(pool)
    .await?;

    Ok(ScoringData {
        runway_count: runway_stats.0,
        max_runway_length_ft: runway_stats.1,
        annual_capacity_m: airport.annual_capacity_m.as_ref().and_then(|d| d.to_f64()),
        annual_pax_latest_m: airport.annual_pax_latest_m.as_ref().and_then(|d| d.to_f64()),
        opened_year: airport.opened_year,
        last_major_reno: airport.last_major_reno,
        avg_delay_pct,
        avg_cancellation_pct,
        avg_delay_minutes,
        delay_airport_pct,
        taxi_out_additional_min: None, // TODO: Eurocontrol PRU ASMA/taxi-out
        weighted_avg_rating,
        total_review_count,
        sub_score_count,
        sub_score_sum,
        avg_rating_last_8q: last_8q
            .as_ref()
            .and_then(|s| s.0.as_ref())
            .and_then(|d| d.to_f64()),
        avg_rating_prior_8q: prior_8q
            .as_ref()
            .and_then(|s| s.0.as_ref())
            .and_then(|d| d.to_f64()),
        destination_count: connectivity.0,
        airline_count: connectivity.1,
        international_pax: pax.as_ref().and_then(|p| p.0),
        total_pax: pax.as_ref().and_then(|p| p.1),
        operator_avg_sentiment: operator_portfolio
            .as_ref()
            .and_then(|o| o.0.as_ref())
            .and_then(|d| d.to_f64()),
        operator_avg_operational: operator_portfolio
            .as_ref()
            .and_then(|o| o.1.as_ref())
            .and_then(|d| d.to_f64()),
        operator_airport_count: operator_portfolio
            .as_ref()
            .map(|o| o.2)
            .unwrap_or(0),
        transport_modes_count: transport.as_ref().map(|t| t.0).unwrap_or(0),
        has_direct_rail: transport.as_ref().map(|t| t.1).unwrap_or(false),
        hub_airline_count: hub_counts.0,
        focus_city_count: hub_counts.1,
        operating_base_count: hub_counts.2,
        lounge_count: lounge_count.0,
        carbon_level: carbon.map(|c| c.0),
    })
}

/// Compute weighted averages for operational data across all years.
/// weight(year) = 1 + max(0, (year - 2015) * 0.3)
pub(crate) fn weighted_avg_ops(
    yearly: &[YearlyOps],
) -> (Option<f64>, Option<f64>, Option<f64>, Option<f64>) {
    if yearly.is_empty() {
        return (None, None, None, None);
    }

    let mut delay_sum = 0.0_f64;
    let mut delay_weight = 0.0_f64;
    let mut cancel_sum = 0.0_f64;
    let mut cancel_weight = 0.0_f64;
    let mut avg_delay_sum = 0.0_f64;
    let mut avg_delay_weight = 0.0_f64;
    let mut airport_pct_sum = 0.0_f64;
    let mut airport_pct_weight = 0.0_f64;

    for row in yearly {
        let w = year_weight(row.period_year);

        if let Some(v) = row.avg_delay_pct.and_then(|d| d.to_f64()) {
            delay_sum += v * w;
            delay_weight += w;
        }
        if let Some(v) = row.avg_cancellation_pct.and_then(|d| d.to_f64()) {
            cancel_sum += v * w;
            cancel_weight += w;
        }
        if let Some(v) = row.avg_delay_minutes.and_then(|d| d.to_f64()) {
            avg_delay_sum += v * w;
            avg_delay_weight += w;
        }
        if let Some(v) = row.avg_delay_airport_pct.and_then(|d| d.to_f64()) {
            airport_pct_sum += v * w;
            airport_pct_weight += w;
        }
    }

    let avg = |sum: f64, weight: f64| -> Option<f64> {
        if weight > 0.0 { Some(sum / weight) } else { None }
    };

    (
        avg(delay_sum, delay_weight),
        avg(cancel_sum, cancel_weight),
        avg(avg_delay_sum, avg_delay_weight),
        avg(airport_pct_sum, airport_pct_weight),
    )
}

/// Compute weighted sentiment averages across all snapshots.
/// Weight = year_weight(year) * sqrt(review_count) — more reviews = higher confidence.
pub(crate) fn weighted_avg_sentiment(
    snapshots: &[SentimentRow],
) -> (Option<f64>, Option<i32>, f64, i32) {
    if snapshots.is_empty() {
        return (None, None, 0.0, 0);
    }

    let mut rating_sum = 0.0_f64;
    let mut rating_weight = 0.0_f64;
    let mut total_reviews = 0_i32;

    // For sub-scores, accumulate weighted values from ALL snapshots
    let mut sub_sums = [0.0_f64; 8]; // queuing, cleanliness, staff, food, shopping, wifi, wayfinding, transport
    let mut sub_weights = [0.0_f64; 8];

    for snap in snapshots {
        let w = year_weight(snap.snapshot_year);
        let review_factor = snap.review_count.map(|c| (c as f64).sqrt()).unwrap_or(1.0);
        let combined_weight = w * review_factor;

        if let Some(rating) = snap.avg_rating.as_ref().and_then(|d| d.to_f64()) {
            rating_sum += rating * combined_weight;
            rating_weight += combined_weight;
        }

        if let Some(c) = snap.review_count {
            total_reviews += c;
        }

        // Sub-scores
        let sub_scores = [
            &snap.score_queuing, &snap.score_cleanliness, &snap.score_staff,
            &snap.score_food_bev, &snap.score_shopping, &snap.score_wifi,
            &snap.score_wayfinding, &snap.score_transport,
        ];
        for (i, score) in sub_scores.iter().enumerate() {
            if let Some(v) = score.as_ref().and_then(|d| d.to_f64()) {
                sub_sums[i] += v * combined_weight;
                sub_weights[i] += combined_weight;
            }
        }
    }

    let weighted_rating = if rating_weight > 0.0 {
        Some(rating_sum / rating_weight)
    } else {
        None
    };

    // Compute weighted sub-score average
    let mut sub_score_sum = 0.0_f64;
    let mut sub_score_count = 0_i32;
    for i in 0..8 {
        if sub_weights[i] > 0.0 {
            sub_score_sum += sub_sums[i] / sub_weights[i];
            sub_score_count += 1;
        }
    }

    (
        weighted_rating,
        if total_reviews > 0 { Some(total_reviews) } else { None },
        sub_score_sum,
        sub_score_count,
    )
}
