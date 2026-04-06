#![allow(dead_code)]
use anyhow::{Context, Result};
use serde::Deserialize;
use sqlx::PgPool;

#[derive(Debug, Clone, Deserialize)]
pub struct SeedAirport {
    pub iata: String,
    pub country: String,
    pub name: String,
    pub skytrax_review_slug: Option<String>,
    pub skytrax_rating_slug: Option<String>,
    pub google_maps_url: Option<String>,
}

/// Load the seed airport list from the database (supported_airports table).
pub async fn load_seed_airports_from_db(pool: &PgPool) -> Result<Vec<SeedAirport>> {
    let rows: Vec<(String, String, String, Option<String>, Option<String>, Option<String>)> =
        sqlx::query_as(
            "SELECT iata_code, country_code, name, \
                    skytrax_review_slug, skytrax_rating_slug, google_maps_url \
             FROM supported_airports WHERE enabled = true ORDER BY iata_code",
        )
        .fetch_all(pool)
        .await
        .context("Failed to load seed airports from database")?;

    Ok(rows
        .into_iter()
        .map(|(iata, country, name, review_slug, rating_slug, google_url)| SeedAirport {
            iata,
            country,
            name,
            skytrax_review_slug: review_slug,
            skytrax_rating_slug: rating_slug,
            google_maps_url: google_url,
        })
        .collect())
}

/// Extract just the IATA codes from the seed config.
pub fn seed_iata_codes(airports: &[SeedAirport]) -> Vec<&str> {
    airports.iter().map(|a| a.iata.as_str()).collect()
}
