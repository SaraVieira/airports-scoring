#![allow(dead_code)]

use rust_decimal::Decimal;
use serde::Deserialize;

// ── CSV row structs ─────────────────────────────────────────────

/// Row from airports.csv
#[derive(Debug, Deserialize)]
pub(super) struct CsvAirport {
    pub id: i64,
    pub ident: String,
    #[serde(rename = "type")]
    pub airport_type: String,
    pub name: String,
    pub latitude_deg: Option<f64>,
    pub longitude_deg: Option<f64>,
    pub elevation_ft: Option<String>,
    pub continent: Option<String>,
    pub iso_country: Option<String>,
    pub iso_region: Option<String>,
    pub municipality: Option<String>,
    pub scheduled_service: Option<String>,
    pub gps_code: Option<String>,
    pub iata_code: Option<String>,
    pub local_code: Option<String>,
    pub home_link: Option<String>,
    pub wikipedia_link: Option<String>,
    pub keywords: Option<String>,
}

/// Row from runways.csv
#[derive(Debug, Deserialize)]
pub(super) struct CsvRunway {
    pub id: i64,
    pub airport_ref: i64,
    pub airport_ident: Option<String>,
    pub length_ft: Option<String>,
    pub width_ft: Option<String>,
    pub surface: Option<String>,
    pub lighted: Option<String>,
    pub closed: Option<String>,
    pub le_ident: Option<String>,
    pub le_latitude_deg: Option<String>,
    pub le_longitude_deg: Option<String>,
    pub le_elevation_ft: Option<String>,
    #[serde(rename = "le_heading_degT")]
    pub le_heading_deg_t: Option<String>,
    pub le_displaced_threshold_ft: Option<String>,
    pub he_ident: Option<String>,
    pub he_latitude_deg: Option<String>,
    pub he_longitude_deg: Option<String>,
    pub he_elevation_ft: Option<String>,
    #[serde(rename = "he_heading_degT")]
    pub he_heading_deg_t: Option<String>,
    pub he_displaced_threshold_ft: Option<String>,
}

/// Row from airport-frequencies.csv
#[derive(Debug, Deserialize)]
pub(super) struct CsvFrequency {
    pub id: i64,
    pub airport_ref: i64,
    pub airport_ident: Option<String>,
    #[serde(rename = "type")]
    pub freq_type: Option<String>,
    pub description: Option<String>,
    pub frequency_mhz: Option<f64>,
}

/// Row from navaids.csv
#[derive(Debug, Deserialize)]
pub(super) struct CsvNavaid {
    pub id: i64,
    #[serde(rename = "filename")]
    pub _filename: Option<String>,
    pub ident: Option<String>,
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub navaid_type: Option<String>,
    pub frequency_khz: Option<String>,
    pub latitude_deg: Option<f64>,
    pub longitude_deg: Option<f64>,
    pub elevation_ft: Option<String>,
    pub iso_country: Option<String>,
    pub dme_frequency_khz: Option<String>,
    pub dme_channel: Option<String>,
    pub dme_latitude_deg: Option<f64>,
    pub dme_longitude_deg: Option<f64>,
    pub dme_elevation_ft: Option<String>,
    pub slaved_variation_deg: Option<String>,
    pub magnetic_variation_deg: Option<String>,
    #[serde(rename = "usageType")]
    pub usage_type: Option<String>,
    pub power: Option<String>,
    pub associated_airport: Option<String>,
}

// ── Helper: parse optional numeric strings ──────────────────────

pub(super) fn parse_opt_i32(s: &Option<String>) -> Option<i32> {
    s.as_deref()
        .and_then(|v| v.trim().parse::<i32>().ok())
}

pub(super) fn parse_opt_f64(s: &Option<String>) -> Option<f64> {
    s.as_deref()
        .and_then(|v| v.trim().parse::<f64>().ok())
}

pub(super) fn parse_opt_decimal(s: &Option<String>) -> Option<Decimal> {
    s.as_deref()
        .and_then(|v| v.trim().parse::<Decimal>().ok())
}

pub(super) fn parse_bool_field(s: &Option<String>) -> Option<bool> {
    s.as_deref().map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
}
