use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tokio::sync::{watch, RwLock, Semaphore};
use tracing::{error, info, warn};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::db;
use crate::models::{Airport, SupportedAirport};
use crate::pipeline;
use crate::scoring;

// ── Types ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct JobInfo {
    pub id: String,
    pub airports: Vec<String>,
    pub sources: Vec<String>,
    pub full_refresh: bool,
    pub score: bool,
    pub status: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub progress: JobProgress,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct JobProgress {
    pub airports_completed: usize,
    pub airports_total: usize,
    pub current_airport: Option<String>,
    pub current_source: Option<String>,
    /// Current phase: "fetching", "reviews", "sentiment", "scoring", or null
    pub phase: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StartJobRequest {
    pub airports: Option<Vec<String>>,
    pub sources: Option<Vec<String>>,
    pub full_refresh: Option<bool>,
    pub score: Option<bool>,
}

// ── JobManager ───────────────────────────────────────────────────

const MAX_JOB_HISTORY: usize = 100;

pub struct JobManager {
    pool: PgPool,
    semaphore: Arc<Semaphore>,
    jobs: Arc<RwLock<HashMap<String, JobInfo>>>,
    cancel_tokens: Arc<RwLock<HashMap<String, watch::Sender<bool>>>>,
    scraper_pool: Option<crate::scraper_pool::ScraperPool>,
}

impl JobManager {
    pub fn new(pool: PgPool, max_concurrency: usize, scraper_pool: Option<crate::scraper_pool::ScraperPool>) -> Self {
        Self {
            pool,
            semaphore: Arc::new(Semaphore::new(max_concurrency)),
            jobs: Arc::new(RwLock::new(HashMap::new())),
            cancel_tokens: Arc::new(RwLock::new(HashMap::new())),
            scraper_pool,
        }
    }

    pub async fn start_job(&self, request: StartJobRequest) -> Result<JobInfo, String> {
        let full_refresh = request.full_refresh.unwrap_or(false);
        let score = request.score.unwrap_or(true);

        // Resolve airports from DB.
        let supported = load_supported_from_db(&self.pool)
            .await
            .map_err(|e| format!("Failed to load supported airports: {e}"))?;

        let airport_iatas: Vec<String> = match request.airports {
            Some(ref codes) => codes.clone(),
            None => supported.iter().map(|s| s.iata_code.clone()).collect(),
        };

        // Resolve sources.
        let sources: Vec<String> = match request.sources {
            Some(ref s) => s.clone(),
            None => pipeline::ALL_SOURCES.iter().map(|s| s.to_string()).collect(),
        };

        let job_id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        let job_info = JobInfo {
            id: job_id.clone(),
            airports: airport_iatas.clone(),
            sources: sources.clone(),
            full_refresh,
            score,
            status: "queued".to_string(),
            started_at: Some(now),
            completed_at: None,
            progress: JobProgress {
                airports_completed: 0,
                airports_total: airport_iatas.len(),
                current_airport: None,
                current_source: None,
                phase: None,
            },
            error: None,
        };

        // Store job and cancel token.
        let (cancel_tx, cancel_rx) = watch::channel(false);

        {
            let mut jobs = self.jobs.write().await;
            // Prune old completed jobs if we have too many.
            if jobs.len() >= MAX_JOB_HISTORY {
                let mut completed: Vec<(String, String)> = jobs
                    .iter()
                    .filter(|(_, j)| j.status == "completed" || j.status == "failed" || j.status == "cancelled")
                    .map(|(id, j)| (id.clone(), j.completed_at.clone().unwrap_or_default()))
                    .collect();
                completed.sort_by(|a, b| a.1.cmp(&b.1));
                // Remove oldest half of completed jobs.
                let to_remove = completed.len() / 2;
                for (id, _) in completed.into_iter().take(to_remove) {
                    jobs.remove(&id);
                }
            }
            jobs.insert(job_id.clone(), job_info.clone());
        }

        {
            let mut tokens = self.cancel_tokens.write().await;
            tokens.insert(job_id.clone(), cancel_tx);
        }

        // Spawn the background task.
        let pool = self.pool.clone();
        let semaphore = self.semaphore.clone();
        let jobs_map = self.jobs.clone();
        let cancel_tokens_map = self.cancel_tokens.clone();
        let scraper_pool = self.scraper_pool.clone();

        tokio::spawn(async move {
            run_job(
                pool,
                semaphore,
                jobs_map,
                cancel_tokens_map,
                job_id,
                airport_iatas,
                sources,
                full_refresh,
                score,
                cancel_rx,
                scraper_pool,
            )
            .await;
        });

        Ok(job_info)
    }

    pub async fn list_jobs(&self) -> Vec<JobInfo> {
        let jobs = self.jobs.read().await;
        let mut list: Vec<JobInfo> = jobs.values().cloned().collect();
        // Most recent first.
        list.sort_by(|a, b| b.started_at.cmp(&a.started_at));
        list
    }

    pub async fn get_job(&self, id: &str) -> Option<JobInfo> {
        let jobs = self.jobs.read().await;
        jobs.get(id).cloned()
    }

    pub async fn cancel_job(&self, id: &str) -> bool {
        let tokens = self.cancel_tokens.read().await;
        if let Some(tx) = tokens.get(id) {
            let _ = tx.send(true);
            true
        } else {
            false
        }
    }

    /// Trigger scoring only (no data fetching).
    pub async fn trigger_scoring(&self) -> Result<(), String> {
        let supported = load_supported_from_db(&self.pool)
            .await
            .map_err(|e| format!("Failed to load supported airports: {e}"))?;

        let mut airports: Vec<Airport> = Vec::new();
        for sa in &supported {
            match db::get_airport_by_iata(&self.pool, &sa.iata_code).await {
                Ok(a) => airports.push(a),
                Err(e) => {
                    warn!(iata = %sa.iata_code, error = %e, "Airport not found in airports table, skipping scoring");
                }
            }
        }

        scoring::score_airports(&self.pool, &airports)
            .await
            .map_err(|e| format!("Scoring failed: {e}"))?;

        Ok(())
    }
}

// ── Background job runner ────────────────────────────────────────

async fn run_job(
    pool: PgPool,
    semaphore: Arc<Semaphore>,
    jobs_map: Arc<RwLock<HashMap<String, JobInfo>>>,
    cancel_tokens_map: Arc<RwLock<HashMap<String, watch::Sender<bool>>>>,
    job_id: String,
    airport_iatas: Vec<String>,
    sources: Vec<String>,
    full_refresh: bool,
    score: bool,
    cancel_rx: watch::Receiver<bool>,
    scraper_pool: Option<crate::scraper_pool::ScraperPool>,
) {
    // Mark running.
    update_job_status(&jobs_map, &job_id, "running", None).await;

    // Load seed data from DB.
    let seed_airports = match crate::config::load_seed_airports_from_db(&pool).await {
        Ok(s) => s,
        Err(e) => {
            let msg = format!("Failed to load seed airports: {e}");
            error!("{msg}");
            update_job_status(&jobs_map, &job_id, "failed", Some(msg)).await;
            cleanup_cancel_token(&cancel_tokens_map, &job_id).await;
            return;
        }
    };

    let seed_iata_codes: Vec<String> = seed_airports.iter().map(|a| a.iata.clone()).collect();
    let seed_airports = Arc::new(seed_airports);

    // Run OurAirports bootstrap if this is a full run (all airports, all sources)
    // or if wikipedia is in the source list (needs wikipedia_url populated).
    // Run ourairports if: running all sources (wikipedia will need it),
    // or wikipedia is explicitly selected.
    let running_all_sources = sources.iter().any(|s| s == "wikipedia")
        || sources == pipeline::ALL_SOURCES.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    let needs_ourairports = running_all_sources;
    if needs_ourairports {
        info!(job_id = %job_id, "Running OurAirports bootstrap");
        update_job_progress(&jobs_map, &job_id, 0, airport_iatas.len(), None, Some("ourairports".to_string())).await;
        let seed_refs: Vec<&str> = seed_iata_codes.iter().map(|s| &**s).collect();
        if let Err(e) = crate::fetchers::ourairports::fetch_all(&pool, full_refresh, &seed_refs).await {
            warn!(job_id = %job_id, error = %e, "OurAirports bootstrap failed, continuing anyway");
        } else {
            info!(job_id = %job_id, "OurAirports bootstrap complete");
        }
    }

    let mut airports_completed = 0;
    let mut had_errors = false;
    update_job_phase(&jobs_map, &job_id, "fetching", None, None).await;
    // Collect review work to run in parallel after non-review sources complete.
    let mut review_work: Vec<(String, crate::models::Airport, Vec<String>)> = Vec::new();
    let mut post_review_work: Vec<(String, crate::models::Airport, Vec<String>)> = Vec::new();

    for iata in &airport_iatas {
        // Check cancellation.
        if *cancel_rx.borrow() {
            info!(job_id = %job_id, "Job cancelled");
            update_job_status(&jobs_map, &job_id, "cancelled", None).await;
            cleanup_cancel_token(&cancel_tokens_map, &job_id).await;
            return;
        }

        // Acquire semaphore permit.
        let _permit = semaphore.acquire().await.expect("Semaphore closed");

        // Resolve airport from DB. If not found, bootstrap from all_airports.
        let airport = match db::get_airport_by_iata(&pool, iata).await {
            Ok(a) => a,
            Err(_) => {
                info!(job_id = %job_id, iata = %iata, "Airport not in DB, bootstrapping from all_airports");
                update_job_progress(
                    &jobs_map, &job_id, airports_completed, airport_iatas.len(),
                    Some(iata.clone()), Some("bootstrap".to_string()),
                ).await;

                // Insert from all_airports reference table
                let inserted = sqlx::query(
                    "INSERT INTO airports (iata_code, icao_code, name, city, country_code, elevation_ft, timezone, airport_type, in_seed_set) \
                     SELECT iata, icao, name, city, country, elevation, tz, 'large_airport', true \
                     FROM all_airports WHERE iata = $1 \
                     ON CONFLICT (iata_code) DO UPDATE SET in_seed_set = true \
                     RETURNING id"
                )
                .bind(iata)
                .fetch_optional(&pool)
                .await;

                if let Err(e) = inserted {
                    warn!(job_id = %job_id, iata = %iata, error = %e, "Failed to bootstrap airport, skipping");
                    airports_completed += 1;
                    update_job_progress(&jobs_map, &job_id, airports_completed, airport_iatas.len(), None, None).await;
                    continue;
                }

                match db::get_airport_by_iata(&pool, iata).await {
                    Ok(a) => a,
                    Err(e) => {
                        warn!(job_id = %job_id, iata = %iata, error = %e, "Airport not found in all_airports, skipping");
                        airports_completed += 1;
                        update_job_progress(&jobs_map, &job_id, airports_completed, airport_iatas.len(), None, None).await;
                        continue;
                    }
                }
            }
        };

        // Split sources into:
        // - non-review: run sequentially now
        // - review: run in parallel later via scraper pool
        // - post-review: sentiment runs after all reviews are ingested
        let review_source_names = ["reviews", "google_reviews", "skytrax"];
        let post_review_names = ["sentiment"];
        let review_sources: Vec<&String> = sources.iter()
            .filter(|s| review_source_names.contains(&s.as_str()))
            .collect();
        let post_review_sources: Vec<&String> = sources.iter()
            .filter(|s| post_review_names.contains(&s.as_str()))
            .collect();
        let non_review_sources: Vec<&String> = sources.iter()
            .filter(|s| !review_source_names.contains(&s.as_str()) && !post_review_names.contains(&s.as_str()))
            .collect();

        // Run non-review sources sequentially.
        for source in &non_review_sources {
            if *cancel_rx.borrow() {
                info!(job_id = %job_id, "Job cancelled during source processing");
                update_job_status(&jobs_map, &job_id, "cancelled", None).await;
                cleanup_cancel_token(&cancel_tokens_map, &job_id).await;
                return;
            }

            update_job_progress(
                &jobs_map, &job_id, airports_completed, airport_iatas.len(),
                Some(iata.clone()), Some(source.to_string()),
            ).await;

            info!(job_id = %job_id, iata = %iata, source = %source, "Running fetcher");

            let seed_refs: Vec<&str> = seed_iata_codes.iter().map(|s| &**s).collect();
            let mut cancel_watch = cancel_rx.clone();
            let result = tokio::select! {
                r = pipeline::dispatch_fetcher(&pool, &airport, source, full_refresh, &seed_refs, &seed_airports) => r,
                _ = async {
                    while !*cancel_watch.borrow_and_update() {
                        if cancel_watch.changed().await.is_err() { std::future::pending::<()>().await; }
                    }
                } => {
                    update_job_status(&jobs_map, &job_id, "cancelled", None).await;
                    cleanup_cancel_token(&cancel_tokens_map, &job_id).await;
                    return;
                }
            };

            let (status_str, record_count, error_msg) = match &result {
                Ok(r) => ("success", r.records_processed, None),
                Err(e) => { had_errors = true; ("failed", 0i32, Some(e.to_string())) }
            };
            if let Err(e) = update_source_status(&pool, iata, source, status_str, record_count, error_msg.as_deref()).await {
                error!(job_id = %job_id, iata = %iata, source = %source, error = %e, "Failed to update source_status");
            }
            match result {
                Ok(r) => info!(job_id = %job_id, iata = %iata, source = %source, records = r.records_processed, "Fetch completed"),
                Err(e) => error!(job_id = %job_id, iata = %iata, source = %source, error = %e, "Fetch failed"),
            }
        }

        // Collect review work for this airport (will be run in parallel after all airports' non-review sources).
        if !review_sources.is_empty() {
            review_work.push((iata.clone(), airport.clone(), review_sources.iter().map(|s| s.to_string()).collect::<Vec<_>>()));
        }
        if !post_review_sources.is_empty() {
            post_review_work.push((iata.clone(), airport.clone(), post_review_sources.iter().map(|s| s.to_string()).collect::<Vec<_>>()));
        }

        airports_completed += 1;
        update_job_progress(&jobs_map, &job_id, airports_completed, airport_iatas.len(), None, None).await;
    }

    // ── Phase 2: Run review sources in parallel across scraper pool ──
    if !review_work.is_empty() {
        info!(job_id = %job_id, airports = review_work.len(), "Starting parallel review phase");
        update_job_phase(&jobs_map, &job_id, "reviews", None, Some("reviews".to_string())).await;

        let mut join_set = tokio::task::JoinSet::new();

        for (iata, airport, review_srcs) in review_work {
            let pool = pool.clone();
            let seed_iata_codes = seed_iata_codes.clone();
            let seed_airports = seed_airports.clone();
            let scraper_pool = scraper_pool.clone();
            let job_id = job_id.clone();
            let jobs_map = jobs_map.clone();

            join_set.spawn(async move {
                let mut review_had_errors = false;
                // Acquire a scraper from the pool (blocks until one is free).
                let scraper_guard = if let Some(ref sp) = scraper_pool {
                    Some(sp.acquire().await)
                } else {
                    None
                };
                let scraper_url = scraper_guard.as_ref().map(|g| g.url.as_str());

                for source in &review_srcs {
                    update_job_phase(&jobs_map, &job_id, "reviews", Some(iata.clone()), Some(source.to_string())).await;
                    info!(job_id = %job_id, iata = %iata, source = %source, scraper = ?scraper_url, "Running review fetcher");

                    let result = if source == "google_reviews" || source == "reviews" {
                        // For google_reviews (or the combined "reviews" which includes google),
                        // use the pool-assigned scraper URL.
                        if source == "google_reviews" {
                            crate::fetchers::google_reviews::fetch_with_url(
                                &pool, &airport, false, &seed_airports, scraper_url,
                            ).await
                        } else {
                            // "reviews" = skytrax + google. Run skytrax first (no scraper needed),
                            // then google with the pool scraper.
                            let skytrax_result = crate::fetchers::skytrax::fetch(&pool, &airport, false, &seed_airports).await;
                            if let Err(e) = &skytrax_result {
                                warn!(iata = %iata, error = %e, "Skytrax fetch failed");
                            }
                            // Then google
                            crate::fetchers::google_reviews::fetch_with_url(
                                &pool, &airport, false, &seed_airports, scraper_url,
                            ).await
                        }
                    } else {
                        // skytrax only — no scraper needed
                        pipeline::dispatch_fetcher(
                            &pool, &airport, source, false,
                            &seed_iata_codes.iter().map(|s| &**s).collect::<Vec<_>>(),
                            &seed_airports,
                        ).await
                    };

                    let (status_str, record_count, error_msg) = match &result {
                        Ok(r) => ("success", r.records_processed, None),
                        Err(e) => ("failed", 0i32, Some(e.to_string())),
                    };
                    let _ = update_source_status(&pool, &iata, source, status_str, record_count, error_msg.as_deref()).await;

                    let failed = result.is_err();
                    match result {
                        Ok(r) => info!(job_id = %job_id, iata = %iata, source = %source, records = r.records_processed, "Review fetch completed"),
                        Err(e) => error!(job_id = %job_id, iata = %iata, source = %source, error = %e, "Review fetch failed"),
                    }
                    if failed { review_had_errors = true; }
                }

                (iata, review_had_errors)
            });
        }

        // Wait for all review tasks to complete.
        while let Some(result) = join_set.join_next().await {
            match result {
                Ok((iata, errors)) => {
                    if errors { had_errors = true; }
                    info!(job_id = %job_id, iata = %iata, errors, "Review phase completed for airport");
                }
                Err(e) => { had_errors = true; error!(job_id = %job_id, error = %e, "Review task panicked"); }
            }
        }

        info!(job_id = %job_id, "Parallel review phase complete");
    }

    // ── Phase 3: Run post-review sources (sentiment) sequentially ──
    if !post_review_work.is_empty() {
        info!(job_id = %job_id, airports = post_review_work.len(), "Running post-review sources (sentiment)");
        update_job_phase(&jobs_map, &job_id, "sentiment", None, None).await;
        for (iata, airport, srcs) in &post_review_work {
            for source in srcs {
                update_job_progress(&jobs_map, &job_id, airports_completed, airport_iatas.len(), Some(iata.clone()), Some(source.clone())).await;
                info!(job_id = %job_id, iata = %iata, source = %source, "Running post-review fetcher");
                let seed_refs: Vec<&str> = seed_iata_codes.iter().map(|s| &**s).collect();
                let result = pipeline::dispatch_fetcher(&pool, airport, source, full_refresh, &seed_refs, &seed_airports).await;
                let (status_str, record_count, error_msg) = match &result {
                    Ok(r) => ("success", r.records_processed, None),
                    Err(e) => { had_errors = true; ("failed", 0i32, Some(e.to_string())) }
                };
                if let Err(e) = update_source_status(&pool, iata, source, status_str, record_count, error_msg.as_deref()).await {
                    error!(job_id = %job_id, iata = %iata, source = %source, error = %e, "Failed to update source_status");
                }
                match result {
                    Ok(r) => info!(job_id = %job_id, iata = %iata, source = %source, records = r.records_processed, "Post-review fetch completed"),
                    Err(e) => error!(job_id = %job_id, iata = %iata, source = %source, error = %e, "Post-review fetch failed"),
                }
            }
        }
    }

    // Run scoring if requested.
    if score {
        info!(job_id = %job_id, "Running scoring");
        update_job_phase(&jobs_map, &job_id, "scoring", None, None).await;
        update_job_progress(
            &jobs_map,
            &job_id,
            airports_completed,
            airport_iatas.len(),
            None,
            Some("scoring".to_string()),
        )
        .await;

        // Reload all airports for scoring.
        let mut all_airports: Vec<Airport> = Vec::new();
        for iata in &airport_iatas {
            if let Ok(a) = db::get_airport_by_iata(&pool, iata).await {
                all_airports.push(a);
            }
        }

        if let Err(e) = scoring::score_airports(&pool, &all_airports).await {
            error!(job_id = %job_id, error = %e, "Scoring failed");
            had_errors = true;
        }
    }

    let final_status = if had_errors { "completed_with_errors" } else { "completed" };
    update_job_status(&jobs_map, &job_id, final_status, None).await;
    cleanup_cancel_token(&cancel_tokens_map, &job_id).await;
}

// ── Helpers ──────────────────────────────────────────────────────

async fn update_job_status(
    jobs_map: &Arc<RwLock<HashMap<String, JobInfo>>>,
    job_id: &str,
    status: &str,
    error: Option<String>,
) {
    let mut jobs = jobs_map.write().await;
    if let Some(job) = jobs.get_mut(job_id) {
        job.status = status.to_string();
        job.error = error;
        if matches!(status, "completed" | "completed_with_errors" | "failed" | "cancelled") {
            job.completed_at = Some(Utc::now().to_rfc3339());
            job.progress.current_airport = None;
            job.progress.current_source = None;
        }
    }
}

async fn update_job_progress(
    jobs_map: &Arc<RwLock<HashMap<String, JobInfo>>>,
    job_id: &str,
    completed: usize,
    total: usize,
    current_airport: Option<String>,
    current_source: Option<String>,
) {
    let mut jobs = jobs_map.write().await;
    if let Some(job) = jobs.get_mut(job_id) {
        job.progress.airports_completed = completed;
        job.progress.airports_total = total;
        job.progress.current_airport = current_airport;
        job.progress.current_source = current_source;
    }
}

async fn update_job_phase(
    jobs_map: &Arc<RwLock<HashMap<String, JobInfo>>>,
    job_id: &str,
    phase: &str,
    current_airport: Option<String>,
    current_source: Option<String>,
) {
    let mut jobs = jobs_map.write().await;
    if let Some(job) = jobs.get_mut(job_id) {
        job.progress.phase = Some(phase.to_string());
        job.progress.current_airport = current_airport;
        job.progress.current_source = current_source;
    }
}

async fn cleanup_cancel_token(
    cancel_tokens: &Arc<RwLock<HashMap<String, watch::Sender<bool>>>>,
    job_id: &str,
) {
    let mut tokens = cancel_tokens.write().await;
    tokens.remove(job_id);
}

async fn update_source_status(
    pool: &PgPool,
    iata: &str,
    source: &str,
    status: &str,
    record_count: i32,
    error_msg: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO source_status (iata_code, source, last_fetched_at, last_status, last_record_count, last_error, updated_at)
        VALUES ($1, $2, now(), $3, $4, $5, now())
        ON CONFLICT (iata_code, source) DO UPDATE SET
            last_fetched_at = now(),
            last_status = EXCLUDED.last_status,
            last_record_count = EXCLUDED.last_record_count,
            last_error = EXCLUDED.last_error,
            updated_at = now()
        "#,
    )
    .bind(iata)
    .bind(source)
    .bind(status)
    .bind(record_count)
    .bind(error_msg)
    .execute(pool)
    .await?;

    Ok(())
}

async fn load_supported_from_db(pool: &PgPool) -> Result<Vec<SupportedAirport>, sqlx::Error> {
    sqlx::query_as::<_, SupportedAirport>(
        "SELECT * FROM supported_airports WHERE enabled = true ORDER BY iata_code",
    )
    .fetch_all(pool)
    .await
}


/// Cancel all running jobs on the Google Reviews scraper service.
#[allow(dead_code)]
async fn cancel_scraper_jobs() {
    let base_url =
        std::env::var("GOOGLE_SCRAPER_URL").unwrap_or_else(|_| "http://localhost:8000".to_string());
    let client = reqwest::Client::new();

    // List jobs from the scraper.
    let resp = match client.get(format!("{}/jobs", base_url)).send().await {
        Ok(r) => r,
        Err(e) => {
            warn!("Failed to list scraper jobs for cancellation: {e}");
            return;
        }
    };

    #[derive(serde::Deserialize)]
    struct ScraperJob {
        job_id: String,
        status: String,
    }

    let jobs: Vec<ScraperJob> = match resp.json().await {
        Ok(j) => j,
        Err(e) => {
            warn!("Failed to parse scraper jobs list: {e}");
            return;
        }
    };

    for job in jobs {
        if job.status == "running" || job.status == "pending" || job.status == "queued" {
            info!(scraper_job_id = %job.job_id, "Cancelling scraper job");
            if let Err(e) = client
                .post(format!("{}/jobs/{}/cancel", base_url, job.job_id))
                .send()
                .await
            {
                warn!(scraper_job_id = %job.job_id, "Failed to cancel scraper job: {e}");
            }
        }
    }
}
