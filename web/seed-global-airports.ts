import dotenv from "dotenv";
import path from "path";
import pg from "pg";

dotenv.config({ path: path.resolve(import.meta.dirname, "..", ".env") });

const DATABASE_URL = process.env.DATABASE_URL;
if (!DATABASE_URL) {
  console.error("DATABASE_URL not found in ../.env");
  process.exit(1);
}

const JSON_URL =
  "https://raw.githubusercontent.com/mwgg/Airports/refs/heads/master/airports.json";

interface AirportEntry {
  icao: string;
  iata: string;
  name: string;
  city: string;
  country: string;
  elevation: number;
  lat: number;
  lon: number;
  tz: string;
}

async function main() {
  console.log("Fetching airports JSON...");
  const resp = await fetch(JSON_URL);
  const data: Record<string, AirportEntry> = await resp.json();

  const entries = Object.values(data).filter(
    (a) => a.icao && a.icao.length === 4
  );
  console.log(`Loaded ${entries.length} airports`);

  const client = new pg.Client({ connectionString: DATABASE_URL });
  await client.connect();

  try {
    await client.query("BEGIN");

    // Create the table if it doesn't exist
    await client.query(`
      CREATE TABLE IF NOT EXISTS all_airports (
        icao CHAR(4) PRIMARY KEY,
        iata CHAR(3),
        name TEXT NOT NULL,
        city TEXT NOT NULL,
        country CHAR(2) NOT NULL,
        elevation INTEGER,
        lat DOUBLE PRECISION,
        lon DOUBLE PRECISION,
        tz TEXT
      )
    `);

    // Truncate and re-insert (fast, idempotent)
    await client.query("TRUNCATE all_airports");

    // Batch insert in chunks of 500
    const chunk = 500;
    let inserted = 0;

    for (let i = 0; i < entries.length; i += chunk) {
      const batch = entries.slice(i, i + chunk);
      const values: unknown[] = [];
      const placeholders: string[] = [];

      for (let j = 0; j < batch.length; j++) {
        const a = batch[j];
        const offset = j * 9;
        placeholders.push(
          `($${offset + 1}, $${offset + 2}, $${offset + 3}, $${offset + 4}, $${offset + 5}, $${offset + 6}, $${offset + 7}, $${offset + 8}, $${offset + 9})`
        );
        values.push(
          a.icao.substring(0, 4),
          a.iata ? a.iata.substring(0, 3) : null,
          a.name || a.icao,
          a.city || a.name || a.icao,
          a.country.substring(0, 2),
          a.elevation || null,
          a.lat || null,
          a.lon || null,
          a.tz || null
        );
      }

      await client.query(
        `INSERT INTO all_airports (icao, iata, name, city, country, elevation, lat, lon, tz)
         VALUES ${placeholders.join(", ")}
         ON CONFLICT (icao) DO UPDATE SET
           iata = COALESCE(EXCLUDED.iata, all_airports.iata),
           name = EXCLUDED.name,
           city = EXCLUDED.city,
           country = EXCLUDED.country,
           elevation = EXCLUDED.elevation,
           lat = EXCLUDED.lat,
           lon = EXCLUDED.lon,
           tz = EXCLUDED.tz`,
        values
      );
      inserted += batch.length;
    }

    // Backfill destination_iata on routes using all_airports
    const linkResult = await client.query(`
      UPDATE routes
      SET destination_iata = aa.iata
      FROM all_airports aa
      WHERE routes.destination_icao = aa.icao
        AND routes.destination_iata IS NULL
        AND aa.iata IS NOT NULL
    `);

    await client.query("COMMIT");
    console.log(`Inserted ${inserted} airports into all_airports`);
    console.log(`Linked ${linkResult.rowCount} route destination_iata values`);
  } catch (err) {
    await client.query("ROLLBACK");
    throw err;
  } finally {
    await client.end();
  }
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
