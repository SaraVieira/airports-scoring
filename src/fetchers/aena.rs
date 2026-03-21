use anyhow::{Context, Result, bail};
use calamine::{open_workbook_auto, open_workbook_auto_from_rs, Reader, Data};
use sqlx::PgPool;
use std::path::Path;
use tracing::{info, warn};

use crate::fetchers::wikipedia::USER_AGENT;
use crate::models::{Airport, FetchResult};

const MADRID_ICAO: &str = "LEMD";
const BARCELONA_ICAO: &str = "LEBL";

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

/// Mapping from filename patterns to years for local files.
fn year_from_filename(filename: &str) -> Option<i16> {
    let lower = filename.to_lowercase();
    // Try to extract a 4-digit year from the filename.
    let re = regex::Regex::new(r"(20\d{2})").ok()?;
    re.captures(&lower)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse::<i16>().ok())
}

/// Fetch AENA traffic data for MAD or BCN only.
/// First tries local files in data/aena/, then falls back to remote download.
pub async fn fetch(pool: &PgPool, airport: &Airport, _full_refresh: bool) -> Result<FetchResult> {
    let icao = airport
        .icao_code
        .as_deref()
        .context("Airport has no ICAO code")?;
    let iata = airport
        .iata_code
        .as_deref()
        .unwrap_or("???");

    if icao != MADRID_ICAO && icao != BARCELONA_ICAO {
        info!(airport = iata, "AENA fetcher only supports MAD and BCN, skipping");
        return Ok(FetchResult {
            records_processed: 0,
            last_record_date: None,
        });
    }

    let mut total_records: i32 = 0;
    let mut last_date = None;

    // 1. Parse local files first.
    let local_dir = Path::new(LOCAL_DATA_DIR);
    if local_dir.is_dir() {
        let mut entries: Vec<_> = std::fs::read_dir(local_dir)?
            .filter_map(|e| e.ok())
            .collect();
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

            match parse_aena_file(&path, airport.id, icao, year, pool).await {
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
            match fetch_remote_aena(&client, &url, airport.id, icao, year, pool).await {
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
    icao: &str,
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

    parse_workbook(workbook, airport_id, icao, year, pool).await
}

async fn parse_aena_file(
    path: &Path,
    airport_id: i32,
    icao: &str,
    year: i16,
    pool: &PgPool,
) -> Result<i32> {
    let workbook = open_workbook_auto(path)
        .with_context(|| format!("Failed to open AENA file: {:?}", path))?;

    parse_workbook(workbook, airport_id, icao, year, pool).await
}

async fn parse_workbook<RS: std::io::Read + std::io::Seek, R: Reader<RS>>(
    mut workbook: R,
    airport_id: i32,
    icao: &str,
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

    let pax = extract_airport_pax(&range, icao);

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

/// Airport name patterns used across AENA reports over the years.
const MADRID_NAMES: &[&str] = &[
    "adolfo suárez madrid-barajas",
    "adolfo suarez madrid-barajas",
    "madrid-barajas",
];

const BARCELONA_NAMES: &[&str] = &[
    "barcelona-el prat j.t.",
    "barcelona-el prat",
    "barcelona",
];

/// Extract passenger count for a specific airport from an AENA worksheet.
///
/// AENA annual reports have a consistent structure:
/// - Rows are sorted by passenger volume (busiest airport first)
/// - Each row has: airport name, total passengers, % change, ...
///   (repeated for passengers, operations, cargo in adjacent column groups)
/// - The airport name may be in column 0 or column 1 (varies by year)
/// - The total passengers is always the first large integer after the name
pub fn extract_airport_pax(range: &calamine::Range<Data>, icao: &str) -> Option<i64> {
    let target_names: &[&str] = if icao == MADRID_ICAO {
        MADRID_NAMES
    } else {
        BARCELONA_NAMES
    };

    for row in range.rows() {
        // Collect all string cells to check for airport name match.
        let row_strings: Vec<String> = row
            .iter()
            .filter_map(|cell| match cell {
                Data::String(s) => Some(s.trim().to_lowercase()),
                _ => None,
            })
            .collect();

        let matches_airport = row_strings.iter().any(|s| {
            target_names.iter().any(|name| s == name)
        });

        if !matches_airport {
            continue;
        }

        // For Barcelona, make sure we don't match a row where "barcelona" appears
        // only in a cargo/operations column (not the passengers column group).
        // The passenger airport name is always one of the first few string cells.
        let first_string = row_strings.first()?;
        let is_pax_row = target_names.iter().any(|name| first_string == name);
        if !is_pax_row {
            // The first string doesn't match — this might be an ops/cargo row
            // where the same airport name appears in a later column group.
            // Only skip if the row has multiple airport name occurrences.
            let match_count = row_strings.iter()
                .filter(|s| target_names.iter().any(|name| *s == name))
                .count();
            if match_count > 1 {
                // Multi-section row (pax + ops + cargo) — the first match IS the pax section.
                // Fall through to extract the number.
            } else {
                continue;
            }
        }

        // Extract the first large number from the row — this is the passenger total.
        // Skip percentage values (floats between -1 and 1) and small numbers.
        for cell in row {
            match cell {
                Data::Float(f) => {
                    let n = *f as i64;
                    if n > 100_000 {
                        return Some(n);
                    }
                }
                Data::Int(n) => {
                    if *n > 100_000 {
                        return Some(*n);
                    }
                }
                _ => {}
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn data_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/aena")
    }

    fn open_test_file(filename: &str) -> calamine::Sheets<std::io::BufReader<std::fs::File>> {
        let path = data_dir().join(filename);
        open_workbook_auto(&path).unwrap_or_else(|e| panic!("Failed to open {}: {}", filename, e))
    }

    fn get_data_range(wb: &mut calamine::Sheets<std::io::BufReader<std::fs::File>>) -> calamine::Range<Data> {
        let sheet = wb.sheet_names().to_vec()
            .into_iter()
            .find(|s| s.to_lowercase() != "mozart reports")
            .expect("No data sheet");
        wb.worksheet_range(&sheet).expect("Failed to read sheet")
    }

    #[test]
    fn year_from_filename_works() {
        assert_eq!(year_from_filename("DEFINITIVOS+2024.xlsx"), Some(2024));
        assert_eq!(year_from_filename("0.Anual_Definitivo_2018.xls"), Some(2018));
        assert_eq!(year_from_filename("TOTAL_2004.xls"), Some(2004));
        assert_eq!(year_from_filename("PROVISIONALES+2025.xlsx"), Some(2025));
        assert_eq!(year_from_filename("no_year_here.xls"), None);
    }

    #[test]
    fn parse_2024_madrid() {
        let mut wb = open_test_file("DEFINITIVOS+2024.xlsx");
        let range = get_data_range(&mut wb);
        let pax = extract_airport_pax(&range, MADRID_ICAO);
        assert_eq!(pax, Some(66197066), "MAD 2024 expected 66,197,066");
    }

    #[test]
    fn parse_2024_barcelona() {
        let mut wb = open_test_file("DEFINITIVOS+2024.xlsx");
        let range = get_data_range(&mut wb);
        let pax = extract_airport_pax(&range, BARCELONA_ICAO);
        assert_eq!(pax, Some(55037892), "BCN 2024 expected 55,037,892");
    }

    #[test]
    fn parse_2023_madrid() {
        let mut wb = open_test_file("DEFINITIVOS_2023.xlsx");
        let range = get_data_range(&mut wb);
        let pax = extract_airport_pax(&range, MADRID_ICAO);
        assert_eq!(pax, Some(60221163), "MAD 2023 expected 60,221,163");
    }

    #[test]
    fn parse_2023_barcelona() {
        let mut wb = open_test_file("DEFINITIVOS_2023.xlsx");
        let range = get_data_range(&mut wb);
        let pax = extract_airport_pax(&range, BARCELONA_ICAO);
        assert_eq!(pax, Some(49910900), "BCN 2023 expected 49,910,900");
    }

    #[test]
    fn parse_2019_madrid() {
        let mut wb = open_test_file("00.Definitivo_2019.xls");
        let range = get_data_range(&mut wb);
        let pax = extract_airport_pax(&range, MADRID_ICAO);
        assert_eq!(pax, Some(61734944), "MAD 2019 expected 61,734,944");
    }

    #[test]
    fn parse_2019_barcelona() {
        let mut wb = open_test_file("00.Definitivo_2019.xls");
        let range = get_data_range(&mut wb);
        let pax = extract_airport_pax(&range, BARCELONA_ICAO);
        assert_eq!(pax, Some(52688455), "BCN 2019 expected 52,688,455");
    }

    #[test]
    fn parse_2018_madrid() {
        let mut wb = open_test_file("0.Anual_Definitivo_2018.xls");
        let range = get_data_range(&mut wb);
        let pax = extract_airport_pax(&range, MADRID_ICAO);
        assert_eq!(pax, Some(57890057), "MAD 2018 expected 57,890,057");
    }

    #[test]
    fn parse_2014_madrid() {
        let mut wb = open_test_file("Definitivo+2014.xls");
        let range = get_data_range(&mut wb);
        let pax = extract_airport_pax(&range, MADRID_ICAO);
        assert_eq!(pax, Some(41833686), "MAD 2014 expected 41,833,686");
    }

    #[test]
    fn parse_2014_barcelona() {
        let mut wb = open_test_file("Definitivo+2014.xls");
        let range = get_data_range(&mut wb);
        let pax = extract_airport_pax(&range, BARCELONA_ICAO);
        assert_eq!(pax, Some(37558981), "BCN 2014 expected 37,558,981");
    }

    #[test]
    fn parse_2008_madrid() {
        let mut wb = open_test_file("12.Estadistica_Diciembre_2008.xls");
        let range = get_data_range(&mut wb);
        let pax = extract_airport_pax(&range, MADRID_ICAO);
        assert_eq!(pax, Some(50846494), "MAD 2008 expected 50,846,494");
    }

    #[test]
    fn parse_2008_barcelona() {
        let mut wb = open_test_file("12.Estadistica_Diciembre_2008.xls");
        let range = get_data_range(&mut wb);
        let pax = extract_airport_pax(&range, BARCELONA_ICAO);
        assert_eq!(pax, Some(30272084), "BCN 2008 expected 30,272,084");
    }

    #[test]
    fn parse_2004_madrid() {
        let mut wb = open_test_file("TOTAL_2004.xls");
        let range = get_data_range(&mut wb);
        let pax = extract_airport_pax(&range, MADRID_ICAO);
        assert!(pax.is_some(), "MAD 2004 should have data");
        assert!(pax.unwrap() > 30_000_000, "MAD 2004 should be >30M, got {:?}", pax);
    }

    #[test]
    fn parse_2025_provisional_madrid() {
        let mut wb = open_test_file("PROVISIONALES+2025.xlsx");
        let range = get_data_range(&mut wb);
        let pax = extract_airport_pax(&range, MADRID_ICAO);
        assert!(pax.is_some(), "MAD 2025 provisional should have data");
        assert!(pax.unwrap() > 10_000_000, "MAD 2025 should be >10M, got {:?}", pax);
    }
}
