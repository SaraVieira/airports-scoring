mod data;
mod dimensions;
mod persist;

#[cfg(test)]
mod tests;

use anyhow::Result;
use chrono::Datelike;
use sqlx::PgPool;
use tracing::info;

use crate::models::Airport;
use data::gather_scoring_data;
use dimensions::*;
pub use persist::upsert_score;

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

/// Compute the all-time composite score for a single airport.
/// Uses weighted averages across all years of data, with recency weighting.
pub async fn compute_score(
    pool: &PgPool,
    airport: &Airport,
) -> Result<ScoreOutput> {
    let data = gather_scoring_data(pool, airport).await?;

    let current_year = chrono::Utc::now().naive_utc().date().year() as i16;
    let infra = score_infrastructure(&data, current_year);
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

/// Compute and persist all-time scores for all airports in the list.
pub async fn score_airports(
    pool: &PgPool,
    airports: &[Airport],
) -> Result<()> {
    for airport in airports {
        let iata = airport.iata_code.as_deref().unwrap_or("???");
        info!(airport = iata, "Computing all-time score");

        let score = compute_score(pool, airport).await?;
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
