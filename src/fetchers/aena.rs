use anyhow::{Context, Result, bail};
use calamine::{open_workbook_auto_from_rs, Reader, Data};
use sqlx::PgPool;
use tracing::{info, warn};

use crate::models::{Airport, FetchResult};

/// AENA traffic statistics page (for reference — actual XLS URLs are constructed below).
const AENA_STATS_BASE: &str =
    "https://www.aena.es/es/estadisticas/estadisticas-de-trafico-aereo.html";

/// AENA ICAO codes for airports we care about.
const MADRID_ICAO: &str = "LEMD";
const BARCELONA_ICAO: &str = "LEBL";

/// Fetch AENA traffic data for MAD or BCN only.
/// Downloads monthly Excel files with passengers, operations, and cargo per airport.
pub async fn fetch(pool: &PgPool, airport: &Airport, _full_refresh: bool) -> Result<FetchResult> {
    let icao = airport
        .icao_code
        .as_deref()
        .context("Airport has no ICAO code")?;
    let iata = airport
        .iata_code
        .as_deref()
        .unwrap_or("???");

    // Only process MAD and BCN.
    if icao != MADRID_ICAO && icao != BARCELONA_ICAO {
        info!(
            airport = iata,
            "AENA fetcher only supports MAD and BCN, skipping"
        );
        return Ok(FetchResult {
            records_processed: 0,
            last_record_date: None,
        });
    }

    let client = reqwest::Client::builder()
        .user_agent("AirportIntelligencePlatform/1.0")
        .build()?;

    // AENA publishes annual summary XLS files. Try recent years.
    let current_year = chrono::Utc::now().naive_utc().date().year();
    let mut total_records: i32 = 0;
    let mut last_date = None;

    // Try to fetch data for the last several years.
    for year in (2004..=current_year).rev() {
        match fetch_aena_year(&client, pool, airport, icao, year as i16).await {
            Ok(count) => {
                if count > 0 {
                    total_records += count;
                    if last_date.is_none() {
                        last_date = Some(chrono::NaiveDate::from_ymd_opt(year, 12, 31).unwrap());
                    }
                }
            }
            Err(e) => {
                // AENA URLs are unpredictable — just log and continue.
                warn!(
                    airport = iata,
                    year = year,
                    error = %e,
                    "Could not fetch AENA data for year"
                );
                // Stop going further back if we can't find data.
                if year < current_year - 2 {
                    break;
                }
            }
        }
    }

    info!(
        airport = iata,
        records = total_records,
        "AENA fetch complete"
    );

    Ok(FetchResult {
        records_processed: total_records,
        last_record_date: last_date,
    })
}

use chrono::Datelike;

/// Attempt to download and parse AENA XLS data for a specific year.
async fn fetch_aena_year(
    client: &reqwest::Client,
    pool: &PgPool,
    airport: &Airport,
    icao: &str,
    year: i16,
) -> Result<i32> {
    // AENA publishes Excel files at somewhat predictable URLs.
    // The exact URL pattern varies, so we try common patterns.
    let urls = vec![
        format!(
            "https://www.aena.es/sites/default/files/estadisticas/trafico_pasajeros_{}.xls",
            year
        ),
        format!(
            "https://www.aena.es/sites/default/files/estadisticas/trafico_pasajeros_{}.xlsx",
            year
        ),
    ];

    for url in &urls {
        match try_download_and_parse(client, pool, airport, icao, year, url).await {
            Ok(count) if count > 0 => return Ok(count),
            Ok(_) => continue,
            Err(_) => continue,
        }
    }

    // If direct download fails, we can still upsert from known data patterns.
    // For now, return 0 to indicate no data found for this year.
    Ok(0)
}

async fn try_download_and_parse(
    client: &reqwest::Client,
    pool: &PgPool,
    airport: &Airport,
    icao: &str,
    year: i16,
    url: &str,
) -> Result<i32> {
    let resp = client.get(url).send().await?;
    if !resp.status().is_success() {
        bail!("HTTP {} for {}", resp.status(), url);
    }

    let bytes = resp.bytes().await?;
    let cursor = std::io::Cursor::new(bytes.to_vec());

    let mut workbook = open_workbook_auto_from_rs(cursor)
        .context("Failed to open AENA workbook")?;

    let sheet_names: Vec<String> = workbook.sheet_names().to_vec();
    let mut records = 0;

    for sheet_name in &sheet_names {
        let range = match workbook.worksheet_range(sheet_name) {
            Ok(r) => r,
            Err(_) => continue,
        };

        // Look for rows containing our airport ICAO or name.
        let airport_name_lower = if icao == MADRID_ICAO {
            "madrid"
        } else {
            "barcelona"
        };

        for row in range.rows() {
            // Check if this row mentions our airport.
            let row_text: String = row
                .iter()
                .filter_map(|cell| match cell {
                    Data::String(s) => Some(s.to_lowercase()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join(" ");

            if !row_text.contains(airport_name_lower) && !row_text.contains(&icao.to_lowercase()) {
                continue;
            }

            // Try to extract passenger count from numeric cells.
            let numbers: Vec<i64> = row
                .iter()
                .filter_map(|cell| match cell {
                    Data::Float(f) => Some(*f as i64),
                    Data::Int(i) => Some(*i),
                    _ => None,
                })
                .filter(|&n| n > 10_000) // passenger counts should be substantial
                .collect();

            if let Some(&total_pax) = numbers.iter().max() {
                sqlx::query(
                    "INSERT INTO pax_yearly (airport_id, year, total_pax, source)
                     VALUES ($1, $2, $3, 'aena')
                     ON CONFLICT (airport_id, year) DO UPDATE SET
                         total_pax = GREATEST(EXCLUDED.total_pax, pax_yearly.total_pax)",
                )
                .bind(airport.id)
                .bind(year)
                .bind(total_pax)
                .execute(pool)
                .await
                .with_context(|| format!("Failed to upsert AENA pax for year {}", year))?;

                records += 1;
                break; // Only need one row per year per airport.
            }
        }
    }

    Ok(records)
}
