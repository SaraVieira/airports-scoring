#!/bin/bash

# Start all dev services in parallel with colored output
# Ctrl+C stops everything

command -v cargo-watch >/dev/null 2>&1 || { echo "Install cargo-watch first: cargo install cargo-watch"; exit 1; }

trap 'kill 0; exit' SIGINT SIGTERM

# Docker services (Postgres + Google Scraper)
docker compose -f docker-compose.dev.yml up &

# Wait for Postgres to be ready
echo "Waiting for Postgres..."
until docker exec airports-scoring-postgres-1 pg_isready -U airports > /dev/null 2>&1; do
  sleep 1
done
echo "Postgres ready"

# Seed database if tables don't exist (first run only)
DB_URL="postgres://airports:airports@localhost:5433/airports"
TABLE_EXISTS=$(psql "$DB_URL" -tAc "SELECT EXISTS(SELECT 1 FROM information_schema.tables WHERE table_name='airports')" 2>/dev/null)
if [ "$TABLE_EXISTS" != "t" ]; then
  echo "First run detected — seeding database..."
  psql "$DB_URL" -q < scripts/airport_schema.sql
  DATABASE_URL="$DB_URL" bash scripts/seed-all-airports.sh
  DATABASE_URL="$DB_URL" bash scripts/seed-reference-data.sh
  echo "Database seeded"
fi

# Rust API (auto-recompile on changes)
cargo watch -i web/ -i design.pen -i .claude/ -x 'run -- serve --port 3001' &

# Frontend
cd web && pnpm dev &

wait
