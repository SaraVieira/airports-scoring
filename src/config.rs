#![allow(dead_code)]
use anyhow::{Context, Result};
use serde::Deserialize;

const DEFAULT_CONFIG_PATH: &str = "airports.json";

#[derive(Debug, Clone, Deserialize)]
pub struct SeedAirport {
    pub iata: String,
    pub country: String,
    pub name: String,
    pub skytrax_review_slug: Option<String>,
    pub skytrax_rating_slug: Option<String>,
}

/// Load the seed airport list from `airports.json` (or a custom path).
pub fn load_seed_airports(path: Option<&str>) -> Result<Vec<SeedAirport>> {
    let p = path.unwrap_or(DEFAULT_CONFIG_PATH);
    let content = std::fs::read_to_string(p)
        .with_context(|| format!("Failed to read airport config from '{}'", p))?;
    let airports: Vec<SeedAirport> =
        serde_json::from_str(&content).context("Failed to parse airports.json")?;
    Ok(airports)
}

/// Extract just the IATA codes from the seed config.
pub fn seed_iata_codes(airports: &[SeedAirport]) -> Vec<&str> {
    airports.iter().map(|a| a.iata.as_str()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_airports_json() {
        let airports = load_seed_airports(None).expect("airports.json should exist at repo root");
        assert!(!airports.is_empty(), "airports.json should not be empty");

        // Verify all entries have required fields.
        for a in &airports {
            assert!(!a.iata.is_empty(), "IATA code must not be empty");
            assert_eq!(a.iata.len(), 3, "IATA code must be 3 chars: {}", a.iata);
            assert!(!a.country.is_empty(), "Country code must not be empty");
            assert!(!a.name.is_empty(), "Name must not be empty");
        }

        // Check a known airport exists.
        assert!(
            airports.iter().any(|a| a.iata == "LHR"),
            "LHR should be in airports.json"
        );
    }
}
