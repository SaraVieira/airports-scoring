#!/usr/bin/env bash
# Seed organisations and map them to airports from european_airport_operators.json
# Usage: bash scripts/seed-operators.sh
# Requires: jq, psql, DATABASE_URL env var

set -euo pipefail

if [ -f .env ]; then
  export $(grep -v '^#' .env | xargs)
fi

DB="${DATABASE_URL:?DATABASE_URL not set}"
JSON_FILE="european_airport_operators.json"

if [ ! -f "$JSON_FILE" ]; then
  echo "Error: $JSON_FILE not found in current directory"
  exit 1
fi

COUNT=$(jq length "$JSON_FILE")
echo "Seeding $COUNT operators from $JSON_FILE..."

# Process each operator
jq -c '.[]' "$JSON_FILE" | while IFS= read -r org; do
  NAME=$(echo "$org" | jq -r '.name')
  SHORT=$(echo "$org" | jq -r '.short_name')
  COUNTRY=$(echo "$org" | jq -r '.country_code')
  ORG_TYPE=$(echo "$org" | jq -r '.org_type')
  OWNERSHIP=$(echo "$org" | jq -r '.ownership_model')
  PUBLIC_PCT=$(echo "$org" | jq -r '.public_share_pct')
  NOTES=$(echo "$org" | jq -r '.notes')

  # Upsert organisation — use head -1 to grab just the ID, not the INSERT tag
  ORG_ID=$(psql "$DB" -tA -c "
    INSERT INTO organisations (name, short_name, country_code, org_type, ownership_model, public_share_pct, notes)
    VALUES (\$\$${NAME}\$\$, \$\$${SHORT}\$\$, '${COUNTRY}', '${ORG_TYPE}', '${OWNERSHIP}', ${PUBLIC_PCT}, \$\$${NOTES}\$\$)
    ON CONFLICT DO NOTHING
    RETURNING id;
  " 2>/dev/null | head -1)

  # If no insert (already exists), look it up
  if [ -z "$ORG_ID" ]; then
    ORG_ID=$(psql "$DB" -tA -c "
      SELECT id FROM organisations WHERE name = \$\$${NAME}\$\$ LIMIT 1;
    " 2>/dev/null | head -1)
  fi

  if [ -z "$ORG_ID" ]; then
    echo "  SKIP: Could not find/create org: $SHORT"
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
