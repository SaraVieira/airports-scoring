use anyhow::{Context, Result, bail};
use calamine::{open_workbook_auto, open_workbook_auto_from_rs, Reader};
use sqlx::PgPool;
use std::path::Path;
use tracing::{info, warn};

use crate::fetchers::wikipedia::USER_AGENT;
use crate::models::{Airport, FetchResult};

mod parser;
pub use parser::extract_airport_pax;

#[cfg(test)]
mod tests;

/// AENA is the Spanish airport operator — this fetcher only runs for
/// airports with country_code "ES". It also handles AENA-operated airports
/// abroad (e.g. London Luton, some Brazilian airports) but only if
/// the airport appears in the AENA spreadsheets.
const AENA_COUNTRY: &str = "ES";

/// Known AENA annual report blob IDs on their Satellite CMS.
/// These are not predictable — each must be discovered from
/// https://www.aena.es/es/estadisticas/informes-anuales.html
const AENA_BLOB_IDS: &[(i16, &str)] = &[
    (2025, "1576873058143"),
    (2024, "1576869794642"),
];

/// Base URL for AENA Satellite CMS blob downloads.
const AENA_BLOB_BASE: &str =
    "https://www.aena.es/sites/Satellite?blobcol=urldata&blobkey=id&blobtable=MungoBlobs&ssbinary=true&blobwhere=";

/// Local directory with pre-downloaded AENA annual report files.
const LOCAL_DATA_DIR: &str = "data/aena";

/// Regex for extracting year from AENA filenames.
static YEAR_RE: std::sync::LazyLock<regex::Regex> =
    std::sync::LazyLock::new(|| regex::Regex::new(r"(20\d{2})").expect("valid regex"));

/// Mapping from filename patterns to years for local files.
fn year_from_filename(filename: &str) -> Option<i16> {
    let lower = filename.to_lowercase();
    YEAR_RE
        .captures(&lower)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse::<i16>().ok())
}

/// Build search terms for matching an airport in AENA spreadsheet rows.
///
/// AENA names vary across years (e.g. "MADRID-BARAJAS" vs
/// "ADOLFO SUÁREZ MADRID-BARAJAS", "BARCELONA" vs "BARCELONA-EL PRAT J.T.").
/// We derive search terms from the airport's city and name so that any
/// Spanish airport added to supported_airports is automatically matched.
fn build_search_terms(airport: &Airport) -> Vec<String> {
    let mut terms = Vec::new();

    // Full airport name (lowercased).
    let name_lower = airport.name.to_lowercase();
    terms.push(name_lower.clone());

    // City name is often how AENA labels the airport (e.g. "VALENCIA", "SEVILLA").
    let city_lower = airport.city.to_lowercase();
    if city_lower != name_lower {
        terms.push(city_lower);
    }

    // For names like "Madrid Barajas", also try "madrid-barajas" (AENA uses hyphens).
    if airport.name.contains(' ') {
        terms.push(airport.name.to_lowercase().replace(' ', "-"));
    }

    terms
}

/// Fetch AENA traffic data. Only runs for Spanish airports (country_code "ES").
/// First tries local files in data/aena/, then falls back to remote download.
pub async fn fetch(pool: &PgPool, airport: &Airport, _full_refresh: bool) -> Result<FetchResult> {
    let iata = airport
        .iata_code
        .as_deref()
        .unwrap_or("???");

    if airport.country_code != AENA_COUNTRY {
        info!(airport = iata, "Not a Spanish airport, skipping AENA");
        return Ok(FetchResult {
            records_processed: 0,
            last_record_date: None,
        });
    }

    let search_terms = build_search_terms(airport);
    let mut total_records: i32 = 0;
    let mut last_date = None;

    // 1. Parse local files first.
    let local_dir = Path::new(LOCAL_DATA_DIR);
    if local_dir.is_dir() {
        let mut entries: Vec<_> = Vec::new();
        let mut dir = tokio::fs::read_dir(local_dir).await?;
        while let Some(entry) = dir.next_entry().await? {
            entries.push(entry);
        }
        entries.sort_by_key(|e| e.file_name());

        for entry in &entries {
            let path = entry.path();
            let fname = path.file_name().unwrap().to_string_lossy().to_string();
            let year = match year_from_filename(&fname) {
                Some(y) => y,
                None => {
                    warn!(file = %fname, "Could not determine year from AENA filename, skipping");
                    continue;
                }
            };

            match parse_aena_file(&path, airport.id, &search_terms, year, pool).await {
                Ok(count) if count > 0 => {
                    total_records += count;
                    let date = chrono::NaiveDate::from_ymd_opt(year as i32, 12, 31).unwrap();
                    if last_date.map_or(true, |d| date > d) {
                        last_date = Some(date);
                    }
                }
                Ok(_) => {}
                Err(e) => {
                    warn!(file = %fname, year = year, error = %e, "Failed to parse AENA file");
                }
            }
        }
    }

    // 2. Try remote downloads for years not covered locally.
    if total_records == 0 {
        let client = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .build()?;

        for &(year, blob_id) in AENA_BLOB_IDS {
            let url = format!("{}{}", AENA_BLOB_BASE, blob_id);
            match fetch_remote_aena(&client, &url, airport.id, &search_terms, year, pool).await {
                Ok(count) if count > 0 => {
                    total_records += count;
                    let date = chrono::NaiveDate::from_ymd_opt(year as i32, 12, 31).unwrap();
                    if last_date.map_or(true, |d| date > d) {
                        last_date = Some(date);
                    }
                }
                Ok(_) => {}
                Err(e) => {
                    warn!(year = year, error = %e, "Failed to fetch remote AENA data");
                }
            }
        }
    }

    info!(airport = iata, records = total_records, "AENA fetch complete");

    Ok(FetchResult {
        records_processed: total_records,
        last_record_date: last_date,
    })
}

async fn fetch_remote_aena(
    client: &reqwest::Client,
    url: &str,
    airport_id: i32,
    search_terms: &[String],
    year: i16,
    pool: &PgPool,
) -> Result<i32> {
    let resp = client.get(url).send().await?;
    if !resp.status().is_success() {
        bail!("HTTP {} for AENA blob", resp.status());
    }

    let bytes = resp.bytes().await?;
    let cursor = std::io::Cursor::new(bytes.to_vec());
    let workbook = open_workbook_auto_from_rs(cursor)
        .context("Failed to open AENA workbook from remote")?;

    parse_workbook(workbook, airport_id, search_terms, year, pool).await
}

async fn parse_aena_file(
    path: &Path,
    airport_id: i32,
    search_terms: &[String],
    year: i16,
    pool: &PgPool,
) -> Result<i32> {
    let workbook = open_workbook_auto(path)
        .with_context(|| format!("Failed to open AENA file: {:?}", path))?;

    parse_workbook(workbook, airport_id, search_terms, year, pool).await
}

async fn parse_workbook<RS: std::io::Read + std::io::Seek, R: Reader<RS>>(
    mut workbook: R,
    airport_id: i32,
    search_terms: &[String],
    year: i16,
    pool: &PgPool,
) -> Result<i32> {
    let sheet_names: Vec<String> = workbook.sheet_names().to_vec();

    // Skip "Mozart Reports" metadata sheet — find the data sheet.
    let data_sheet = sheet_names
        .iter()
        .find(|s| s.to_lowercase() != "mozart reports")
        .context("No data sheet found in AENA workbook")?
        .clone();

    let range = workbook
        .worksheet_range(&data_sheet)
        .map_err(|_| anyhow::anyhow!("Failed to read AENA data sheet"))?;

    let pax = extract_airport_pax(&range, search_terms);

    match pax {
        Some(total_pax) => {
            sqlx::query(
                "INSERT INTO pax_yearly (airport_id, year, total_pax, source)
                 VALUES ($1, $2, $3, 'aena')
                 ON CONFLICT (airport_id, year) DO UPDATE SET
                     total_pax = GREATEST(EXCLUDED.total_pax, pax_yearly.total_pax)",
            )
            .bind(airport_id)
            .bind(year)
            .bind(total_pax)
            .execute(pool)
            .await
            .with_context(|| format!("Failed to upsert AENA pax for year {}", year))?;

            Ok(1)
        }
        None => Ok(0),
    }
}
