#!/usr/bin/env bash
# Seed organisations and map them to airports from european_airport_operators.json
# Usage: bash scripts/seed-operators.sh
# Requires: jq, psql, DATABASE_URL env var

set -euo pipefail

if [ -f .env ]; then
  export $(grep -v '^#' .env | xargs)
fi

DB="${DATABASE_URL:?DATABASE_URL not set}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
JSON_FILE="${SCRIPT_DIR}/../data/european_airport_operators.json"

if [ ! -f "$JSON_FILE" ]; then
  echo "Error: $JSON_FILE not found in current directory"
  exit 1
fi

COUNT=$(jq length "$JSON_FILE")
echo "Seeding $COUNT operators from $JSON_FILE..."

# Clean slate: clear operator references then delete all organisations
echo "Clearing existing operator data..."
psql "$DB" -q -c "UPDATE airports SET operator_id = NULL, owner_id = NULL;"
psql "$DB" -q -c "DELETE FROM operator_scores;"
psql "$DB" -q -c "DELETE FROM organisations;"
psql "$DB" -q -c "ALTER SEQUENCE organisations_id_seq RESTART WITH 1;"

# Process each operator
jq -c '.[]' "$JSON_FILE" | while IFS= read -r org; do
  NAME=$(echo "$org" | jq -r '.name')
  SHORT=$(echo "$org" | jq -r '.short_name')
  COUNTRY=$(echo "$org" | jq -r '.country_code')
  ORG_TYPE=$(echo "$org" | jq -r '.org_type')
  OWNERSHIP=$(echo "$org" | jq -r '.ownership_model')
  PUBLIC_PCT=$(echo "$org" | jq -r '.public_share_pct')
  NOTES=$(echo "$org" | jq -r '.notes')

  # Insert organisation with generated slug
  ORG_ID=$(psql "$DB" -tA -c "
    INSERT INTO organisations (name, short_name, country_code, org_type, ownership_model, public_share_pct, notes, slug)
    VALUES (
      \$\$${NAME}\$\$, \$\$${SHORT}\$\$, '${COUNTRY}', '${ORG_TYPE}', '${OWNERSHIP}', ${PUBLIC_PCT}, \$\$${NOTES}\$\$,
      LOWER(TRIM(BOTH '-' FROM REGEXP_REPLACE(
        REGEXP_REPLACE(unaccent(\$\$${NAME}\$\$), '-+', '-', 'g'),
        '[^a-zA-Z0-9]+', '-', 'g'
      )))
    )
    RETURNING id;
  " 2>/dev/null | head -1)

  if [ -z "$ORG_ID" ]; then
    echo "  SKIP: Could not create org: $SHORT"
    continue
  fi

  # Map airports to this operator
  AIRPORTS=$(echo "$org" | jq -r '.airports[]')
  MAPPED=0
  for IATA in $AIRPORTS; do
    RESULT=$(psql "$DB" -tA -c "
      UPDATE airports SET operator_id = ${ORG_ID}
      WHERE iata_code = '${IATA}' AND (operator_id IS NULL OR operator_id != ${ORG_ID})
      RETURNING iata_code;
    " 2>/dev/null | head -1)
    if [ -n "$RESULT" ]; then
      MAPPED=$((MAPPED + 1))
    fi
  done

  TOTAL=$(echo "$org" | jq '.airports | length')
  echo "  ${SHORT}: org_id=${ORG_ID}, mapped ${MAPPED}/${TOTAL} airports"
done

echo "Done — operators seeded and airports mapped"
