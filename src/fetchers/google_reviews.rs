use anyhow::{Context, Result};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use tracing::{info, warn};

use crate::config::SeedAirport;
use crate::models::{Airport, FetchResult};

/// Maximum time to wait for a scrape job to complete (seconds).
const JOB_TIMEOUT_SECS: u64 = 600;

// ── Scraper API types ────────────────────────────────────────

#[derive(Debug, Serialize)]
struct ScrapeRequest {
    url: String,
    headless: bool,
    sort_by: String,
    download_images: bool,
}

#[derive(Debug, Deserialize)]
struct ScrapeResponse {
    job_id: String,
}

#[derive(Debug, Deserialize)]
struct JobStatus {
    status: String,
    reviews: Option<Vec<GoogleReview>>,
}

#[derive(Debug, Deserialize)]
struct GoogleReview {
    reviewer_name: Option<String>,
    rating: Option<i16>,
    date_iso: Option<String>,
    text: Option<String>,
    language: Option<String>,
}

// ── Helpers ──────────────────────────────────────────────────

/// Generate a deterministic synthetic source_url for dedup.
/// Format: google://{iata}/{sha256(reviewer_name|date_iso|text[..100])}
fn synthetic_source_url(iata: &str, review: &GoogleReview) -> String {
    let name = review.reviewer_name.as_deref().unwrap_or("");
    let date = review.date_iso.as_deref().unwrap_or("");
    let text = review.text.as_deref().unwrap_or("");
    let text_snippet = &text[..text.len().min(100)];

    let mut hasher = Sha256::new();
    hasher.update(format!("{}|{}|{}", name, date, text_snippet));
    let hash = hex::encode(hasher.finalize());

    format!("google://{}/{}", iata, hash)
}

/// Check if the scraper service is reachable.
async fn check_scraper_health(base_url: &str) -> bool {
    let client = reqwest::Client::new();
    match client
        .get(format!("{}/", base_url))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
    {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    }
}

// ── Main fetcher ─────────────────────────────────────────────

/// Fetch Google Maps reviews via the google-reviews-scraper-pro REST API.
///
/// Gracefully skips if:
/// - Airport has no `google_maps_url` configured in airports.json
/// - Scraper service is not running
pub async fn fetch(
    pool: &PgPool,
    airport: &Airport,
    full_refresh: bool,
    seed_airports: &[SeedAirport],
) -> Result<FetchResult> {
    let iata = airport
        .iata_code
        .as_deref()
        .context("Airport has no IATA code")?;

    // Find this airport in seed config to get google_maps_url
    let seed = seed_airports.iter().find(|s| s.iata == iata);
    let google_maps_url = seed.and_then(|s| s.google_maps_url.as_deref());

    let google_maps_url = match google_maps_url {
        Some(url) if !url.is_empty() => url,
        _ => {
            info!(airport = iata, "No google_maps_url configured, skipping Google Reviews");
            return Ok(FetchResult {
                records_processed: 0,
                last_record_date: None,
            });
        }
    };

    // Read scraper config from env
    let base_url =
        std::env::var("GOOGLE_SCRAPER_URL").unwrap_or_else(|_| "http://localhost:8000".to_string());
    let api_key = std::env::var("GOOGLE_SCRAPER_API_KEY").unwrap_or_default();

    // Health check
    if !check_scraper_health(&base_url).await {
        warn!(
            airport = iata,
            "Google Reviews scraper not reachable at {}. Skipping. \
             Start it with: bash scripts/start-google-scraper.sh",
            base_url
        );
        return Ok(FetchResult {
            records_processed: 0,
            last_record_date: None,
        });
    }

    // Determine incremental cutoff
    let last_record_date = if full_refresh {
        None
    } else {
        let last: Option<(Option<NaiveDate>,)> = sqlx::query_as(
            "SELECT last_record_date FROM pipeline_runs \
             WHERE airport_id = $1 AND source = 'google_reviews' AND status = 'success' \
             ORDER BY completed_at DESC LIMIT 1",
        )
        .bind(airport.id)
        .fetch_optional(pool)
        .await?;

        last.and_then(|(d,)| d)
    };

    info!(
        airport = iata,
        since = ?last_record_date,
        "Starting Google Reviews scrape"
    );

    // Submit scrape job
    let client = reqwest::Client::new();
    let scrape_resp = client
        .post(format!("{}/scrape", base_url))
        .header("X-API-Key", &api_key)
        .json(&ScrapeRequest {
            url: google_maps_url.to_string(),
            headless: true,
            sort_by: "newest".to_string(),
            download_images: false,
        })
        .send()
        .await
        .context("Failed to submit scrape job")?;

    if !scrape_resp.status().is_success() {
        let status = scrape_resp.status();
        let body = scrape_resp.text().await.unwrap_or_default();
        anyhow::bail!("Scraper returned {} for {}: {}", status, iata, body);
    }

    let job: ScrapeResponse = scrape_resp
        .json()
        .await
        .context("Failed to parse scrape response")?;

    info!(airport = iata, job_id = %job.job_id, "Scrape job submitted, polling...");

    // Poll for completion with exponential backoff
    let mut poll_interval = std::time::Duration::from_secs(5);
    let max_interval = std::time::Duration::from_secs(30);
    let deadline =
        tokio::time::Instant::now() + std::time::Duration::from_secs(JOB_TIMEOUT_SECS);

    let reviews = loop {
        if tokio::time::Instant::now() >= deadline {
            anyhow::bail!(
                "Google Reviews scrape job timed out after {}s for {}",
                JOB_TIMEOUT_SECS,
                iata
            );
        }

        tokio::time::sleep(poll_interval).await;
        poll_interval = (poll_interval * 2).min(max_interval);

        let status_resp = client
            .get(format!("{}/jobs/{}", base_url, job.job_id))
            .header("X-API-Key", &api_key)
            .send()
            .await
            .context("Failed to poll job status")?;

        let status: JobStatus = status_resp
            .json()
            .await
            .context("Failed to parse job status")?;

        match status.status.as_str() {
            "completed" => {
                break status.reviews.unwrap_or_default();
            }
            "failed" => {
                anyhow::bail!("Google Reviews scrape job failed for {}", iata);
            }
            _ => {
                // still pending/running
            }
        }
    };

    info!(
        airport = iata,
        total_reviews = reviews.len(),
        "Scrape job completed, processing reviews"
    );

    // Filter and insert reviews
    let mut records: i32 = 0;
    let mut latest_date: Option<NaiveDate> = None;

    for review in &reviews {
        // Parse review date
        let review_date = review
            .date_iso
            .as_ref()
            .and_then(|d| {
                // Try ISO 8601 datetime first, then plain date
                NaiveDate::parse_from_str(d, "%Y-%m-%dT%H:%M:%SZ")
                    .or_else(|_| NaiveDate::parse_from_str(d, "%Y-%m-%dT%H:%M:%S%.fZ"))
                    .or_else(|_| NaiveDate::parse_from_str(d, "%Y-%m-%d"))
                    .ok()
            });

        // Incremental: skip reviews older than last_record_date
        if let (Some(rd), Some(cutoff)) = (review_date, last_record_date) {
            if rd <= cutoff {
                continue;
            }
        }

        let source_url = synthetic_source_url(iata, review);

        // Normalize rating: Google 1-5 → 1-10 scale (multiply by 2)
        let overall_rating = review.rating.map(|r| r * 2);

        let upsert = sqlx::query(
            r#"
            INSERT INTO reviews_raw (
                airport_id, source, review_date, author,
                overall_rating, review_text, source_url
            ) VALUES (
                $1, 'google', $2, $3,
                $4, $5, $6
            )
            ON CONFLICT (source_url) DO UPDATE SET
                overall_rating = EXCLUDED.overall_rating,
                review_text = EXCLUDED.review_text
            "#,
        )
        .bind(airport.id)
        .bind(review_date)
        .bind(&review.reviewer_name)
        .bind(overall_rating)
        .bind(&review.text)
        .bind(&source_url)
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
                warn!(error = %e, "Failed to upsert Google review, skipping");
            }
        }
    }

    info!(airport = iata, records = records, "Google Reviews upserted");

    Ok(FetchResult {
        records_processed: records,
        last_record_date: latest_date,
    })
}
