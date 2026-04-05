use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::LazyLock;
use std::time::Instant;
use tokio::sync::Mutex;
use tracing::{info, warn};
use utoipa::ToSchema;

use crate::server::AppState;

// ── Cache ──────────────────────────────────────────────────────

const CACHE_TTL_SECS: u64 = 30;
const CACHE_CLEANUP_SECS: u64 = 300;

static PULSE_CACHE: LazyLock<Mutex<HashMap<String, (Instant, LivePulseResponse)>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

// ── Response types ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AircraftState {
    pub icao24: String,
    pub callsign: Option<String>,
    pub lat: f64,
    pub lon: f64,
    /// Barometric altitude in meters
    pub altitude: Option<f64>,
    /// Ground speed in m/s
    pub velocity: Option<f64>,
    /// Heading in degrees (0 = north, clockwise)
    pub heading: Option<f64>,
    /// Vertical rate in m/s (negative = descending)
    pub vertical_rate: Option<f64>,
    pub on_ground: bool,
    /// "arriving", "departing", "cruising", or "ground"
    pub status: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PulseCounts {
    pub total: u32,
    pub in_air: u32,
    pub on_ground: u32,
    pub arriving: u32,
    pub departing: u32,
    pub cruising: u32,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LivePulseResponse {
    pub iata: String,
    pub airport_lat: f64,
    pub airport_lon: f64,
    pub timestamp: i64,
    pub aircraft: Vec<AircraftState>,
    pub counts: PulseCounts,
    pub cached: bool,
}

// ── OpenSky response parsing ───────────────────────────────────

#[derive(Debug, Deserialize)]
struct StatesResponse {
    time: i64,
    states: Option<Vec<Vec<serde_json::Value>>>,
}

// ── Helpers ────────────────────────────────────────────────────

/// Compute a bounding box of ~`radius_km` around a point.
fn bounding_box(lat: f64, lon: f64, radius_km: f64) -> (f64, f64, f64, f64) {
    let lat_delta = radius_km / 111.0;
    let lon_delta = radius_km / (111.0 * lat.to_radians().cos().abs().max(0.01));
    (
        lat - lat_delta,
        lat + lat_delta,
        lon - lon_delta,
        lon + lon_delta,
    )
}

fn classify(on_ground: bool, vertical_rate: Option<f64>) -> &'static str {
    if on_ground {
        return "ground";
    }
    match vertical_rate {
        Some(vr) if vr < -1.0 => "arriving",
        Some(vr) if vr > 1.0 => "departing",
        _ => "cruising",
    }
}

fn parse_state_vector(sv: &[serde_json::Value]) -> Option<AircraftState> {
    // OpenSky state vector indices:
    // 0=icao24, 1=callsign, 5=longitude, 6=latitude, 7=baro_altitude,
    // 8=on_ground, 9=velocity, 10=true_track, 11=vertical_rate
    if sv.len() < 12 {
        return None;
    }

    let icao24 = sv[0].as_str()?.to_string();
    let callsign = sv[1].as_str().map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    let lon = sv[5].as_f64()?;
    let lat = sv[6].as_f64()?;
    let altitude = sv[7].as_f64();
    let on_ground = sv[8].as_bool().unwrap_or(false);
    let velocity = sv[9].as_f64();
    let heading = sv[10].as_f64();
    let vertical_rate = sv[11].as_f64();

    let status = classify(on_ground, vertical_rate).to_string();

    Some(AircraftState {
        icao24,
        callsign,
        lat,
        lon,
        altitude,
        velocity,
        heading,
        vertical_rate,
        on_ground,
        status,
    })
}

// ── Handler ────────────────────────────────────────────────────

/// Get live flight activity around an airport.
#[utoipa::path(
    get,
    path = "/api/airports/{iata}/live",
    params(("iata" = String, Path, description = "IATA airport code")),
    responses(
        (status = 200, description = "Live flight pulse", body = LivePulseResponse),
        (status = 404, description = "Airport not found or no location data"),
        (status = 503, description = "OpenSky unavailable or no credentials"),
    ),
    tag = "airports"
)]
pub async fn get_live_pulse(
    State(state): State<AppState>,
    Path(iata): Path<String>,
) -> Result<Json<LivePulseResponse>, StatusCode> {
    let iata = iata.to_uppercase();

    // Check cache
    {
        let mut cache = PULSE_CACHE.lock().await;

        // Cleanup old entries
        cache.retain(|_, (instant, _)| instant.elapsed().as_secs() < CACHE_CLEANUP_SECS);

        if let Some((instant, cached)) = cache.get(&iata) {
            if instant.elapsed().as_secs() < CACHE_TTL_SECS {
                let mut resp = cached.clone();
                resp.cached = true;
                return Ok(Json(resp));
            }
        }
    }

    // Get airport location
    let row: Option<(f64, f64)> = sqlx::query_as(
        r#"
        SELECT ST_Y(location::geometry)::float8 AS lat,
               ST_X(location::geometry)::float8 AS lon
        FROM airports
        WHERE iata_code = $1 AND location IS NOT NULL
        "#,
    )
    .bind(&iata)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| {
        warn!(error = %e, "DB error fetching airport location");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let (airport_lat, airport_lon) = row.ok_or(StatusCode::NOT_FOUND)?;

    // Build bounding box
    let (lamin, lamax, lomin, lomax) = bounding_box(airport_lat, airport_lon, 50.0);

    // Get OpenSky token
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let token = crate::fetchers::opensky::get_access_token(&client)
        .await
        .map_err(|e| {
            warn!(error = %e, "OpenSky auth failed");
            StatusCode::SERVICE_UNAVAILABLE
        })?;

    // Call OpenSky /states/all with bounding box
    let url = format!(
        "https://opensky-network.org/api/states/all?lamin={}&lamax={}&lomin={}&lomax={}",
        lamin, lamax, lomin, lomax
    );

    let resp = client
        .get(&url)
        .bearer_auth(&token)
        .send()
        .await
        .map_err(|e| {
            warn!(error = %e, "OpenSky request failed");
            StatusCode::SERVICE_UNAVAILABLE
        })?;

    if !resp.status().is_success() {
        let status = resp.status();
        warn!(%status, airport = %iata, "OpenSky returned non-200");
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }

    let states_resp: StatesResponse = resp.json().await.map_err(|e| {
        warn!(error = %e, "Failed to parse OpenSky response");
        StatusCode::SERVICE_UNAVAILABLE
    })?;

    // Parse aircraft
    let aircraft: Vec<AircraftState> = states_resp
        .states
        .unwrap_or_default()
        .iter()
        .filter_map(|sv| parse_state_vector(sv))
        .collect();

    // Compute counts
    let mut counts = PulseCounts {
        total: aircraft.len() as u32,
        in_air: 0,
        on_ground: 0,
        arriving: 0,
        departing: 0,
        cruising: 0,
    };

    for ac in &aircraft {
        match ac.status.as_str() {
            "ground" => counts.on_ground += 1,
            "arriving" => {
                counts.in_air += 1;
                counts.arriving += 1;
            }
            "departing" => {
                counts.in_air += 1;
                counts.departing += 1;
            }
            "cruising" => {
                counts.in_air += 1;
                counts.cruising += 1;
            }
            _ => {}
        }
    }

    let response = LivePulseResponse {
        iata: iata.clone(),
        airport_lat,
        airport_lon,
        timestamp: states_resp.time,
        aircraft,
        counts,
        cached: false,
    };

    // Store in cache
    {
        let mut cache = PULSE_CACHE.lock().await;
        cache.insert(iata.clone(), (Instant::now(), response.clone()));
    }

    info!(
        airport = %iata,
        total = response.counts.total,
        arriving = response.counts.arriving,
        departing = response.counts.departing,
        "Live pulse fetched"
    );

    Ok(Json(response))
}
