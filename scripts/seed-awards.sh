#!/usr/bin/env bash
# Seed airport awards from skytrax_awards.json and aci_asq_awards.json
# Usage: bash scripts/seed-awards.sh
# Requires: jq, psql, DATABASE_URL env var

set -euo pipefail

if [ -f .env ]; then
  export $(grep -v '^#' .env | xargs)
fi

DB="${DATABASE_URL:?DATABASE_URL not set}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SKYTRAX_FILE="${SCRIPT_DIR}/../data/skytrax_awards.json"
ACI_FILE="${SCRIPT_DIR}/../data/aci_asq_awards.json"

for f in "$SKYTRAX_FILE" "$ACI_FILE"; do
  if [ ! -f "$f" ]; then
    echo "Error: $f not found"
    exit 1
  fi
done

echo "Truncating airport_awards..."
psql "$DB" -q -c "TRUNCATE airport_awards RESTART IDENTITY;"

echo "Seeding Skytrax awards..."
jq -c '.[]' "$SKYTRAX_FILE" | while IFS= read -r row; do
  iata=$(echo "$row" | jq -r '.iata_code')
  year=$(echo "$row" | jq -r '.year')
  category=$(echo "$row" | jq -r '.category')
  region=$(echo "$row" | jq -r '.region // empty')

  region_val="NULL"
  [ -n "$region" ] && region_val="'$region'"

  psql "$DB" -q -c "INSERT INTO airport_awards (iata_code, source, year, category, region)
    VALUES ('$iata', 'skytrax', $year, '$category', $region_val)
    ON CONFLICT DO NOTHING;" 2>/dev/null
done

echo "Seeding ACI ASQ awards..."
jq -c '.[]' "$ACI_FILE" | while IFS= read -r row; do
  iata=$(echo "$row" | jq -r '.iata_code')
  year=$(echo "$row" | jq -r '.year')
  region=$(echo "$row" | jq -r '.region // empty')
  size_bucket=$(echo "$row" | jq -r '.size_bucket // empty')
  rank=$(echo "$row" | jq -r '.rank // empty')

  region_val="NULL"
  [ -n "$region" ] && region_val="'$region'"

  bucket_val="NULL"
  [ -n "$size_bucket" ] && bucket_val="'$size_bucket'"

  rank_val="NULL"
  [ -n "$rank" ] && rank_val="$rank"

  psql "$DB" -q -c "INSERT INTO airport_awards (iata_code, source, year, category, region, size_bucket, rank)
    VALUES ('$iata', 'aci_asq', $year, 'aci_asq', $region_val, $bucket_val, $rank_val)
    ON CONFLICT DO NOTHING;" 2>/dev/null
done

echo ""
echo "Done. Counts:"
psql "$DB" -c "SELECT source, COUNT(*) as awards, COUNT(DISTINCT iata_code) as airports FROM airport_awards GROUP BY source;"
