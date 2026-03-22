#!/bin/bash
set -euo pipefail

# Starts the google-reviews-scraper-pro REST API on http://localhost:8000
# Assumes setup-google-scraper.sh has been run first.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SCRAPER_DIR="$REPO_ROOT/../google-reviews-scraper-pro"

if [ ! -d "$SCRAPER_DIR" ]; then
  echo "Error: Scraper not found at $SCRAPER_DIR"
  echo "Run 'bash scripts/setup-google-scraper.sh' first."
  exit 1
fi

cd "$SCRAPER_DIR"
source .venv/bin/activate

echo "Starting Google Reviews scraper on http://localhost:8000..."
echo "API docs at http://localhost:8000/docs"
echo ""
python api_server.py
