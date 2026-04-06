//! Finavia (Finland) passenger data fetcher.
//!
//! Downloads the "Passengers by Airport" Excel file from finavia.fi.
//! Contains monthly domestic/international/total passenger counts for all
//! 20 Finavia-operated airports. Data updated monthly.

use anyhow::{Context, Result};
use calamine::{open_workbook_auto_from_rs, Data, Reader};
use chrono::{Datelike, NaiveDate, Utc};
use sqlx::PgPool;
use std::io::Cursor;
use tracing::info;

use crate::models::{Airport, FetchResult};

const FINAVIA_XLSX_URL: &str =
    "https://www.finavia.fi/sites/default/files/documents/Passengers%20by%20Airport-fi_69.xlsx";

const UA: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36";

/// Map Finavia airport names (as they appear in the Excel) to IATA codes.
fn name_to_iata(name: &str) -> Option<&'static str> {
    let lower = name.trim().to_lowercase();
    match lower.as_str() {
        "helsinki" => Some("HEL"),
        "rovaniemi" => Some("RVN"),
        "kittilä" | "kittila" => Some("KTT"),
        "oulu" => Some("OUL"),
        "ivalo" => Some("IVL"),
        "turku" => Some("TKU"),
        "kuusamo" => Some("KAO"),
        "vaasa" => Some("VAA"),
        "kuopio" => Some("KUO"),
        "tampere" => Some("TMP"),
        "kemi-tornio" => Some("KEM"),
        "kajaani" => Some("KAJ"),
        "joensuu" => Some("JOE"),
        "mariehamn" => Some("MHQ"),
        "kokkola-pietarsaari" => Some("KOK"),
        "jyväskylä" | "jyvaskyla" => Some("JYV"),
        "pori" => Some("POR"),
        "savonlinna" => Some("SVL"),
        "halli kuorevesi" | "halli" => Some("KEV"),
        "utti" => Some("UTI"),
        _ => None,
    }
}

/// Fetch passenger data from Finavia for a Finnish airport.
/// Only runs for airports with country_code "FI".
pub async fn fetch(pool: &PgPool, airport: &Airport, _full_refresh: bool) -> Result<FetchResult> {
    let iata = airport
        .iata_code
        .as_deref()
        .context("Airport has no IATA code")?;

    if airport.country_code != "FI" {
        return Ok(FetchResult {
            records_processed: 0,
            last_record_date: None,
        });
    }

    info!(airport = iata, "Fetching Finavia passenger data");

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent(UA)
        .build()?;

    let resp = client
        .get(FINAVIA_XLSX_URL)
        .header("Accept", "*/*")
        .send()
        .await
        .context("Failed to download Finavia Excel")?;

    if !resp.status().is_success() {
        anyhow::bail!("Finavia download returned {}", resp.status());
    }

    let bytes = resp.bytes().await?;
    info!(airport = iata, size = bytes.len(), "Downloaded Finavia Excel");

    // Parse the Excel file
    let cursor = Cursor::new(&bytes[..]);
    let mut workbook = open_workbook_auto_from_rs(cursor)
        .context("Failed to parse Finavia Excel")?;

    let sheet_name = workbook
        .sheet_names()
        .first()
        .cloned()
        .context("No sheets in Finavia Excel")?;

    let range = workbook
        .worksheet_range(&sheet_name)
        .context("Failed to read Finavia worksheet")?;

    // The Excel has a specific layout:
    // Row 1: "Passenger volumes by airports"
    // Row 2: month headers (MM/YY format) across columns
    // Row 3: "Domestic" / "International" / "Total" sub-headers
    // Row 4: "Passengers" / "Change-%" sub-sub-headers
    // Row 5+: airport name in col A, data in subsequent columns
    //
    // The "Year to date" and "Total" columns contain cumulative data.
    // We want the "Total" → "Passengers" column for each month period.

    // Find our airport row
    let mut airport_row: Option<usize> = None;
    for (row_idx, row) in range.rows().enumerate() {
        if let Some(cell) = row.first() {
            let cell_str = cell.to_string();
            if let Some(mapped_iata) = name_to_iata(&cell_str) {
                if mapped_iata == iata {
                    airport_row = Some(row_idx);
                    break;
                }
            }
        }
    }

    let airport_row = match airport_row {
        Some(r) => r,
        None => {
            info!(airport = iata, "Airport not found in Finavia Excel");
            return Ok(FetchResult {
                records_processed: 0,
                last_record_date: None,
            });
        }
    };

    // Parse the "Year to date" total from the last "Total" → "Passengers" column
    // The layout repeats: for each time period, there are 6 columns:
    //   Domestic Passengers, Domestic Change-%,
    //   International Passengers, International Change-%,
    //   Total Passengers, Total Change-%
    //
    // We aggregate: find the "Year to date" section's Total Passengers column

    // Read row 2 to find time period headers
    let header_row: Vec<String> = range
        .rows()
        .nth(1)
        .map(|r| r.iter().map(|c| c.to_string()).collect())
        .unwrap_or_default();

    // Find columns that have "Year to date" or specific month headers
    // For simplicity, the last group of 6 columns before "Year to date" is the cumulative
    // The "Year to date" Total Passengers is what we want for annual aggregation

    // Find the "Year to date" column group
    let mut ytd_total_col: Option<usize> = None;
    for (i, h) in header_row.iter().enumerate() {
        if h.contains("Year to date") || h.contains("year to date") {
            // The Total Passengers column is 4 positions after the start of this group
            // (Domestic Pax, Domestic Change, Intl Pax, Intl Change, Total Pax)
            ytd_total_col = Some(i + 4);
            break;
        }
    }

    // Also find domestic and international YTD columns
    let ytd_domestic_col = ytd_total_col.map(|c| c - 4); // Domestic Passengers
    let ytd_intl_col = ytd_total_col.map(|c| c - 2); // International Passengers

    let data_row: Vec<Data> = range
        .rows()
        .nth(airport_row)
        .map(|r| r.to_vec())
        .unwrap_or_default();

    let current_year = Utc::now().year() as i16;

    // Extract YTD data
    fn cell_to_i64(data_row: &[Data], col: Option<usize>) -> Option<i64> {
        col.and_then(|c| data_row.get(c))
            .and_then(|v| match v {
                Data::Float(f) => Some(*f as i64),
                Data::Int(i) => Some(*i),
                _ => None,
            })
    }

    let total_pax = cell_to_i64(&data_row, ytd_total_col);
    let domestic_pax = cell_to_i64(&data_row, ytd_domestic_col);
    let international_pax = cell_to_i64(&data_row, ytd_intl_col);

    let total = total_pax.unwrap_or(0);
    if total == 0 {
        info!(airport = iata, "No passenger data found in Finavia Excel");
        return Ok(FetchResult {
            records_processed: 0,
            last_record_date: None,
        });
    }

    info!(
        airport = iata,
        total_pax = total,
        domestic = domestic_pax,
        international = international_pax,
        year = current_year,
        "Parsed Finavia pax data"
    );

    sqlx::query(
        r#"
        INSERT INTO pax_yearly (airport_id, year, total_pax, domestic_pax, international_pax, source)
        VALUES ($1, $2, $3, $4, $5, 'finavia')
        ON CONFLICT (airport_id, year) DO UPDATE SET
            total_pax = GREATEST(EXCLUDED.total_pax, pax_yearly.total_pax),
            domestic_pax = COALESCE(EXCLUDED.domestic_pax, pax_yearly.domestic_pax),
            international_pax = COALESCE(EXCLUDED.international_pax, pax_yearly.international_pax),
            source = 'finavia'
        "#,
    )
    .bind(airport.id)
    .bind(current_year)
    .bind(total)
    .bind(domestic_pax)
    .bind(international_pax)
    .execute(pool)
    .await
    .with_context(|| format!("Failed to upsert Finavia pax for {iata}"))?;

    let latest = NaiveDate::from_ymd_opt(current_year as i32, 12, 31).unwrap();

    info!(airport = iata, "Finavia pax data upserted");

    Ok(FetchResult {
        records_processed: 1,
        last_record_date: Some(latest),
    })
}
