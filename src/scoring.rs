use anyhow::Result;
use rust_decimal::prelude::*;
use sqlx::PgPool;
use tracing::info;

use crate::models::Airport;

/// Default v1 weights for scoring dimensions.
const W_INFRASTRUCTURE: f64 = 0.15;
const W_OPERATIONAL: f64 = 0.25;
const W_SENTIMENT: f64 = 0.25;
const W_SENTIMENT_VELOCITY: f64 = 0.15;
const W_CONNECTIVITY: f64 = 0.10;
const W_OPERATOR: f64 = 0.10;

/// Result of a score computation for one airport.
#[derive(Debug, Clone)]
pub struct ScoreOutput {
    pub airport_id: i32,
    pub reference_year: i16,
    pub score_infrastructure: f64,
    pub score_operational: f64,
    pub score_sentiment: f64,
    pub score_sentiment_velocity: f64,
    pub score_connectivity: f64,
    pub score_operator: f64,
    pub score_total: f64,
    /// Free-text snarky commentary from the local ML pipeline.
    pub commentary: Option<String>,
}

/// Raw data pulled from DB for scoring an airport.
#[derive(Debug)]
struct ScoringData {
    runway_count: i64,
    max_runway_length_ft: Option<i32>,
    terminal_count: Option<i16>,
    total_gates: Option<i16>,
    annual_capacity_m: Option<f64>,
    // Operational
    avg_delay_pct: Option<f64>,
    avg_cancellation_pct: Option<f64>,
    avg_delay_minutes: Option<f64>,
    delay_airport_pct: Option<f64>,
    // Sentiment
    latest_avg_rating: Option<f64>,
    prev_avg_rating: Option<f64>,
    latest_positive_pct: Option<f64>,
    // Connectivity
    route_count: i64,
    airline_count: i64,
    // Operator portfolio
    operator_avg_score: Option<f64>,
}

/// Compute the composite score for a single airport.
pub async fn compute_score(
    pool: &PgPool,
    airport: &Airport,
    reference_year: i16,
) -> Result<ScoreOutput> {
    let data = gather_scoring_data(pool, airport, reference_year).await?;

    let infra = score_infrastructure(&data, airport, reference_year);
    let operational = score_operational(&data);
    let sentiment = score_sentiment(&data);
    let velocity = score_sentiment_velocity(&data);
    let connectivity = score_connectivity(&data);
    let operator = score_operator(&data);

    let total = infra * W_INFRASTRUCTURE
        + operational * W_OPERATIONAL
        + sentiment * W_SENTIMENT
        + velocity * W_SENTIMENT_VELOCITY
        + connectivity * W_CONNECTIVITY
        + operator * W_OPERATOR;

    Ok(ScoreOutput {
        airport_id: airport.id,
        reference_year,
        score_infrastructure: infra,
        score_operational: operational,
        score_sentiment: sentiment,
        score_sentiment_velocity: velocity,
        score_connectivity: connectivity,
        score_operator: operator,
        score_total: total,
        commentary: None, // Filled by ML pipeline (python/sentiment_pipeline.py)
    })
}

async fn gather_scoring_data(
    pool: &PgPool,
    airport: &Airport,
    reference_year: i16,
) -> Result<ScoringData> {
    // Runway stats
    let runway_stats: (i64, Option<i32>) = sqlx::query_as(
        "SELECT COUNT(*), MAX(length_ft) FROM runways WHERE airport_id = $1 AND closed = FALSE",
    )
    .bind(airport.id)
    .fetch_one(pool)
    .await?;

    // Operational stats (average over reference year)
    let ops: (Option<Decimal>, Option<Decimal>, Option<Decimal>, Option<Decimal>) = sqlx::query_as(
        "SELECT AVG(delay_pct), AVG(cancellation_pct), AVG(avg_delay_minutes), AVG(delay_airport_pct) \
         FROM operational_stats WHERE airport_id = $1 AND period_year = $2",
    )
    .bind(airport.id)
    .bind(reference_year)
    .fetch_one(pool)
    .await?;

    // Latest sentiment snapshot
    let latest_sentiment: Option<(Option<Decimal>, Option<Decimal>)> = sqlx::query_as(
        "SELECT avg_rating, positive_pct FROM sentiment_snapshots \
         WHERE airport_id = $1 ORDER BY snapshot_year DESC, snapshot_quarter DESC NULLS LAST LIMIT 1",
    )
    .bind(airport.id)
    .fetch_optional(pool)
    .await?;

    // Previous year sentiment for velocity
    let prev_sentiment: Option<(Option<Decimal>,)> = sqlx::query_as(
        "SELECT avg_rating FROM sentiment_snapshots \
         WHERE airport_id = $1 AND snapshot_year = $2 - 1 \
         ORDER BY snapshot_quarter DESC NULLS LAST LIMIT 1",
    )
    .bind(airport.id)
    .bind(reference_year)
    .fetch_optional(pool)
    .await?;

    // Route connectivity
    let connectivity: (i64, i64) = sqlx::query_as(
        "SELECT COUNT(DISTINCT destination_icao), COUNT(DISTINCT airline_icao) \
         FROM routes WHERE origin_id = $1",
    )
    .bind(airport.id)
    .fetch_one(pool)
    .await?;

    // Operator portfolio average (if this airport has an operator with other scored airports)
    let operator_avg: Option<(Decimal,)> = sqlx::query_as(
        "SELECT AVG(s.score_total) FROM airport_scores s \
         JOIN airports a ON a.id = s.airport_id \
         WHERE a.operator_id = $1 AND s.is_latest = TRUE AND a.id != $2",
    )
    .bind(airport.operator_id)
    .bind(airport.id)
    .fetch_optional(pool)
    .await?;

    Ok(ScoringData {
        runway_count: runway_stats.0,
        max_runway_length_ft: runway_stats.1,
        terminal_count: airport.terminal_count,
        total_gates: airport.total_gates,
        annual_capacity_m: airport.annual_capacity_m.as_ref().and_then(|d| d.to_f64()),
        avg_delay_pct: ops.0.and_then(|d| d.to_f64()),
        avg_cancellation_pct: ops.1.and_then(|d| d.to_f64()),
        avg_delay_minutes: ops.2.and_then(|d| d.to_f64()),
        delay_airport_pct: ops.3.and_then(|d| d.to_f64()),
        latest_avg_rating: latest_sentiment.as_ref().and_then(|s| s.0.as_ref()).and_then(|d| d.to_f64()),
        prev_avg_rating: prev_sentiment.as_ref().and_then(|s| s.0.as_ref()).and_then(|d| d.to_f64()),
        latest_positive_pct: latest_sentiment.as_ref().and_then(|s| s.1.as_ref()).and_then(|d| d.to_f64()),
        route_count: connectivity.0,
        airline_count: connectivity.1,
        operator_avg_score: operator_avg.as_ref().and_then(|o| o.0.to_f64()),
    })
}

/// Infrastructure: runways, capacity, terminals, gates, age.
/// Benchmarked against large European hub standards.
fn score_infrastructure(data: &ScoringData, airport: &Airport, reference_year: i16) -> f64 {
    let mut score = 50.0; // baseline

    // Runways (1=0, 2=+15, 3=+25, 4+=+30)
    score += match data.runway_count {
        0 => -20.0,
        1 => 0.0,
        2 => 15.0,
        3 => 25.0,
        _ => 30.0,
    };

    // Runway length (longer = can handle widebodies)
    if let Some(len) = data.max_runway_length_ft {
        score += if len >= 12000 { 10.0 }
        else if len >= 10000 { 5.0 }
        else { 0.0 };
    }

    // Terminals
    if let Some(t) = data.terminal_count {
        score += (t as f64 * 3.0).min(15.0);
    }

    // Capacity
    if let Some(cap) = data.annual_capacity_m {
        score += (cap * 0.3).min(10.0);
    }

    // Age penalty for very old airports without renovation
    if let Some(year) = airport.opened_year {
        let age = reference_year as i32 - year as i32;
        if age > 50 {
            score -= 5.0;
        }
    }

    score.clamp(0.0, 100.0)
}

/// Operational: delays, cancellations, airport-caused delays penalised more.
fn score_operational(data: &ScoringData) -> f64 {
    let mut score = 80.0; // start high, deduct for problems

    if let Some(delay_pct) = data.avg_delay_pct {
        // 10% delayed = -5, 20% = -15, 30% = -30
        score -= (delay_pct * 1.0).min(40.0);
    }

    if let Some(cancel_pct) = data.avg_cancellation_pct {
        score -= (cancel_pct * 3.0).min(20.0);
    }

    if let Some(avg_mins) = data.avg_delay_minutes {
        // Deduct for average delay length
        score -= (avg_mins * 0.5).min(15.0);
    }

    // Airport-caused delays are worse than weather/ATC
    if let Some(airport_pct) = data.delay_airport_pct {
        score -= (airport_pct * 0.5).min(10.0);
    }

    score.clamp(0.0, 100.0)
}

/// Sentiment: based on normalised avg_rating (0-5) and positive percentage.
fn score_sentiment(data: &ScoringData) -> f64 {
    match data.latest_avg_rating {
        Some(rating) => {
            // Rating is 0-5, map to 0-100
            let base = (rating / 5.0) * 80.0;
            // Bonus for high positive percentage
            let bonus = data.latest_positive_pct
                .map(|p| (p / 100.0) * 20.0)
                .unwrap_or(10.0);
            (base + bonus).clamp(0.0, 100.0)
        }
        None => 50.0, // no data = neutral
    }
}

/// Sentiment velocity: is the airport improving or declining?
/// 50 = flat, >50 = improving, <50 = declining.
fn score_sentiment_velocity(data: &ScoringData) -> f64 {
    match (data.latest_avg_rating, data.prev_avg_rating) {
        (Some(latest), Some(prev)) => {
            let delta = latest - prev;
            // Map delta to 0-100 scale: -2.0 = 0, 0 = 50, +2.0 = 100
            (50.0 + delta * 25.0).clamp(0.0, 100.0)
        }
        _ => 50.0, // no trend data = flat
    }
}

/// Connectivity: destination count, airline count.
fn score_connectivity(data: &ScoringData) -> f64 {
    // Destinations: 200+ = top score, scale linearly
    let dest_score = ((data.route_count as f64) / 200.0 * 70.0).min(70.0);
    // Airlines: 50+ = top score
    let airline_score = ((data.airline_count as f64) / 50.0 * 30.0).min(30.0);
    (dest_score + airline_score).clamp(0.0, 100.0)
}

/// Operator: portfolio average of other airports run by same operator.
fn score_operator(data: &ScoringData) -> f64 {
    data.operator_avg_score.unwrap_or(50.0)
}

/// Persist a computed score into the airport_scores table.
/// Wrapped in a transaction so the UPDATE + INSERT are atomic.
pub async fn upsert_score(pool: &PgPool, score: &ScoreOutput) -> Result<()> {
    let mut tx = pool.begin().await?;

    // Un-mark previous latest
    sqlx::query("UPDATE airport_scores SET is_latest = FALSE WHERE airport_id = $1 AND is_latest = TRUE")
        .bind(score.airport_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query(
        "INSERT INTO airport_scores \
         (airport_id, score_version, reference_year, \
          score_infrastructure, score_operational, score_sentiment, \
          score_sentiment_velocity, score_connectivity, score_operator, \
          score_total, \
          weight_infrastructure, weight_operational, weight_sentiment, \
          weight_sentiment_velocity, weight_connectivity, weight_operator, \
          is_latest, commentary) \
         VALUES ($1, 'v1', $2, $3, $4, $5, $6, $7, $8, $9, \
                 $10, $11, $12, $13, $14, $15, TRUE, $16)",
    )
    .bind(score.airport_id)
    .bind(score.reference_year)
    .bind(Decimal::from_f64(score.score_infrastructure))
    .bind(Decimal::from_f64(score.score_operational))
    .bind(Decimal::from_f64(score.score_sentiment))
    .bind(Decimal::from_f64(score.score_sentiment_velocity))
    .bind(Decimal::from_f64(score.score_connectivity))
    .bind(Decimal::from_f64(score.score_operator))
    .bind(Decimal::from_f64(score.score_total))
    .bind(Decimal::from_f64(W_INFRASTRUCTURE))
    .bind(Decimal::from_f64(W_OPERATIONAL))
    .bind(Decimal::from_f64(W_SENTIMENT))
    .bind(Decimal::from_f64(W_SENTIMENT_VELOCITY))
    .bind(Decimal::from_f64(W_CONNECTIVITY))
    .bind(Decimal::from_f64(W_OPERATOR))
    .bind(&score.commentary)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(())
}

/// Compute and persist scores for all airports in the list.
pub async fn score_airports(
    pool: &PgPool,
    airports: &[Airport],
    reference_year: i16,
) -> Result<()> {
    for airport in airports {
        let iata = airport.iata_code.as_deref().unwrap_or("???");
        info!(airport = iata, "Computing score");

        let score = compute_score(pool, airport, reference_year).await?;
        upsert_score(pool, &score).await?;

        info!(
            airport = iata,
            total = format!("{:.1}", score.score_total),
            infra = format!("{:.1}", score.score_infrastructure),
            ops = format!("{:.1}", score.score_operational),
            sentiment = format!("{:.1}", score.score_sentiment),
            velocity = format!("{:.1}", score.score_sentiment_velocity),
            "Score computed"
        );
    }
    Ok(())
}
