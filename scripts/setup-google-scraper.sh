#!/bin/bash
set -euo pipefail

# Clones google-reviews-scraper-pro as a sibling directory and sets up its venv.
# Run once from the airports-scoring repo root:
#   bash scripts/setup-google-scraper.sh

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SCRAPER_DIR="$REPO_ROOT/../google-reviews-scraper-pro"

if [ -d "$SCRAPER_DIR" ]; then
  echo "Scraper repo already exists at $SCRAPER_DIR — pulling latest..."
  cd "$SCRAPER_DIR" && git pull
else
  echo "Cloning google-reviews-scraper-pro..."
  git clone https://github.com/georgekhananaev/google-reviews-scraper-pro.git "$SCRAPER_DIR"
fi

cd "$SCRAPER_DIR"

if [ ! -d ".venv" ]; then
  echo "Creating Python venv..."
  # Needs Python <=3.13 for pre-built pydantic-core wheels
  if command -v python3.13 &>/dev/null; then
    python3.13 -m venv .venv
  elif command -v python3.12 &>/dev/null; then
    python3.12 -m venv .venv
  else
    python3 -m venv .venv
  fi
fi

echo "Installing dependencies..."
source .venv/bin/activate
pip install -r requirements.txt

# Disable MongoDB — we read reviews from the job API response, not Mongo
if [ -f config.yaml ]; then
  sed -i '' 's/use_mongodb: true/use_mongodb: false/' config.yaml
  echo "Disabled MongoDB in config.yaml (not needed)"
fi

echo ""
echo "Setup complete."
echo "Start the scraper with: bash scripts/start-google-scraper.sh"
echo "Or manually: cd $SCRAPER_DIR && source .venv/bin/activate && python api_server.py"
