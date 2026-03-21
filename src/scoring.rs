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
    // Infrastructure
    runway_count: i64,
    max_runway_length_ft: Option<i32>,
    annual_capacity_m: Option<f64>,
    annual_pax_latest_m: Option<f64>,
    opened_year: Option<i16>,
    last_major_reno: Option<i16>,
    // Operational
    avg_delay_pct: Option<f64>,
    avg_cancellation_pct: Option<f64>,
    avg_delay_minutes: Option<f64>,
    delay_airport_pct: Option<f64>,
    taxi_out_additional_min: Option<f64>,
    // Sentiment
    latest_avg_rating: Option<f64>,
    review_count: Option<i32>,
    sub_score_count: i32,
    sub_score_sum: f64,
    // Velocity (4-quarter rolling averages)
    avg_rating_last_4q: Option<f64>,
    avg_rating_prior_4q: Option<f64>,
    // Connectivity
    destination_count: i64,
    airline_count: i64,
    international_pax: Option<i64>,
    total_pax: Option<i64>,
    // Operator portfolio
    operator_avg_sentiment: Option<f64>,
    operator_avg_operational: Option<f64>,
    operator_airport_count: i64,
}

/// Compute the composite score for a single airport.
pub async fn compute_score(
    pool: &PgPool,
    airport: &Airport,
    reference_year: i16,
) -> Result<ScoreOutput> {
    let data = gather_scoring_data(pool, airport, reference_year).await?;

    let infra = score_infrastructure(&data, reference_year);
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
        commentary: None,
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

    // Taxi-out additional time from Eurocontrol (stored in operational_stats notes or as a
    // separate metric). For now we approximate from avg_delay_minutes * a factor if not
    // available directly. In future, ASMA/taxi-out data from Eurocontrol PRU can be stored
    // in a dedicated column.
    // TODO: Add taxi_out_additional_min column to operational_stats when Eurocontrol PRU
    // ASMA/taxi-out CSV parsing is implemented.
    let taxi_out: Option<f64> = None;

    // Latest 4 quarters of sentiment for rolling average.
    // Annual snapshots (snapshot_quarter IS NULL) are treated as belonging to
    // (snapshot_year, Q4) so they anchor to year-end.
    let last_4q: Option<(Option<Decimal>, Option<i32>)> = sqlx::query_as(
        "SELECT AVG(avg_rating), SUM(review_count)::INT \
         FROM sentiment_snapshots \
         WHERE airport_id = $1 \
         AND (snapshot_year * 4 + COALESCE(snapshot_quarter, 4)) > ($2 * 4 - 4) \
         AND (snapshot_year * 4 + COALESCE(snapshot_quarter, 4)) <= ($2 * 4)",
    )
    .bind(airport.id)
    .bind(reference_year as i32)
    .fetch_optional(pool)
    .await?;

    // Prior 4 quarters of sentiment (year before last 4Q).
    let prior_4q: Option<(Option<Decimal>,)> = sqlx::query_as(
        "SELECT AVG(avg_rating) \
         FROM sentiment_snapshots \
         WHERE airport_id = $1 \
         AND (snapshot_year * 4 + COALESCE(snapshot_quarter, 4)) > ($2 * 4 - 8) \
         AND (snapshot_year * 4 + COALESCE(snapshot_quarter, 4)) <= ($2 * 4 - 4)",
    )
    .bind(airport.id)
    .bind(reference_year as i32)
    .fetch_optional(pool)
    .await?;

    // Latest sentiment snapshot for sub-scores and avg_rating
    let latest_sentiment: Option<(
        Option<Decimal>, Option<i32>,
        Option<Decimal>, Option<Decimal>, Option<Decimal>, Option<Decimal>,
        Option<Decimal>, Option<Decimal>, Option<Decimal>, Option<Decimal>,
    )> = sqlx::query_as(
        "SELECT avg_rating, review_count, \
         score_queuing, score_cleanliness, score_staff, score_food_bev, \
         score_shopping, score_wifi, score_wayfinding, score_transport \
         FROM sentiment_snapshots \
         WHERE airport_id = $1 \
         ORDER BY snapshot_year DESC, snapshot_quarter DESC NULLS LAST LIMIT 1",
    )
    .bind(airport.id)
    .fetch_optional(pool)
    .await?;

    // Compute sub-score sum and count from the latest snapshot
    let (sub_score_sum, sub_score_count, latest_avg_rating, review_count) =
        if let Some(ref s) = latest_sentiment {
            let scores = [s.2, s.3, s.4, s.5, s.6, s.7, s.8, s.9];
            let mut sum = 0.0_f64;
            let mut count = 0_i32;
            for score in &scores {
                if let Some(ref d) = score {
                    if let Some(f) = d.to_f64() {
                        sum += f;
                        count += 1;
                    }
                }
            }
            (
                sum,
                count,
                s.0.as_ref().and_then(|d| d.to_f64()),
                s.1,
            )
        } else {
            (0.0, 0, None, None)
        };

    // Route connectivity — COALESCE handles openflights routes that only have IATA columns.
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

    // Operator portfolio: average sentiment + operational scores across other airports
    let operator_portfolio: Option<(Option<Decimal>, Option<Decimal>, i64)> = sqlx::query_as(
        "SELECT AVG(s.score_sentiment), AVG(s.score_operational), COUNT(DISTINCT a.id) \
         FROM airport_scores s \
         JOIN airports a ON a.id = s.airport_id \
         WHERE a.operator_id = $1 AND s.is_latest = TRUE",
    )
    .bind(airport.operator_id)
    .fetch_optional(pool)
    .await?;

    Ok(ScoringData {
        runway_count: runway_stats.0,
        max_runway_length_ft: runway_stats.1,
        annual_capacity_m: airport.annual_capacity_m.as_ref().and_then(|d| d.to_f64()),
        annual_pax_latest_m: airport.annual_pax_latest_m.as_ref().and_then(|d| d.to_f64()),
        opened_year: airport.opened_year,
        last_major_reno: airport.last_major_reno,
        avg_delay_pct: ops.0.and_then(|d| d.to_f64()),
        avg_cancellation_pct: ops.1.and_then(|d| d.to_f64()),
        avg_delay_minutes: ops.2.and_then(|d| d.to_f64()),
        delay_airport_pct: ops.3.and_then(|d| d.to_f64()),
        taxi_out_additional_min: taxi_out,
        latest_avg_rating,
        review_count,
        sub_score_count,
        sub_score_sum,
        avg_rating_last_4q: last_4q
            .as_ref()
            .and_then(|s| s.0.as_ref())
            .and_then(|d| d.to_f64()),
        avg_rating_prior_4q: prior_4q
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
    })
}

/// Infrastructure score (weight: 15%)
///
/// runway_score     = LEAST(runway_count / 3.0, 1.0) * 100
/// length_score     = LEAST(longest_runway_ft / 13000.0, 1.0) * 100
/// age_score        = renovation-aware aging formula
/// capacity_score   = LEAST((annual_pax_latest / capacity) * 100, 100)
///
/// score = runway_score * 0.35 + length_score * 0.25 + age_score * 0.25 + capacity_score * 0.15
fn score_infrastructure(data: &ScoringData, reference_year: i16) -> f64 {
    let runway_score = (data.runway_count as f64 / 3.0).min(1.0) * 100.0;

    let length_score = data
        .max_runway_length_ft
        .map(|len| (len as f64 / 13000.0).min(1.0) * 100.0)
        .unwrap_or(0.0);

    let age_score = if let Some(reno) = data.last_major_reno {
        // Recently renovated: penalty based on years since renovation
        (100.0 - (reference_year as f64 - reno as f64) * 3.0).max(0.0)
    } else if let Some(opened) = data.opened_year {
        // No renovation: slower penalty based on age since opening
        (100.0 - (reference_year as f64 - opened as f64) * 1.5).max(0.0)
    } else {
        50.0 // no data = neutral
    };

    let capacity_score = match (data.annual_pax_latest_m, data.annual_capacity_m) {
        (Some(pax), Some(cap)) if cap > 0.0 => ((pax / cap) * 100.0).min(100.0),
        _ => 50.0,
    };

    let score = runway_score * 0.35 + length_score * 0.25 + age_score * 0.25 + capacity_score * 0.15;
    score.clamp(0.0, 100.0)
}

/// Operational score (weight: 25%)
///
/// delay_score        = GREATEST(0, 100 - (delay_pct * 2.5))
/// avg_delay_score    = GREATEST(0, 100 - (avg_delay_minutes * 3))
/// cancellation_score = GREATEST(0, 100 - (cancellation_pct * 10))
/// taxi_score         = GREATEST(0, 100 - (taxi_out_additional_min * 10))
///
/// attribution_modifier = 1.0 - (airport_delay_pct * 0.003)
///
/// score = (delay * 0.35 + avg_delay * 0.25 + cancel * 0.20 + taxi * 0.20) * modifier
fn score_operational(data: &ScoringData) -> f64 {
    let delay_score = data
        .avg_delay_pct
        .map(|d| (100.0 - d * 2.5).max(0.0))
        .unwrap_or(70.0); // no data = slightly below neutral

    let avg_delay_score = data
        .avg_delay_minutes
        .map(|d| (100.0 - d * 3.0).max(0.0))
        .unwrap_or(70.0);

    let cancellation_score = data
        .avg_cancellation_pct
        .map(|d| (100.0 - d * 10.0).max(0.0))
        .unwrap_or(80.0);

    let taxi_score = data
        .taxi_out_additional_min
        .map(|d| (100.0 - d * 10.0).max(0.0))
        .unwrap_or(70.0);

    let attribution_modifier = data
        .delay_airport_pct
        .map(|d| 1.0 - d * 0.003)
        .unwrap_or(1.0);

    let raw = delay_score * 0.35
        + avg_delay_score * 0.25
        + cancellation_score * 0.20
        + taxi_score * 0.20;

    (raw * attribution_modifier).clamp(0.0, 100.0)
}

/// Sentiment score (weight: 25%)
///
/// rating_score    = ((avg_rating - 1) / 9.0) * 100   -- normalise 1-10 to 0-100
/// sub_score_avg   = ((sum_of_non_null_sub_scores - count) / (count * 4.0)) * 100
/// confidence      = LEAST(review_count / 500.0, 1.0)
///
/// score = (rating_score * 0.6 + sub_score_avg * 0.4) * confidence
///       + rating_score * (1 - confidence) * 0.6
fn score_sentiment(data: &ScoringData) -> f64 {
    match data.latest_avg_rating {
        Some(rating) => {
            // avg_rating is on 0-5 scale in sentiment_snapshots, but reviews are 1-10.
            // The HANDOFF formula uses 1-10, so we convert: multiply by 2 to get 0-10 range.
            let rating_10 = rating * 2.0;
            let rating_score = ((rating_10 - 1.0) / 9.0) * 100.0;

            let sub_score_avg = if data.sub_score_count > 0 {
                // Sub-scores are 0-5 scale. Normalise: (sum - count) / (count * 4) * 100
                let count = data.sub_score_count as f64;
                ((data.sub_score_sum - count) / (count * 4.0)) * 100.0
            } else {
                rating_score // fallback to rating if no sub-scores
            };

            let confidence = data
                .review_count
                .map(|c| (c as f64 / 500.0).min(1.0))
                .unwrap_or(0.0);

            let score = (rating_score * 0.6 + sub_score_avg * 0.4) * confidence
                + rating_score * (1.0 - confidence) * 0.6;

            score.clamp(0.0, 100.0)
        }
        None => 50.0,
    }
}

/// Sentiment velocity score (weight: 15%)
///
/// yoy_delta = avg_rating_last_4_quarters - avg_rating_prior_4_quarters  (on 0-5 scale)
/// score = LEAST(100, GREATEST(0, 50 + (yoy_delta * 20)))
///
/// 50 = flat, 70 = +1.0 rating improvement YoY, 30 = -1.0 YoY decline
fn score_sentiment_velocity(data: &ScoringData) -> f64 {
    match (data.avg_rating_last_4q, data.avg_rating_prior_4q) {
        (Some(last), Some(prior)) => {
            let yoy_delta = last - prior;
            (50.0 + yoy_delta * 20.0).clamp(0.0, 100.0)
        }
        _ => 50.0, // no trend data = flat
    }
}

/// Connectivity score (weight: 10%)
///
/// destination_score = LEAST(unique_destination_count / 100.0, 1.0) * 100
/// airline_score     = LEAST(airline_count / 30.0, 1.0) * 100
/// intl_ratio_score  = (international_pax / total_pax) * 100
///
/// score = destination_score * 0.4 + airline_score * 0.3 + intl_ratio_score * 0.3
fn score_connectivity(data: &ScoringData) -> f64 {
    let destination_score = (data.destination_count as f64 / 100.0).min(1.0) * 100.0;
    let airline_score = (data.airline_count as f64 / 30.0).min(1.0) * 100.0;

    let intl_ratio_score = match (data.international_pax, data.total_pax) {
        (Some(intl), Some(total)) if total > 0 => (intl as f64 / total as f64) * 100.0,
        _ => 50.0, // no data = assume half international
    };

    let score = destination_score * 0.4 + airline_score * 0.3 + intl_ratio_score * 0.3;
    score.clamp(0.0, 100.0)
}

/// Operator score (weight: 10%)
///
/// Average of sentiment + operational scores across all airports
/// the same operator manages in the dataset.
/// If only 1 airport for this operator, weight 50% with neutral baseline of 50.
fn score_operator(data: &ScoringData) -> f64 {
    match (data.operator_avg_sentiment, data.operator_avg_operational) {
        (Some(sentiment), Some(operational)) => {
            let portfolio_avg = (sentiment + operational) / 2.0;
            if data.operator_airport_count <= 1 {
                // Single airport: blend 50/50 with neutral baseline
                (portfolio_avg * 0.5) + (50.0 * 0.5)
            } else {
                portfolio_avg
            }
        }
        _ => 50.0,
    }
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
            connectivity = format!("{:.1}", score.score_connectivity),
            operator = format!("{:.1}", score.score_operator),
            "Score computed"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_data() -> ScoringData {
        ScoringData {
            runway_count: 0,
            max_runway_length_ft: None,
            annual_capacity_m: None,
            annual_pax_latest_m: None,
            opened_year: None,
            last_major_reno: None,
            avg_delay_pct: None,
            avg_cancellation_pct: None,
            avg_delay_minutes: None,
            delay_airport_pct: None,
            taxi_out_additional_min: None,
            latest_avg_rating: None,
            review_count: None,
            sub_score_count: 0,
            sub_score_sum: 0.0,
            avg_rating_last_4q: None,
            avg_rating_prior_4q: None,
            destination_count: 0,
            airline_count: 0,
            international_pax: None,
            total_pax: None,
            operator_avg_sentiment: None,
            operator_avg_operational: None,
            operator_airport_count: 0,
        }
    }

    #[test]
    fn infrastructure_large_hub() {
        let data = ScoringData {
            runway_count: 4,
            max_runway_length_ft: Some(13000),
            annual_capacity_m: Some(80.0),
            annual_pax_latest_m: Some(70.0),
            last_major_reno: Some(2020),
            ..empty_data()
        };
        let score = score_infrastructure(&data, 2024);
        // runway: min(4/3,1)*100 = 100 * 0.35 = 35
        // length: min(13000/13000,1)*100 = 100 * 0.25 = 25
        // age: 100 - (2024-2020)*3 = 88 * 0.25 = 22
        // capacity: (70/80)*100 = 87.5 * 0.15 = 13.125
        // total = 95.125
        assert!((score - 95.125).abs() < 0.01, "got {}", score);
    }

    #[test]
    fn infrastructure_single_runway_no_data() {
        let data = ScoringData {
            runway_count: 1,
            ..empty_data()
        };
        let score = score_infrastructure(&data, 2024);
        // runway: (1/3)*100 = 33.3 * 0.35 = 11.67
        // length: 0 * 0.25 = 0
        // age: 50 * 0.25 = 12.5
        // capacity: 50 * 0.15 = 7.5
        // total ~ 31.67
        assert!((score - 31.67).abs() < 0.1, "got {}", score);
    }

    #[test]
    fn infrastructure_old_unrenovated_airport() {
        let data = ScoringData {
            runway_count: 2,
            max_runway_length_ft: Some(10000),
            opened_year: Some(1950),
            ..empty_data()
        };
        let score = score_infrastructure(&data, 2024);
        // runway: min(2/3,1)*100 = 66.7 * 0.35 = 23.3
        // length: min(10000/13000,1)*100 = 76.9 * 0.25 = 19.2
        // age: 100 - 74*1.5 = 0 (clamped) * 0.25 = 0
        // capacity: 50 * 0.15 = 7.5
        // total ~ 50.0
        // Age penalty bottoms out, but runways+length keep score around 50.
        assert!(score < 55.0, "old airport should not score high, got {}", score);
        assert!(score > 40.0, "airport has decent runways, got {}", score);
    }

    #[test]
    fn operational_perfect() {
        let data = ScoringData {
            avg_delay_pct: Some(0.0),
            avg_cancellation_pct: Some(0.0),
            avg_delay_minutes: Some(0.0),
            delay_airport_pct: Some(0.0),
            taxi_out_additional_min: Some(0.0),
            ..empty_data()
        };
        let score = score_operational(&data);
        assert!((score - 100.0).abs() < 0.01, "got {}", score);
    }

    #[test]
    fn operational_high_delays() {
        let data = ScoringData {
            avg_delay_pct: Some(40.0),          // 100 - 100 = 0
            avg_cancellation_pct: Some(5.0),    // 100 - 50 = 50
            avg_delay_minutes: Some(30.0),      // 100 - 90 = 10
            delay_airport_pct: Some(50.0),      // modifier: 1 - 0.15 = 0.85
            taxi_out_additional_min: Some(5.0), // 100 - 50 = 50
            ..empty_data()
        };
        let score = score_operational(&data);
        // raw = 0*0.35 + 10*0.25 + 50*0.20 + 50*0.20 = 22.5
        // * 0.85 = 19.125
        assert!((score - 19.125).abs() < 0.01, "got {}", score);
    }

    #[test]
    fn operational_no_data_defaults() {
        let data = empty_data();
        let score = score_operational(&data);
        // 70*0.35 + 70*0.25 + 80*0.20 + 70*0.20 = 72
        assert!((score - 72.0).abs() < 0.01, "got {}", score);
    }

    #[test]
    fn sentiment_high_rating_high_confidence() {
        let data = ScoringData {
            latest_avg_rating: Some(4.5), // 0-5 -> 9.0 on 0-10 -> (9-1)/9*100 = 88.89
            review_count: Some(1000),     // confidence = 1.0
            sub_score_count: 4,
            sub_score_sum: 18.0,          // (18-4)/(4*4)*100 = 87.5
            ..empty_data()
        };
        let score = score_sentiment(&data);
        // (88.89 * 0.6 + 87.5 * 0.4) * 1.0 + 0 = 88.33
        assert!(score > 85.0 && score < 92.0, "got {}", score);
    }

    #[test]
    fn sentiment_no_data_returns_neutral() {
        let data = empty_data();
        let score = score_sentiment(&data);
        assert!((score - 50.0).abs() < 0.01, "got {}", score);
    }

    #[test]
    fn sentiment_low_confidence() {
        let data = ScoringData {
            latest_avg_rating: Some(4.0), // -> 8.0 -> (8-1)/9*100 = 77.78
            review_count: Some(50),       // confidence = 0.1
            sub_score_count: 0,
            ..empty_data()
        };
        let score = score_sentiment(&data);
        // (77.78*0.6 + 77.78*0.4)*0.1 + 77.78*0.9*0.6 = 7.78 + 42.0 = 49.8
        assert!(score > 45.0 && score < 55.0, "low confidence should temper, got {}", score);
    }

    #[test]
    fn velocity_improving() {
        let data = ScoringData {
            avg_rating_last_4q: Some(4.0),
            avg_rating_prior_4q: Some(3.0),
            ..empty_data()
        };
        let score = score_sentiment_velocity(&data);
        assert!((score - 70.0).abs() < 0.01, "got {}", score);
    }

    #[test]
    fn velocity_declining() {
        let data = ScoringData {
            avg_rating_last_4q: Some(2.5),
            avg_rating_prior_4q: Some(4.0),
            ..empty_data()
        };
        let score = score_sentiment_velocity(&data);
        assert!((score - 20.0).abs() < 0.01, "got {}", score);
    }

    #[test]
    fn velocity_no_data_returns_flat() {
        let data = empty_data();
        let score = score_sentiment_velocity(&data);
        assert!((score - 50.0).abs() < 0.01, "got {}", score);
    }

    #[test]
    fn connectivity_large_hub() {
        let data = ScoringData {
            destination_count: 200,
            airline_count: 50,
            international_pax: Some(60_000_000),
            total_pax: Some(80_000_000),
            ..empty_data()
        };
        let score = score_connectivity(&data);
        // 100*0.4 + 100*0.3 + 75*0.3 = 92.5
        assert!((score - 92.5).abs() < 0.01, "got {}", score);
    }

    #[test]
    fn connectivity_small_airport() {
        let data = ScoringData {
            destination_count: 10,
            airline_count: 3,
            ..empty_data()
        };
        let score = score_connectivity(&data);
        // 10*0.4 + 10*0.3 + 50*0.3 = 22
        assert!((score - 22.0).abs() < 0.01, "got {}", score);
    }

    #[test]
    fn operator_multi_airport() {
        let data = ScoringData {
            operator_avg_sentiment: Some(75.0),
            operator_avg_operational: Some(85.0),
            operator_airport_count: 5,
            ..empty_data()
        };
        let score = score_operator(&data);
        assert!((score - 80.0).abs() < 0.01, "got {}", score);
    }

    #[test]
    fn operator_single_airport_blending() {
        let data = ScoringData {
            operator_avg_sentiment: Some(90.0),
            operator_avg_operational: Some(80.0),
            operator_airport_count: 1,
            ..empty_data()
        };
        let score = score_operator(&data);
        // (90+80)/2 = 85, blended: 85*0.5 + 50*0.5 = 67.5
        assert!((score - 67.5).abs() < 0.01, "got {}", score);
    }

    #[test]
    fn operator_no_data_returns_neutral() {
        let data = empty_data();
        let score = score_operator(&data);
        assert!((score - 50.0).abs() < 0.01, "got {}", score);
    }

    #[test]
    fn all_scores_clamped_0_to_100() {
        let extreme = ScoringData {
            runway_count: 100,
            max_runway_length_ft: Some(99999),
            annual_capacity_m: Some(0.001),
            annual_pax_latest_m: Some(999.0),
            opened_year: Some(1800),
            last_major_reno: None,
            avg_delay_pct: Some(100.0),
            avg_cancellation_pct: Some(100.0),
            avg_delay_minutes: Some(999.0),
            delay_airport_pct: Some(100.0),
            taxi_out_additional_min: Some(999.0),
            latest_avg_rating: Some(5.0),
            review_count: Some(99999),
            sub_score_count: 8,
            sub_score_sum: 40.0,
            avg_rating_last_4q: Some(5.0),
            avg_rating_prior_4q: Some(0.0),
            destination_count: 9999,
            airline_count: 9999,
            international_pax: Some(999_999_999),
            total_pax: Some(1),
            operator_avg_sentiment: Some(100.0),
            operator_avg_operational: Some(100.0),
            operator_airport_count: 100,
        };
        for (name, val) in [
            ("infra", score_infrastructure(&extreme, 2024)),
            ("ops", score_operational(&extreme)),
            ("sent", score_sentiment(&extreme)),
            ("vel", score_sentiment_velocity(&extreme)),
            ("conn", score_connectivity(&extreme)),
            ("oper", score_operator(&extreme)),
        ] {
            assert!(val >= 0.0 && val <= 100.0, "{} = {} out of bounds", name, val);
        }
    }
}
