use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tokio::sync::{watch, RwLock, Semaphore};
use tracing::{error, info, warn};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::config::SeedAirport;
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
}

impl JobManager {
    pub fn new(pool: PgPool, max_concurrency: usize) -> Self {
        Self {
            pool,
            semaphore: Arc::new(Semaphore::new(max_concurrency)),
            jobs: Arc::new(RwLock::new(HashMap::new())),
            cancel_tokens: Arc::new(RwLock::new(HashMap::new())),
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
) {
    // Mark running.
    update_job_status(&jobs_map, &job_id, "running", None).await;

    // Load seed data from DB.
    let seed_airports = match load_seed_from_db(&pool).await {
        Ok(s) => s,
        Err(e) => {
            let msg = format!("Failed to load seed airports: {e}");
            error!("{msg}");
            update_job_status(&jobs_map, &job_id, "failed", Some(msg)).await;
            cleanup_cancel_token(&cancel_tokens_map, &job_id).await;
            return;
        }
    };

    let seed_iata_codes: Vec<&str> = seed_airports.iter().map(|a| a.iata.as_str()).collect();

    let mut airports_completed = 0;
    let mut had_errors = false;

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

        // Resolve airport from DB.
        let airport = match db::get_airport_by_iata(&pool, iata).await {
            Ok(a) => a,
            Err(e) => {
                warn!(job_id = %job_id, iata = %iata, error = %e, "Airport not found, skipping");
                airports_completed += 1;
                update_job_progress(&jobs_map, &job_id, airports_completed, airport_iatas.len(), None, None).await;
                continue;
            }
        };

        // Run each source.
        for source in &sources {
            // Check cancellation before each source.
            if *cancel_rx.borrow() {
                info!(job_id = %job_id, "Job cancelled during source processing");
                update_job_status(&jobs_map, &job_id, "cancelled", None).await;
                cleanup_cancel_token(&cancel_tokens_map, &job_id).await;
                return;
            }

            update_job_progress(
                &jobs_map,
                &job_id,
                airports_completed,
                airport_iatas.len(),
                Some(iata.clone()),
                Some(source.clone()),
            )
            .await;

            info!(job_id = %job_id, iata = %iata, source = %source, "Running fetcher");

            let mut cancel_watch = cancel_rx.clone();
            let result = tokio::select! {
                r = pipeline::dispatch_fetcher(
                    &pool,
                    &airport,
                    source,
                    full_refresh,
                    &seed_iata_codes,
                    &seed_airports,
                ) => r,
                _ = async {
                    while !*cancel_watch.borrow_and_update() {
                        if cancel_watch.changed().await.is_err() {
                            std::future::pending::<()>().await;
                        }
                    }
                } => {
                    info!(job_id = %job_id, iata = %iata, source = %source, "Fetcher interrupted by cancellation");
                    // If this was a google_reviews or reviews source, cancel active scraper jobs.
                    if source == "google_reviews" || source == "reviews" {
                        cancel_scraper_jobs().await;
                    }
                    update_job_status(&jobs_map, &job_id, "cancelled", None).await;
                    cleanup_cancel_token(&cancel_tokens_map, &job_id).await;
                    return;
                }
            };

            // Update source_status table.
            let (status_str, record_count, error_msg) = match &result {
                Ok(r) => ("success", r.records_processed, None),
                Err(e) => {
                    had_errors = true;
                    ("failed", 0i32, Some(e.to_string()))
                }
            };

            if let Err(e) = update_source_status(&pool, iata, source, status_str, record_count, error_msg.as_deref()).await {
                error!(job_id = %job_id, iata = %iata, source = %source, error = %e, "Failed to update source_status");
            }

            match result {
                Ok(r) => {
                    info!(job_id = %job_id, iata = %iata, source = %source, records = r.records_processed, "Fetch completed");
                }
                Err(e) => {
                    error!(job_id = %job_id, iata = %iata, source = %source, error = %e, "Fetch failed");
                }
            }
        }

        airports_completed += 1;
        update_job_progress(&jobs_map, &job_id, airports_completed, airport_iatas.len(), None, None).await;
    }

    // Run scoring if requested.
    if score {
        info!(job_id = %job_id, "Running scoring");
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

    let final_status = if had_errors { "completed" } else { "completed" };
    // Even with partial errors we mark as completed — errors are logged per source.
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
        if status == "completed" || status == "failed" || status == "cancelled" {
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

async fn load_seed_from_db(pool: &PgPool) -> Result<Vec<SeedAirport>, sqlx::Error> {
    let rows = load_supported_from_db(pool).await?;
    Ok(rows
        .into_iter()
        .map(|r| SeedAirport {
            iata: r.iata_code,
            country: r.country_code,
            name: r.name,
            skytrax_review_slug: r.skytrax_review_slug,
            skytrax_rating_slug: r.skytrax_rating_slug,
            google_maps_url: r.google_maps_url,
        })
        .collect())
}

/// Cancel all running jobs on the Google Reviews scraper service.
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
