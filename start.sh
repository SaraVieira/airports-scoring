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

# Rust API (auto-recompile on changes)
cargo watch -i web/ -i design.pen -i .claude/ -x 'run -- serve --port 3001' &

# Frontend
cd web && pnpm dev &

wait
