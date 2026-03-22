use anyhow::Result;
use rust_decimal::prelude::*;
use sqlx::PgPool;

use super::{
    ScoreOutput,
    W_CONNECTIVITY, W_INFRASTRUCTURE, W_OPERATIONAL,
    W_OPERATOR, W_SENTIMENT, W_SENTIMENT_VELOCITY,
};

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
    .bind(0_i16) // 0 = all-time composite score
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
