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

/// Maximum time to wait for a scraper job before giving up (10 minutes).
const MAX_POLL_DURATION_SECS: u64 = 600;

/// Maximum reviews to fetch per airport from the scraper API.
const MAX_REVIEWS_PER_AIRPORT: u32 = 1500;

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
/// - Airport has no `google_maps_url` configured in supported_airports
/// - Scraper service is not running
pub async fn fetch(
    pool: &PgPool,
    airport: &Airport,
    full_refresh: bool,
    seed_airports: &[SeedAirport],
) -> Result<FetchResult> {
    fetch_with_url(pool, airport, full_refresh, seed_airports, None).await
}

/// Fetch with an explicit scraper URL (used by the pool-based job runner).
pub async fn fetch_with_url(
    pool: &PgPool,
    airport: &Airport,
    full_refresh: bool,
    seed_airports: &[SeedAirport],
    scraper_url: Option<&str>,
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

    // Read scraper config from param or env
    let base_url = scraper_url
        .map(|s| s.to_string())
        .unwrap_or_else(|| std::env::var("GOOGLE_SCRAPER_URL").unwrap_or_else(|_| "http://localhost:8000".to_string()));
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

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .connect_timeout(std::time::Duration::from_secs(30))
        .build()
        .context("Failed to build HTTP client")?;
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

    info!(airport = iata, job_id = %job.job_id, "Scrape job submitted, polling and ingesting reviews as they arrive...");

    // Poll for completion, ingesting reviews on each poll cycle.
    let mut poll_interval = std::time::Duration::from_secs(POLL_MIN_INTERVAL_SECS);
    let max_interval = std::time::Duration::from_secs(POLL_MAX_INTERVAL_SECS);
    let start = tokio::time::Instant::now();
    let mut consecutive_errors = 0u32;
    const MAX_POLL_ERRORS: u32 = 10;
    // Track how many reviews we've already ingested so we only process new ones.
    let mut ingested_offset: u32 = 0;

    // We need the place_id to fetch reviews. Try to find it early.
    let mut place_id: Option<String> = None;
    let mut timed_out = false;

    loop {
        tokio::time::sleep(poll_interval).await;
        poll_interval = (poll_interval * 2).min(max_interval);

        let elapsed = start.elapsed().as_secs();

        // Bail if we've been polling too long
        if elapsed > MAX_POLL_DURATION_SECS {
            warn!(airport = iata, elapsed_secs = elapsed, ingested = ingested_offset,
                "Scraper polling timed out after {}s, ingesting what we have", MAX_POLL_DURATION_SECS);
            timed_out = true;
            break;
        }

        info!(airport = iata, elapsed_secs = elapsed, ingested = ingested_offset,
            "Polling scraper for {} reviews...", iata);

        // Check job status
        let status_resp = match client
            .get(format!("{}/jobs/{}", base_url, job.job_id))
            .header("X-API-Key", &api_key)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                consecutive_errors += 1;
                warn!(airport = iata, error = %e, attempt = consecutive_errors, "Failed to poll scraper, retrying...");
                if consecutive_errors >= MAX_POLL_ERRORS {
                    anyhow::bail!("Scraper unreachable after {} attempts for {}: {}", MAX_POLL_ERRORS, iata, e);
                }
                continue;
            }
        };

        let status: JobStatus = match status_resp.json().await {
            Ok(s) => s,
            Err(e) => {
                consecutive_errors += 1;
                warn!(airport = iata, error = %e, "Failed to parse scraper status, retrying...");
                if consecutive_errors >= MAX_POLL_ERRORS {
                    anyhow::bail!("Failed to parse scraper status after {} attempts for {}", MAX_POLL_ERRORS, iata);
                }
                continue;
            }
        };
        consecutive_errors = 0;

        let is_done = matches!(status.status.as_str(), "completed" | "failed");

        // Try to resolve place_id if we don't have it yet
        if place_id.is_none() {
            if let Ok(resp) = client
                .get(format!("{}/places", base_url))
                .header("X-API-Key", &api_key)
                .send()
                .await
            {
                if let Ok(places) = resp.json::<Vec<Place>>().await {
                    place_id = places
                        .iter()
                        .find(|p| p.original_url.as_deref().map_or(false, |u| u == google_maps_url))
                        .map(|p| p.place_id.clone());
                }
            }
        }

        // Ingest latest reviews from the scraper (always from offset 0).
        // The scraper is still adding reviews so offset-based pagination shifts;
        // instead we grab the latest batch each cycle and let ON CONFLICT dedup.
        if let Some(ref pid) = place_id {
            // Paginate through everything currently available
            let mut page_offset: u32 = 0;
            loop {
                let page = match client
                    .get(format!("{}/reviews/{}?limit={}&offset={}", base_url, pid, REVIEWS_PAGE_SIZE, page_offset))
                    .header("X-API-Key", &api_key)
                    .send().await
                {
                    Ok(r) => match r.json::<ReviewsPage>().await { Ok(p) => p, Err(_) => break },
                    Err(_) => break,
                };

                if page.reviews.is_empty() { break; }

                let mut batch_count = 0;
                for review in &page.reviews {
                    let review_date = review.review_date.as_deref().and_then(parse_review_date);
                    if let (Some(rd), Some(cutoff)) = (review_date, last_record_date) {
                        if rd <= cutoff { continue; }
                    }
                    let source_url = format!("google://{}/{}", iata, review.review_id);
                    let overall_rating = review.rating.map(|r| (r * 2.0).round() as i16);
                    let review_text = review.text();
                    let _ = sqlx::query(
                        r#"INSERT INTO reviews_raw (airport_id, source, review_date, overall_rating, review_text, source_url)
                           VALUES ($1, 'google', $2, $3, $4, $5)
                           ON CONFLICT (source_url) DO UPDATE SET overall_rating = EXCLUDED.overall_rating, review_text = EXCLUDED.review_text"#,
                    )
                    .bind(airport.id).bind(review_date).bind(overall_rating).bind(review_text).bind(&source_url)
                    .execute(pool).await;
                    batch_count += 1;
                }

                page_offset += page.reviews.len() as u32;
                ingested_offset = page_offset;

                if batch_count > 0 {
                    info!(airport = iata, batch = batch_count, total = ingested_offset, "Ingested reviews batch");
                }

                if page_offset >= page.total || page_offset >= MAX_REVIEWS_PER_AIRPORT {
                    break;
                }
            }

            if ingested_offset >= MAX_REVIEWS_PER_AIRPORT {
                info!(airport = iata, max = MAX_REVIEWS_PER_AIRPORT, "Hit review cap, stopping");
                break;
            }
        }

        if is_done {
            if status.status == "failed" {
                warn!(airport = iata, "Scraper job failed, using reviews collected so far");
            }
            break;
        }
    }

    // Final pass: ingest any remaining reviews we missed during polling
    // (the scraper may have added more between our last poll and completion).

    // ── Final pass: pick up any remaining reviews ─────────────

    // Resolve place_id if we still don't have it
    if place_id.is_none() {
        let places_resp: Vec<Place> = client
            .get(format!("{}/places", base_url))
            .header("X-API-Key", &api_key)
            .send()
            .await
            .context("Failed to fetch places")?
            .json()
            .await
            .context("Failed to parse places")?;

        place_id = places_resp
            .iter()
            .find(|p| p.original_url.as_deref().map_or(false, |u| u == google_maps_url))
            .map(|p| p.place_id.clone());
    }

    let final_place_id = place_id.context(format!(
        "No place found matching URL {} for {}",
        google_maps_url, iata
    ))?;

    let mut records: i32 = ingested_offset as i32;
    let mut latest_date: Option<NaiveDate> = None;
    let mut offset: u32 = ingested_offset;

    // Continue from where streaming left off
    loop {
        let page: ReviewsPage = client
            .get(format!(
                "{}/reviews/{}?limit={}&offset={}",
                base_url, final_place_id, REVIEWS_PAGE_SIZE, offset
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
        if offset >= page.total || offset >= MAX_REVIEWS_PER_AIRPORT {
            if offset >= MAX_REVIEWS_PER_AIRPORT {
                info!(airport = iata, max = MAX_REVIEWS_PER_AIRPORT, "Hit review cap, stopping pagination");
            }
            break;
        }
    }

    info!(airport = iata, records = records, timed_out, "Google Reviews upserted");

    if timed_out {
        anyhow::bail!(
            "Scraper timed out after {}s for {} — ingested {} reviews but job did not complete",
            MAX_POLL_DURATION_SECS, iata, records
        );
    }

    Ok(FetchResult {
        records_processed: records,
        last_record_date: latest_date,
    })
}
