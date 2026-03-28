#!/usr/bin/env bash
# Seed the all_airports reference table from mwgg/Airports GitHub repo.
# Usage: bash scripts/seed-all-airports.sh
# Requires: curl, jq, psql (or pass DB connection via DATABASE_URL)

set -euo pipefail

# Load .env from repo root
if [ -f .env ]; then
  export $(grep -v '^#' .env | xargs)
fi

DB="${DATABASE_URL:?DATABASE_URL not set}"
JSON_URL="https://raw.githubusercontent.com/mwgg/Airports/refs/heads/master/airports.json"

echo "Fetching airports JSON..."
TMPFILE=$(mktemp)
curl -sL "$JSON_URL" -o "$TMPFILE"

COUNT=$(jq 'to_entries | map(select(.value.icao and (.value.icao | length) == 4)) | length' "$TMPFILE")
echo "Found $COUNT airports with valid ICAO codes"

echo "Creating table and inserting..."
psql "$DB" -q <<'SQL'
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
);
CREATE INDEX IF NOT EXISTS idx_all_airports_iata ON all_airports(iata);
TRUNCATE all_airports;
SQL

# Convert JSON to CSV and COPY into Postgres
jq -r '
  to_entries[]
  | select(.value.icao and (.value.icao | length) == 4)
  | [
      .value.icao,
      (if .value.iata and (.value.iata | length) > 0 then .value.iata else "" end),
      .value.name,
      .value.city,
      .value.country,
      (.value.elevation // 0),
      .value.lat,
      .value.lon,
      .value.tz
    ]
  | @csv
' "$TMPFILE" | psql "$DB" -q -c "COPY all_airports (icao, iata, name, city, country, elevation, lat, lon, tz) FROM STDIN WITH (FORMAT csv)"

rm "$TMPFILE"

INSERTED=$(psql "$DB" -tA -c "SELECT count(*) FROM all_airports")
echo "Done — $INSERTED airports seeded"
