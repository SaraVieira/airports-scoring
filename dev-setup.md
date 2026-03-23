# Dev Setup

## Architecture

The platform has three components:

1. **Rust API** (`airport-fetch serve`) — owns all DB access (reads + writes), pipeline orchestration, scoring
2. **TanStack Start frontend** (`web/`) — pure UI, calls the Rust API for all data
3. **Google Reviews scraper** (optional sidecar) — Selenium-based scraper for Google Maps reviews

```
Rust API (:3001) ←→ PostgreSQL + PostGIS
    ↑                    ↑
    |                    |
Frontend (:3000)    Google Scraper (:8000)
```

## Prerequisites

- **Rust** (stable toolchain, 1.85+)
- **Node.js** 20+ with **pnpm**
- **Python** 3.10+ (for sentiment pipeline)
- **PostgreSQL** with PostGIS extension

## Environment

Create a `.env` file in the repo root:

```
DATABASE_URL=postgres://user:pass@localhost:5432/airports
HF_TOKEN=hf_your_huggingface_token
OPENSKY_ID=your_opensky_client_id
OPENSKY_SECRET=your_opensky_client_secret
GOOGLE_SCRAPER_URL=http://localhost:8000
```

Frontend env (in `.env` or set when running):

```
VITE_API_URL=http://localhost:3001
VITE_API_KEY=
```

- **HF_TOKEN**: Get from https://huggingface.co/settings/tokens (free, needed for sentiment pipeline)
- **OPENSKY_ID / OPENSKY_SECRET**: Optional. Register at https://opensky-network.org/
- **GOOGLE_SCRAPER_URL**: Optional. URL of the google-reviews-scraper-pro REST API
- **VITE_API_URL**: Where the Rust API is running (default: `http://localhost:3001`)
- **VITE_API_KEY**: API key for the Rust API (empty = no auth in dev)

## Quick Start

### 1. Database

Make sure PostgreSQL is running with PostGIS enabled. Create the database:

```bash
createdb airports
psql -d airports -c "CREATE EXTENSION IF NOT EXISTS postgis;"
```

### 2. Rust API

```bash
# Install dependencies and build
cargo build

# Run database migrations
sqlx migrate run --source migrations/

# Start the API server on port 3001
cargo run -- serve --port 3001
```

Verify: `curl http://localhost:3001/health` should return `{"status":"ok"}`

### 3. Seed Data (first time only)

The API needs base data. Run the pipeline to bootstrap:

```bash
# Bootstrap airport records from OurAirports
cargo run -- fetch --all --source ourairports

# Fetch Wikipedia data (pax stats, metadata)
cargo run -- fetch --all --source wikipedia --full-refresh

# Seed the 29K global airports for route lookups
cd web && pnpm install && pnpm db:seed-airports && cd ..
```

### 4. Frontend

```bash
cd web
pnpm install
pnpm dev              # Starts on http://localhost:3000
```

The frontend calls the Rust API at `VITE_API_URL` (default `http://localhost:3001`).

### 5. Generate TypeScript Types (after API changes)

When Rust API response types change, regenerate the frontend types:

```bash
# API must be running on :3001
cd web
pnpm generate-types
```

## Google Reviews Scraper (optional)

The scraper is optional — the pipeline skips Google reviews gracefully if it's not running.

```bash
# One-time setup
bash scripts/setup-google-scraper.sh

# Start the scraper (runs on http://localhost:8000)
bash scripts/start-google-scraper.sh
```

## Python Venv (for sentiment)

```bash
python3 -m venv .venv
.venv/bin/pip install -r python/requirements.txt
```

## Running the Pipeline

The binary has two modes: `serve` (API server) and `fetch` (CLI pipeline).

### API Server

```bash
cargo run -- serve --port 3001
```

### CLI Fetch (local development)

```bash
# Single airport, single source
cargo run -- fetch BER --source wikipedia

# All airports, single source
cargo run -- fetch --all --source eurocontrol

# All airports, all sources, then score
cargo run -- fetch --all --score

# Force re-fetch
cargo run -- fetch --all --source wikipedia --full-refresh
```

### Via Admin UI

Once the API is running, go to `http://localhost:3000/admin`:

1. Enter admin password (or anything if `ADMIN_PASSWORD` is not set)
2. Use "Refresh All" to trigger an incremental fetch for all airports
3. Use "Jobs" page to start custom jobs (pick airports + sources)
4. Use "Airports" page to add/remove/edit tracked airports

### Data Sources

| Source | Flag | What it fetches |
|--------|------|----------------|
| **ourairports** | `--source ourairports` | Runways, frequencies, navaids, basic airport info |
| **wikipedia** | `--source wikipedia` | Pax stats, opened year, operator, terminals, Skytrax history, ACI awards |
| **eurocontrol** | `--source eurocontrol` | Monthly flights, delay %, cause breakdown |
| **routes** | `--source routes` | Route network (OPDI + Jonty/FlightRadar24 fallback) |
| **eurostat** | `--source eurostat` | Historical passenger traffic |
| **metar** | `--source metar` | Daily weather (temp, wind, visibility) |
| **reviews** | `--source reviews` | Skytrax + Google reviews (both scrapers) |
| **skytrax** | `--source skytrax` | Skytrax reviews only |
| **google_reviews** | `--source google_reviews` | Google Maps reviews only (needs scraper) |
| **sentiment** | `--source sentiment` | RoBERTa + NLI sentiment analysis on unprocessed reviews |
| **carbon_accreditation** | `--source carbon_accreditation` | ACI carbon accreditation levels |
| **priority_pass** | `--source priority_pass` | Priority Pass lounge data |
| **opensky** | `--source opensky` | Flight movements from OpenSky Network |
| **caa** | `--source caa` | UK CAA passenger statistics |
| **aena** | `--source aena` | Spanish AENA passenger statistics |

### Fresh Setup Pipeline Order

```bash
cargo run -- fetch --all --source ourairports
cargo run -- fetch --all --source wikipedia --full-refresh
cargo run -- fetch --all --source eurostat
cargo run -- fetch --all --source eurocontrol --full-refresh
cargo run -- fetch --all --source routes
cargo run -- fetch --all --source reviews
cargo run -- fetch --all --source sentiment
cargo run -- fetch --all --score
```

### Eurocontrol Delay Files

ATFM delay files are behind antibot protection. Download manually:

1. Go to https://ansperformance.eu/csv/
2. Download `apt_dly_{year}.csv.bz2` files
3. Place in `data/aena/ert_dly/`

## Adding a New Airport

### Via Admin UI (recommended)

1. Go to `http://localhost:3000/admin/airports`
2. Click "+ Add Airport"
3. Fill in IATA, name, country code, Skytrax slugs, Google Maps URL
4. Click "Fetch" to run the pipeline for that airport

### Via Database

Airports are tracked in the `supported_airports` table. The API reads from this table (not `airports.json`).

```sql
INSERT INTO supported_airports (iata_code, country_code, name, skytrax_review_slug, google_maps_url)
VALUES ('JFK', 'US', 'New York JFK', 'new-york-jfk-airport', 'https://maps.app.goo.gl/abc123');
```

Then run the pipeline:

```bash
cargo run -- fetch JFK --source ourairports
cargo run -- fetch JFK --source wikipedia --full-refresh
cargo run -- fetch JFK --score
```

## Deployment (Coolify)

The project deploys as a Docker Compose stack on Coolify:

- **Postgres** (PostGIS 17) — internal, hardcoded credentials
- **Rust API** — builds from `Dockerfile`, runs migrations on startup
- **Google Scraper** — builds from `google-scraper.Dockerfile`

Frontend deploys separately (Vercel, Coolify, etc.) with `VITE_API_URL` pointing to the API.

### Required Env Vars (Coolify)

| Variable | Service | Required |
|----------|---------|----------|
| `API_KEY` | API | Yes (protects all endpoints) |
| `ADMIN_PASSWORD` | API | Yes (protects admin endpoints) |
| `HF_TOKEN` | API | Yes (sentiment pipeline) |
| `OPENSKY_ID` | API | No |
| `OPENSKY_SECRET` | API | No |
| `VITE_API_URL` | Frontend | Yes |
| `VITE_API_KEY` | Frontend | Yes (must match API_KEY) |

### Cron Jobs (Coolify)

Configure HTTP cron jobs in Coolify hitting these endpoints:

| Schedule | Endpoint | What it does |
|----------|----------|-------------|
| Weekly | `POST /api/cron/full-refresh` | Full refresh all airports + score |
| Daily | `POST /api/cron/sentiment` | Sentiment analysis + score |
| Weekly | `POST /api/cron/reviews` | Scrape new reviews |

All cron endpoints require the `X-API-Key` header.

## Project Structure

```
airports-scoring/
├── .env                        # DATABASE_URL + tokens (repo root)
├── airports.json               # Legacy seed list (use supported_airports table instead)
├── Dockerfile                  # Rust API + Python (multi-stage)
├── docker-compose.yml          # Postgres + API + Google Scraper
├── google-scraper.Dockerfile   # Google Reviews scraper
├── migrations/                 # sqlx migrations (auto-run on API startup)
├── src/                        # Rust API + pipeline
│   ├── main.rs                 # CLI: serve / fetch subcommands
│   ├── server/                 # Axum API server
│   │   ├── mod.rs              # Router, OpenAPI, startup
│   │   ├── auth.rs             # API key + admin password middleware
│   │   ├── jobs.rs             # Background job manager
│   │   └── routes/             # Endpoint handlers
│   │       ├── airports.rs     # Public read endpoints
│   │       ├── admin.rs        # Admin CRUD + job management
│   │       └── cron.rs         # Cron triggers
│   ├── pipeline.rs             # Fetch orchestration
│   ├── scoring/                # Composite score computation
│   ├── fetchers/               # One module per data source
│   ├── models/                 # DB row types
│   ├── config.rs               # Loads airports.json (legacy)
│   └── db/                     # Connection pool, pipeline run tracking
├── python/                     # Python helpers
│   ├── sentiment_pipeline.py   # RoBERTa + NLI sentiment analysis
│   ├── skytrax_scraper.py      # Skytrax review scraper
│   ├── priority_pass_scraper.py # Priority Pass lounge scraper
│   └── requirements.txt
├── data/                       # Static data files
│   ├── carbon_accreditation.json
│   └── aena/ert_dly/           # Eurocontrol delay files (manual download)
├── scripts/                    # Shell helpers
│   ├── backup-db.sh
│   ├── verify-db-restore.sh
│   ├── setup-google-scraper.sh
│   └── start-google-scraper.sh
├── google-reviews-scraper-pro/ # Git submodule
└── web/                        # TanStack Start frontend
    ├── app/
    │   ├── api/                # Generated types + API client
    │   ├── routes/             # Pages (file-based routing)
    │   │   ├── index.tsx       # Homepage
    │   │   ├── airport.$iata.tsx # Airport detail
    │   │   └── admin/          # Admin UI
    │   ├── components/         # UI components
    │   ├── hooks/              # Custom hooks
    │   ├── utils/              # Shared utilities
    │   └── server/             # Server functions (proxy to Rust API)
    └── package.json
```
