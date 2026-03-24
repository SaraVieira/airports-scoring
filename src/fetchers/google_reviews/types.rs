use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Scraper API types ────────────────────────────────────────

#[derive(Debug, Serialize)]
pub(crate) struct ScrapeRequest {
    pub url: String,
    pub headless: bool,
    pub sort_by: String,
    pub download_images: bool,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ScrapeResponse {
    pub job_id: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct JobStatus {
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Place {
    pub place_id: String,
    pub original_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ReviewsPage {
    pub total: u32,
    pub reviews: Vec<GoogleReview>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct GoogleReview {
    pub review_id: String,
    pub rating: Option<f64>,
    /// Maps language code → text, e.g. {"en": "Great airport"}
    pub review_text: Option<HashMap<String, String>>,
    pub review_date: Option<String>,
}

impl GoogleReview {
    /// Extract review text, preferring English, falling back to first available.
    pub fn text(&self) -> Option<&str> {
        let map = self.review_text.as_ref()?;
        if map.is_empty() {
            return None;
        }
        map.get("en")
            .or_else(|| map.values().next())
            .map(|s| s.as_str())
    }
}

// ── Helpers ──────────────────────────────────────────────────

/// Parse a review_date string like "2025-03-22T14:43:55+00:00" into NaiveDate.
pub(crate) fn parse_review_date(date_str: &str) -> Option<NaiveDate> {
    chrono::DateTime::parse_from_rfc3339(date_str)
        .map(|dt| dt.date_naive())
        .ok()
        .or_else(|| {
            NaiveDate::parse_from_str(&date_str[..10.min(date_str.len())], "%Y-%m-%d").ok()
        })
}

/// Check if the scraper service is reachable.
pub(crate) async fn check_scraper_health(base_url: &str) -> bool {
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
