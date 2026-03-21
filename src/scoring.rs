use anyhow::Result;
use sqlx::PgPool;

use crate::models::Airport;

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
    /// Free-text commentary explaining the score.
    /// Will be populated by the local ML pipeline (e.g. a local LLM
    /// summarising strengths and weaknesses).
    pub commentary: Option<String>,
}

/// Compute the composite score for a single airport.
///
/// This pulls together all available data (traffic, delays, sentiment,
/// routes, infrastructure) and produces dimension scores plus a weighted
/// total.
///
/// The `commentary` field on the returned `ScoreOutput` is left as `None`
/// here; it is filled in by a separate ML pipeline step that generates
/// natural-language summaries.
pub async fn compute_score(
    _pool: &PgPool,
    _airport: &Airport,
    _reference_year: i16,
) -> Result<ScoreOutput> {
    todo!("Implement score computation")
}

/// Persist a computed score into the airport_scores table, marking it as
/// the latest and un-marking any previous latest row.
pub async fn upsert_score(_pool: &PgPool, _score: &ScoreOutput) -> Result<()> {
    todo!("Implement score upsert")
}

/// Compute scores for all airports in the provided list and persist them.
pub async fn score_airports(
    _pool: &PgPool,
    _airports: &[Airport],
    _reference_year: i16,
) -> Result<()> {
    todo!("Implement batch scoring")
}
