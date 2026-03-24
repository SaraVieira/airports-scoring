mod types;

use anyhow::{Context, Result};
use chrono::NaiveDate;
use sqlx::PgPool;
use tracing::{info, warn};

use crate::config::SeedAirport;
use crate::models::{Airport, FetchResult};
use types::*;

/// Poll interval bounds for job status checks.
const POLL_MIN_INTERVAL_SECS: u64 = 5;
const POLL_MAX_INTERVAL_SECS: u64 = 30;

/// Page size when fetching reviews from the scraper API.
const REVIEWS_PAGE_SIZE: u32 = 200;

/// Fetch Google Maps reviews via the google-reviews-scraper-pro REST API.
///
/// Flow:
/// 1. Submit scrape job and poll until complete
/// 2. Find the place_id matching our URL via GET /places
/// 3. Paginate GET /reviews/{place_id} to fetch all reviews
/// 4. Upsert into reviews_raw
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

    // ── Step 1: Submit scrape job and wait ────────────────────

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

    // Poll for completion
    let mut poll_interval = std::time::Duration::from_secs(POLL_MIN_INTERVAL_SECS);
    let max_interval = std::time::Duration::from_secs(POLL_MAX_INTERVAL_SECS);
    let start = tokio::time::Instant::now();

    loop {
        tokio::time::sleep(poll_interval).await;
        poll_interval = (poll_interval * 2).min(max_interval);

        let elapsed = start.elapsed().as_secs();
        if elapsed % 60 < POLL_MAX_INTERVAL_SECS {
            info!(airport = iata, elapsed_secs = elapsed, "Still waiting for scraper...");
        }

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
            "completed" => break,
            "failed" => {
                anyhow::bail!("Google Reviews scrape job failed for {}", iata);
            }
            _ => {}
        }
    }

    // ── Step 2: Find place_id matching our URL ───────────────

    let places_resp: Vec<Place> = client
        .get(format!("{}/places", base_url))
        .header("X-API-Key", &api_key)
        .send()
        .await
        .context("Failed to fetch places")?
        .json()
        .await
        .context("Failed to parse places")?;

    let place = places_resp
        .iter()
        .find(|p| {
            p.original_url
                .as_deref()
                .map_or(false, |u| u == google_maps_url)
        })
        .context(format!(
            "No place found matching URL {} for {}",
            google_maps_url, iata
        ))?;

    info!(
        airport = iata,
        place_id = %place.place_id,
        "Found place, fetching reviews..."
    );

    // ── Step 3: Paginate reviews ─────────────────────────────

    let mut records: i32 = 0;
    let mut latest_date: Option<NaiveDate> = None;
    let mut offset: u32 = 0;

    loop {
        let page: ReviewsPage = client
            .get(format!(
                "{}/reviews/{}?limit={}&offset={}",
                base_url, place.place_id, REVIEWS_PAGE_SIZE, offset
            ))
            .header("X-API-Key", &api_key)
            .send()
            .await
            .context("Failed to fetch reviews page")?
            .json()
            .await
            .context("Failed to parse reviews page")?;

        if page.reviews.is_empty() {
            break;
        }

        info!(
            airport = iata,
            offset = offset,
            page_size = page.reviews.len(),
            total = page.total,
            "Processing reviews page"
        );

        for review in &page.reviews {
            let review_date = review
                .review_date
                .as_deref()
                .and_then(parse_review_date);

            // Incremental: skip reviews older than last_record_date
            if let (Some(rd), Some(cutoff)) = (review_date, last_record_date) {
                if rd <= cutoff {
                    continue;
                }
            }

            // Use review_id as source_url for dedup (it's unique per review)
            let source_url = format!("google://{}/{}", iata, review.review_id);

            // Normalize rating: Google 1-5 → 1-10 scale (multiply by 2)
            let overall_rating = review.rating.map(|r| (r * 2.0).round() as i16);

            let review_text = review.text();

            let upsert = sqlx::query(
                r#"
                INSERT INTO reviews_raw (
                    airport_id, source, review_date,
                    overall_rating, review_text, source_url
                ) VALUES (
                    $1, 'google', $2,
                    $3, $4, $5
                )
                ON CONFLICT (source_url) DO UPDATE SET
                    overall_rating = EXCLUDED.overall_rating,
                    review_text = EXCLUDED.review_text
                "#,
            )
            .bind(airport.id)
            .bind(review_date)
            .bind(overall_rating)
            .bind(review_text)
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

        offset += page.reviews.len() as u32;
        if offset >= page.total {
            break;
        }
    }

    info!(airport = iata, records = records, "Google Reviews upserted");

    Ok(FetchResult {
        records_processed: records,
        last_record_date: latest_date,
    })
}
