use chrono::NaiveDate;
use rust_decimal::Decimal;

/// Raw METAR observation parsed from IEM CSV.
#[derive(Debug)]
#[allow(dead_code)]
pub(super) struct MetarObs {
    pub(super) date: NaiveDate,
    pub(super) temp_f: Option<f64>,
    pub(super) dewpoint_f: Option<f64>,
    pub(super) visibility_miles: Option<f64>,
    pub(super) wind_speed_kt: Option<f64>,
    pub(super) wind_gust_kt: Option<f64>,
    pub(super) precip_in: Option<f64>,
    pub(super) sky_level1_ft: Option<f64>,
    pub(super) weather_codes: String,
}

/// Aggregated daily METAR summary.
#[derive(Debug, Default)]
pub(super) struct DailySummary {
    pub(super) temps_c: Vec<f64>,
    pub(super) visibilities_m: Vec<f64>,
    pub(super) wind_speeds_kt: Vec<f64>,
    pub(super) max_gust_kt: Option<f64>,
    pub(super) precipitation_flag: bool,
    pub(super) fog_flag: bool,
    pub(super) low_ceiling_flag: bool,
    pub(super) thunderstorm_flag: bool,
    pub(super) count: i32,
}

pub(super) fn avg_decimal(vals: &[f64]) -> Option<Decimal> {
    if vals.is_empty() {
        return None;
    }
    let sum: f64 = vals.iter().sum();
    Decimal::from_f64_retain(sum / vals.len() as f64)
}

pub(super) fn min_decimal(vals: &[f64]) -> Option<Decimal> {
    vals.iter()
        .cloned()
        .reduce(f64::min)
        .and_then(Decimal::from_f64_retain)
}

pub(super) fn max_decimal(vals: &[f64]) -> Option<Decimal> {
    vals.iter()
        .cloned()
        .reduce(f64::max)
        .and_then(Decimal::from_f64_retain)
}
