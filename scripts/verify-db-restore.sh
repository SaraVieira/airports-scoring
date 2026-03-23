#!/usr/bin/env bash
set -euo pipefail

# Verify that a restored database has the same row counts as the original.
# Usage: ./scripts/verify-db-restore.sh <OLD_DATABASE_URL> <NEW_DATABASE_URL>

OLD_DB="${1:?Usage: verify-db-restore.sh <OLD_DATABASE_URL> <NEW_DATABASE_URL>}"
NEW_DB="${2:?Usage: verify-db-restore.sh <OLD_DATABASE_URL> <NEW_DATABASE_URL>}"

TABLES=(
  airports organisations countries regions
  pax_yearly operational_stats sentiment_snapshots airport_scores
  reviews_raw routes runways metar_daily
  wikipedia_snapshots carbon_accreditation ground_transport
  lounges hub_status pipeline_runs
  frequencies navaids airport_slugs all_airports
)

echo "Comparing row counts between OLD and NEW databases..."
echo ""

PASS=0
FAIL=0

for table in "${TABLES[@]}"; do
  OLD_COUNT=$(psql "$OLD_DB" -t -c "SELECT count(*) FROM $table;" 2>/dev/null | tr -d ' ' || echo "N/A")
  NEW_COUNT=$(psql "$NEW_DB" -t -c "SELECT count(*) FROM $table;" 2>/dev/null | tr -d ' ' || echo "N/A")

  if [ "$OLD_COUNT" = "$NEW_COUNT" ]; then
    echo "  ✓ $table: $OLD_COUNT rows"
    PASS=$((PASS + 1))
  else
    echo "  ✗ $table: OLD=$OLD_COUNT NEW=$NEW_COUNT"
    FAIL=$((FAIL + 1))
  fi
done

echo ""
echo "Results: $PASS passed, $FAIL failed"

if [ "$FAIL" -gt 0 ]; then
  echo "WARNING: Row count mismatches detected. Investigate before proceeding."
  exit 1
fi

echo "All tables match. Safe to proceed."
