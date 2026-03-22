use anyhow::{Context, Result};
use chrono::NaiveDate;
use serde::Deserialize;
use sqlx::PgPool;
use tracing::{info, warn};

use crate::models::{Airport, FetchResult};

#[derive(Debug, Deserialize)]
struct ScraperOutput {
    #[allow(dead_code)]
    airport: String,
    star_rating: Option<i16>,
    reviews: Vec<ScrapedReview>,
}

#[derive(Debug, Deserialize)]
struct ScrapedReview {
    review_date: Option<String>,
    author: Option<String>,
    author_country: Option<String>,
    overall_rating: Option<i16>,
    score_queuing: Option<i16>,
    score_cleanliness: Option<i16>,
    score_staff: Option<i16>,
    score_food_bev: Option<i16>,
    score_wifi: Option<i16>,
    score_wayfinding: Option<i16>,
    score_transport: Option<i16>,
    recommended: Option<bool>,
    verified: Option<bool>,
    trip_type: Option<String>,
    review_title: Option<String>,
    review_text: Option<String>,
    source_url: Option<String>,
}

/// Fetch Skytrax reviews by calling the Python Playwright scraper as a subprocess.
/// Reads JSON from stdout and upserts reviews into reviews_raw.
pub async fn fetch(pool: &PgPool, airport: &Airport, full_refresh: bool) -> Result<FetchResult> {
    let iata = airport
        .iata_code
        .as_deref()
        .context("Airport has no IATA code")?;

    // Determine --since date from last pipeline run
    let since = if full_refresh {
        "2010-01-01".to_string()
    } else {
        let last: Option<(Option<NaiveDate>,)> = sqlx::query_as(
            "SELECT last_record_date FROM pipeline_runs \
             WHERE airport_id = $1 AND source = 'skytrax' AND status = 'success' \
             ORDER BY completed_at DESC LIMIT 1",
        )
        .bind(airport.id)
        .fetch_optional(pool)
        .await?;

        match last {
            Some((Some(d),)) => d.to_string(),
            _ => "2020-01-01".to_string(),
        }
    };

    // Find python — prefer .venv
    let python = if std::path::Path::new(".venv/bin/python3").exists() {
        ".venv/bin/python3"
    } else {
        "python3"
    };

    info!(airport = iata, since = %since, "Running Skytrax scraper");

    // Default to 50 pages (~500 reviews at 10/page). Use --full-refresh for more.
    let max_pages = if full_refresh { "200" } else { "50" };

    let output = tokio::process::Command::new(python)
        .arg("python/skytrax_scraper.py")
        .arg("--airport")
        .arg(iata)
        .arg("--since")
        .arg(&since)
        .arg("--max-pages")
        .arg(max_pages)
        .output()
        .await
        .context("Failed to run skytrax_scraper.py")?;

    if !output.stderr.is_empty() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Scraper logs to stderr — only warn if it looks like an error
        for line in stderr.lines() {
            if line.contains("ERROR") || line.contains("Error") {
                warn!(airport = iata, line = %line, "Skytrax scraper stderr");
            }
        }
    }

    if !output.status.success() {
        anyhow::bail!(
            "Skytrax scraper exited with status {} for {}",
            output.status,
            iata
        );
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: ScraperOutput = serde_json::from_str(stdout.trim())
        .context("Failed to parse Skytrax scraper JSON output")?;

    info!(
        airport = iata,
        reviews = result.reviews.len(),
        star_rating = result.star_rating,
        "Skytrax scraper returned data"
    );

    // Upsert reviews into reviews_raw
    let mut records: i32 = 0;
    let mut latest_date: Option<NaiveDate> = None;

    for review in &result.reviews {
        let review_date = review
            .review_date
            .as_ref()
            .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok());

        let upsert = sqlx::query(
            r#"
            INSERT INTO reviews_raw (
                airport_id, source, review_date, author, author_country,
                overall_rating, score_queuing, score_cleanliness, score_staff,
                score_food_bev, score_wifi, score_wayfinding, score_transport,
                recommended, verified, trip_type, review_title, review_text,
                source_url
            ) VALUES (
                $1, 'skytrax', $2, $3, $4,
                $5, $6, $7, $8,
                $9, $10, $11, $12,
                $13, $14, $15, $16, $17,
                $18
            )
            ON CONFLICT (source_url) DO UPDATE SET
                overall_rating = EXCLUDED.overall_rating,
                review_text = EXCLUDED.review_text
            "#,
        )
        .bind(airport.id)
        .bind(review_date)
        .bind(&review.author)
        .bind(&review.author_country)
        .bind(review.overall_rating)
        .bind(review.score_queuing)
        .bind(review.score_cleanliness)
        .bind(review.score_staff)
        .bind(review.score_food_bev)
        .bind(review.score_wifi)
        .bind(review.score_wayfinding)
        .bind(review.score_transport)
        .bind(review.recommended)
        .bind(review.verified)
        .bind(&review.trip_type)
        .bind(&review.review_title)
        .bind(&review.review_text)
        .bind(&review.source_url)
        .execute(pool)
        .await;

        match upsert {
            Ok(_) => {
                records += 1;
                if let Some(d) = review_date {
                    latest_date = Some(match latest_date {
                        Some(prev) if d > prev => d,
                        Some(prev) => prev,
                        None => d,
                    });
                }
            }
            Err(e) => {
                warn!(error = %e, "Failed to upsert review, skipping");
            }
        }
    }

    info!(airport = iata, records = records, "Skytrax reviews upserted");

    // Store star_rating in the most recent sentiment snapshot for this airport
    if let Some(stars) = result.star_rating {
        let updated = sqlx::query(
            r#"
            UPDATE sentiment_snapshots
            SET skytrax_stars = $1
            WHERE id = (
                SELECT id FROM sentiment_snapshots
                WHERE airport_id = $2 AND source = 'skytrax'
                ORDER BY snapshot_year DESC, snapshot_quarter DESC
                LIMIT 1
            )
            "#,
        )
        .bind(stars)
        .bind(airport.id)
        .execute(pool)
        .await;

        match updated {
            Ok(r) => {
                if r.rows_affected() > 0 {
                    info!(airport = iata, stars = stars, "Updated skytrax_stars on latest sentiment snapshot");
                } else {
                    warn!(airport = iata, stars = stars, "No sentiment snapshot found to store skytrax_stars — run sentiment pipeline first");
                }
            }
            Err(e) => {
                warn!(airport = iata, error = %e, "Failed to update skytrax_stars");
            }
        }
    }

    Ok(FetchResult {
        records_processed: records,
        last_record_date: latest_date,
    })
}
