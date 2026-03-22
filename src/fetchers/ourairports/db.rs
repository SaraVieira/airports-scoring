use std::collections::HashMap;
use anyhow::{Context, Result};
use rust_decimal::Decimal;
use sqlx::PgPool;
use tracing::info;
use super::types::{
    parse_bool_field, parse_opt_decimal, parse_opt_f64, parse_opt_i32, CsvAirport, CsvFrequency,
    CsvNavaid, CsvRunway,
};

/// Upsert seed airports into Postgres and return the OA-id-to-DB-id mapping.
pub(super) async fn upsert_airports(
    pool: &PgPool,
    seed_airports: &[&CsvAirport],
) -> Result<(i32, HashMap<i64, i32>)> {
    let mut records: i32 = 0;
    let mut oa_id_to_db_id: HashMap<i64, i32> = HashMap::new();
    for airport in seed_airports {
        let lat = airport.latitude_deg.unwrap_or(0.0);
        let lon = airport.longitude_deg.unwrap_or(0.0);
        let iata = airport.iata_code.as_deref().unwrap_or("");
        let country = airport.iso_country.as_deref().unwrap_or("XX");
        let municipality = airport.municipality.as_deref().unwrap_or("Unknown");
        let scheduled = airport
            .scheduled_service
            .as_deref()
            .map(|v| v == "yes")
            .unwrap_or(false);

        let row: (i32,) = sqlx::query_as(
            r#"
            INSERT INTO airports (
                iata_code, icao_code, ourairports_id,
                name, city, country_code,
                location, elevation_ft,
                airport_type, scheduled_service,
                wikipedia_url, website_url,
                in_seed_set
            ) VALUES (
                $1, $2, $3,
                $4, $5, $6,
                ST_MakePoint($7, $8)::geography, $9,
                $10, $11,
                $12, $13,
                TRUE
            )
            ON CONFLICT (iata_code) DO UPDATE SET
                icao_code        = EXCLUDED.icao_code,
                ourairports_id   = EXCLUDED.ourairports_id,
                name             = EXCLUDED.name,
                city             = EXCLUDED.city,
                country_code     = EXCLUDED.country_code,
                location         = EXCLUDED.location,
                elevation_ft     = EXCLUDED.elevation_ft,
                airport_type     = EXCLUDED.airport_type,
                scheduled_service = EXCLUDED.scheduled_service,
                wikipedia_url    = EXCLUDED.wikipedia_url,
                website_url      = EXCLUDED.website_url,
                in_seed_set      = TRUE,
                updated_at       = NOW()
            RETURNING id
            "#,
        )
        .bind(iata)
        .bind(airport.gps_code.as_deref().or(Some(&airport.ident)))
        .bind(
            i32::try_from(airport.id)
                .with_context(|| format!("OurAirports ID {} exceeds i32 range", airport.id))?,
        )
        .bind(&airport.name)
        .bind(municipality)
        .bind(country)
        .bind(lon) // ST_MakePoint takes (lon, lat)
        .bind(lat)
        .bind(parse_opt_i32(&airport.elevation_ft))
        .bind(&airport.airport_type)
        .bind(scheduled)
        .bind(airport.wikipedia_link.as_deref())
        .bind(airport.home_link.as_deref())
        .fetch_one(pool)
        .await
        .with_context(|| format!("Failed to upsert airport {}", iata))?;

        oa_id_to_db_id.insert(airport.id, row.0);
        records += 1;
    }
    info!(count = records, "Upserted airports");
    Ok((records, oa_id_to_db_id))
}

/// Delete-and-reinsert runways for seed airports.
pub(super) async fn insert_runways(
    pool: &PgPool,
    csv_runways: &[CsvRunway],
    seed_oa_ids: &HashMap<i64, &str>,
    oa_id_to_db_id: &HashMap<i64, i32>,
) -> Result<i32> {
    let mut runway_count: i32 = 0;
    let mut runways_by_airport: HashMap<i64, Vec<&CsvRunway>> = HashMap::new();
    for rwy in csv_runways {
        if seed_oa_ids.contains_key(&rwy.airport_ref) {
            runways_by_airport
                .entry(rwy.airport_ref)
                .or_default()
                .push(rwy);
        }
    }
    for (oa_id, rwys) in &runways_by_airport {
        let db_id = match oa_id_to_db_id.get(oa_id) {
            Some(id) => *id,
            None => continue,
        };
        // Delete existing runways for this airport.
        sqlx::query("DELETE FROM runways WHERE airport_id = $1")
            .bind(db_id)
            .execute(pool)
            .await
            .context("Failed to delete existing runways")?;

        for rwy in rwys {
            let ident = match (&rwy.le_ident, &rwy.he_ident) {
                (Some(le), Some(he)) => Some(format!("{}/{}", le, he)),
                (Some(le), None) => Some(le.clone()),
                (None, Some(he)) => Some(he.clone()),
                (None, None) => None,
            };

            sqlx::query(
                r#"
                INSERT INTO runways (
                    airport_id, ident, le_ident, he_ident,
                    length_ft, width_ft, surface, lighted, closed,
                    le_latitude_deg, le_longitude_deg, le_elevation_ft,
                    "le_heading_degT", le_displaced_threshold_ft,
                    he_latitude_deg, he_longitude_deg, he_elevation_ft,
                    "he_heading_degT", he_displaced_threshold_ft
                ) VALUES (
                    $1, $2, $3, $4,
                    $5, $6, $7, $8, $9,
                    $10, $11, $12, $13, $14,
                    $15, $16, $17, $18, $19
                )
                "#,
            )
            .bind(db_id)
            .bind(&ident)
            .bind(rwy.le_ident.as_deref())
            .bind(rwy.he_ident.as_deref())
            .bind(parse_opt_i32(&rwy.length_ft))
            .bind(parse_opt_i32(&rwy.width_ft))
            .bind(rwy.surface.as_deref())
            .bind(parse_bool_field(&rwy.lighted))
            .bind(parse_bool_field(&rwy.closed))
            .bind(parse_opt_f64(&rwy.le_latitude_deg))
            .bind(parse_opt_f64(&rwy.le_longitude_deg))
            .bind(parse_opt_i32(&rwy.le_elevation_ft))
            .bind(parse_opt_decimal(&rwy.le_heading_deg_t))
            .bind(parse_opt_i32(&rwy.le_displaced_threshold_ft))
            .bind(parse_opt_f64(&rwy.he_latitude_deg))
            .bind(parse_opt_f64(&rwy.he_longitude_deg))
            .bind(parse_opt_i32(&rwy.he_elevation_ft))
            .bind(parse_opt_decimal(&rwy.he_heading_deg_t))
            .bind(parse_opt_i32(&rwy.he_displaced_threshold_ft))
            .execute(pool)
            .await
            .context("Failed to insert runway")?;

            runway_count += 1;
        }
    }
    info!(count = runway_count, "Inserted runways");
    Ok(runway_count)
}

/// Delete-and-reinsert frequencies for seed airports.
pub(super) async fn insert_frequencies(
    pool: &PgPool,
    csv_frequencies: &[CsvFrequency],
    seed_oa_ids: &HashMap<i64, &str>,
    oa_id_to_db_id: &HashMap<i64, i32>,
) -> Result<i32> {
    let mut freq_count: i32 = 0;
    let mut freqs_by_airport: HashMap<i64, Vec<&CsvFrequency>> = HashMap::new();
    for freq in csv_frequencies {
        if seed_oa_ids.contains_key(&freq.airport_ref) {
            freqs_by_airport
                .entry(freq.airport_ref)
                .or_default()
                .push(freq);
        }
    }
    for (oa_id, freqs) in &freqs_by_airport {
        let db_id = match oa_id_to_db_id.get(oa_id) {
            Some(id) => *id,
            None => continue,
        };
        // Delete existing frequencies for this airport.
        sqlx::query("DELETE FROM frequencies WHERE airport_id = $1")
            .bind(db_id)
            .execute(pool)
            .await
            .context("Failed to delete existing frequencies")?;

        for freq in freqs {
            let mhz = match freq.frequency_mhz {
                Some(v) => Decimal::try_from(v).unwrap_or_default(),
                None => continue,
            };

            sqlx::query(
                r#"
                INSERT INTO frequencies (airport_id, freq_type, description, frequency_mhz)
                VALUES ($1, $2, $3, $4)
                "#,
            )
            .bind(db_id)
            .bind(freq.freq_type.as_deref())
            .bind(freq.description.as_deref())
            .bind(mhz)
            .execute(pool)
            .await
            .context("Failed to insert frequency")?;

            freq_count += 1;
        }
    }
    info!(count = freq_count, "Inserted frequencies");
    Ok(freq_count)
}

/// Delete-and-reinsert navaids for seed airports.
pub(super) async fn insert_navaids(
    pool: &PgPool,
    csv_navaids: &[CsvNavaid],
    seed_airports: &[&CsvAirport],
    oa_id_to_db_id: &HashMap<i64, i32>,
) -> Result<i32> {
    let mut navaid_count: i32 = 0;
    // Build a reverse map from ICAO ident -> db airport id.
    let mut icao_to_db_id: HashMap<String, i32> = HashMap::new();
    for airport in seed_airports {
        let icao = airport.gps_code.as_deref().unwrap_or(&airport.ident);
        if let Some(&db_id) = oa_id_to_db_id.get(&airport.id) {
            icao_to_db_id.insert(icao.to_string(), db_id);
        }
    }
    let mut navaids_by_airport: HashMap<String, Vec<&CsvNavaid>> = HashMap::new();
    for nav in csv_navaids {
        if let Some(ref assoc) = nav.associated_airport {
            if icao_to_db_id.contains_key(assoc) {
                navaids_by_airport
                    .entry(assoc.clone())
                    .or_default()
                    .push(nav);
            }
        }
    }

    for (icao, navs) in &navaids_by_airport {
        let db_id = match icao_to_db_id.get(icao) {
            Some(id) => *id,
            None => continue,
        };

        // Delete existing navaids for this airport.
        sqlx::query("DELETE FROM navaids WHERE airport_id = $1")
            .bind(db_id)
            .execute(pool)
            .await
            .context("Failed to delete existing navaids")?;

        for nav in navs {
            sqlx::query(
                r#"
                INSERT INTO navaids (
                    airport_id, ident, name, navaid_type,
                    frequency_khz, latitude_deg, longitude_deg,
                    elevation_ft, associated_airport_icao
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                "#,
            )
            .bind(db_id)
            .bind(nav.ident.as_deref())
            .bind(nav.name.as_deref())
            .bind(nav.navaid_type.as_deref())
            .bind(parse_opt_i32(&nav.frequency_khz))
            .bind(nav.latitude_deg)
            .bind(nav.longitude_deg)
            .bind(parse_opt_i32(&nav.elevation_ft))
            .bind(nav.associated_airport.as_deref())
            .execute(pool)
            .await
            .context("Failed to insert navaid")?;

            navaid_count += 1;
        }
    }
    info!(count = navaid_count, "Inserted navaids");
    Ok(navaid_count)
}
