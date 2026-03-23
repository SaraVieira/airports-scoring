#!/usr/bin/env bash
set -euo pipefail

# Load DATABASE_URL from .env
if [ -f .env ]; then
  export $(grep -v '^#' .env | xargs)
fi

if [ -z "${DATABASE_URL:-}" ]; then
  echo "ERROR: DATABASE_URL not set"
  exit 1
fi

TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_DIR="backups"
mkdir -p "$BACKUP_DIR"

echo "Backing up database..."
pg_dump "$DATABASE_URL" -Fc -f "$BACKUP_DIR/airports_${TIMESTAMP}.dump"
pg_dump "$DATABASE_URL" --no-owner -f "$BACKUP_DIR/airports_${TIMESTAMP}.sql"

echo "Backups saved to:"
echo "  $BACKUP_DIR/airports_${TIMESTAMP}.dump (custom format)"
echo "  $BACKUP_DIR/airports_${TIMESTAMP}.sql (plain SQL)"

# Verify the dump is restorable (dry run)
pg_restore --list "$BACKUP_DIR/airports_${TIMESTAMP}.dump" > /dev/null
echo "Backup verified successfully."
