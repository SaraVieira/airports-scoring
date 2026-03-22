use anyhow::{Context, Result};
use rust_decimal::Decimal;
use serde::Deserialize;
use sqlx::PgPool;
use tracing::{info, warn};

use crate::models::{Airport, FetchResult};

#[derive(Debug, Deserialize)]
struct PipelineOutput {
    #[allow(dead_code)]
    airport: String,
    source: Option<String>,
    sentiment_snapshots: Vec<Snapshot>,
}

#[derive(Debug, Deserialize)]
struct Snapshot {
    snapshot_year: i16,
    snapshot_quarter: i16,
    avg_rating: Option<f64>,
    review_count: Option<i32>,
    positive_pct: Option<f64>,
    negative_pct: Option<f64>,
    neutral_pct: Option<f64>,
    score_queuing: Option<f64>,
    score_cleanliness: Option<f64>,
    score_staff: Option<f64>,
    score_food_bev: Option<f64>,
    score_wifi: Option<f64>,
    score_wayfinding: Option<f64>,
    score_transport: Option<f64>,
    commentary: Option<String>,
}

fn to_dec(v: Option<f64>) -> Option<Decimal> {
    v.and_then(|f| {
        let mut d = Decimal::from_f64_retain(f)?;
        d.rescale(2);
        Some(d)
    })
}

/// Review sources to process sentiment for.
const REVIEW_SOURCES: &[&str] = &["skytrax", "google"];

/// Run the Python sentiment ML pipeline as a subprocess.
/// Loops over each review source (skytrax, google) to process them separately,
/// producing correctly tagged sentiment_snapshots per source.
pub async fn fetch(pool: &PgPool, airport: &Airport, _full_refresh: bool) -> Result<FetchResult> {
    let iata = airport
        .iata_code
        .as_deref()
        .context("Airport has no IATA code")?;

    let python = if std::path::Path::new(".venv/bin/python3").exists() {
        ".venv/bin/python3"
    } else {
        "python3"
    };

    let mut total_records: i32 = 0;

    for &source in REVIEW_SOURCES {
        // Check if there are unprocessed reviews for this airport + source
        let unprocessed: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM reviews_raw r \
             JOIN airports a ON r.airport_id = a.id \
             WHERE a.iata_code = $1 AND r.source = $2 \
             AND (r.processed IS NULL OR r.processed = FALSE)",
        )
        .bind(iata)
        .bind(source)
        .fetch_one(pool)
        .await?;

        if unprocessed.0 == 0 {
            info!(airport = iata, source = source, "No unprocessed reviews, skipping");
            continue;
        }

        info!(
            airport = iata,
            source = source,
            unprocessed = unprocessed.0,
            "Running sentiment pipeline"
        );

        let output = tokio::process::Command::new(python)
            .arg("python/sentiment_pipeline.py")
            .arg("--airport")
            .arg(iata)
            .arg("--source")
            .arg(source)
            .output()
            .await
            .context("Failed to run sentiment_pipeline.py")?;

        if !output.stderr.is_empty() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            for line in stderr.lines() {
                if line.contains("ERROR") || line.contains("Error") {
                    warn!(airport = iata, source = source, line = %line, "Sentiment pipeline stderr");
                }
            }
        }

        if !output.status.success() {
            warn!(
                airport = iata,
                source = source,
                status = %output.status,
                "Sentiment pipeline failed for source, continuing with next"
            );
            continue;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let result: PipelineOutput = serde_json::from_str(stdout.trim())
            .context("Failed to parse sentiment pipeline JSON output")?;

        // Use source from pipeline output if available, otherwise use the loop variable
        let effective_source = result.source.as_deref().unwrap_or(source);

        info!(
            airport = iata,
            source = effective_source,
            snapshots = result.sentiment_snapshots.len(),
            "Sentiment pipeline returned data"
        );

        // Upsert snapshots into sentiment_snapshots
        let mut records: i32 = 0;

        for snap in &result.sentiment_snapshots {
            let upsert = sqlx::query(
                r#"
                INSERT INTO sentiment_snapshots (
                    airport_id, source, snapshot_year, snapshot_quarter,
                    avg_rating, review_count, positive_pct, negative_pct, neutral_pct,
                    score_queuing, score_cleanliness, score_staff,
                    score_food_bev, score_wifi, score_wayfinding, score_transport,
                    notes
                ) VALUES (
                    $1, $2, $3, $4,
                    $5, $6, $7, $8, $9,
                    $10, $11, $12,
                    $13, $14, $15, $16,
                    $17
                )
                ON CONFLICT (airport_id, source, snapshot_year, snapshot_quarter)
                DO UPDATE SET
                    avg_rating = EXCLUDED.avg_rating,
                    review_count = EXCLUDED.review_count,
                    positive_pct = EXCLUDED.positive_pct,
                    negative_pct = EXCLUDED.negative_pct,
                    neutral_pct = EXCLUDED.neutral_pct,
                    score_queuing = EXCLUDED.score_queuing,
                    score_cleanliness = EXCLUDED.score_cleanliness,
                    score_staff = EXCLUDED.score_staff,
                    score_food_bev = EXCLUDED.score_food_bev,
                    score_wifi = EXCLUDED.score_wifi,
                    score_wayfinding = EXCLUDED.score_wayfinding,
                    score_transport = EXCLUDED.score_transport,
                    notes = EXCLUDED.notes
                "#,
            )
            .bind(airport.id)
            .bind(effective_source)
            .bind(snap.snapshot_year)
            .bind(snap.snapshot_quarter)
            .bind(to_dec(snap.avg_rating))
            .bind(snap.review_count)
            .bind(to_dec(snap.positive_pct))
            .bind(to_dec(snap.negative_pct))
            .bind(to_dec(snap.neutral_pct))
            .bind(to_dec(snap.score_queuing))
            .bind(to_dec(snap.score_cleanliness))
            .bind(to_dec(snap.score_staff))
            .bind(to_dec(snap.score_food_bev))
            .bind(to_dec(snap.score_wifi))
            .bind(to_dec(snap.score_wayfinding))
            .bind(to_dec(snap.score_transport))
            .bind(&snap.commentary)
            .execute(pool)
            .await;

            match upsert {
                Ok(_) => records += 1,
                Err(e) => {
                    warn!(
                        year = snap.snapshot_year,
                        quarter = snap.snapshot_quarter,
                        source = effective_source,
                        error = %e,
                        "Failed to upsert sentiment snapshot"
                    );
                }
            }
        }

        info!(
            airport = iata,
            source = effective_source,
            records = records,
            "Sentiment snapshots upserted"
        );
        total_records += records;
    }

    Ok(FetchResult {
        records_processed: total_records,
        last_record_date: None,
    })
}
