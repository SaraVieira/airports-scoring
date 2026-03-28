# Airport Intelligence Platform

## Tech Stack

- **Frontend**: React 19 + TanStack Start, Tailwind CSS v4, shadcn/ui (base-nova), Recharts, cmdk
- **Backend**: Rust API server + CLI pipeline (`airport-fetch`) with Tokio, SQLx, Axum
- **Database**: PostgreSQL + PostGIS
- **ML Pipeline**: Python 3.10+ with RoBERTa (GoEmotions) + NLI cross-encoder
- **Package managers**: pnpm (web), cargo (Rust), pip with venv (Python)

## Project Structure

```
airports-scoring/
├── .env                    # DATABASE_URL + API tokens (repo root, NOT web/)
├── airports.json           # Source of truth for airport config
├── start.sh                # Start all dev services (Docker + Rust + Frontend)
├── docker-compose.dev.yml  # Postgres + Google scraper for local dev
├── src/                    # Rust data pipeline + API server
│   ├── main.rs             # CLI entry point (serve / fetch commands)
│   ├── server/             # Axum API server (routes, auth, jobs, SSE logs)
│   ├── pipeline.rs         # Orchestrates all fetchers
│   ├── scoring/            # Composite score computation
│   ├── fetchers/           # One module per data source
│   └── models/             # DB row types
├── migrations/             # SQL migrations (run automatically on server start)
├── scripts/
│   ├── airport_schema.sql  # Full DB schema (run once for fresh DB)
│   └── seed-all-airports.sh # Seeds 29K global airports into all_airports table
├── web/                    # TanStack Start frontend
│   └── app/
│       ├── routes/         # File-based routing (includes admin/)
│       ├── components/     # UI components (single/, home/, admin/, ui/)
│       ├── hooks/          # Custom hooks (use-sentiment, use-single-airport, use-admin-auth)
│       ├── utils/          # Shared utilities (types, scoring, format, snark, constants)
│       └── server/         # Server functions (search, admin API proxy)
├── python/                 # Sentiment pipeline + scrapers
└── scripts/                # Shell helpers
```

## Dev Setup (Fresh)

```bash
# Just run start.sh — it handles everything:
bash start.sh
# On first run it auto-detects an empty DB and seeds:
#   1. scripts/airport_schema.sql      — all tables
#   2. scripts/seed-all-airports.sh    — 29K global airports
#   3. scripts/seed-reference-data.sh  — countries, organisations, views, constraints
```

To re-seed manually (e.g. after wiping the DB):

```bash
DB=postgres://airports:airports@localhost:5433/airports
psql "$DB" -q < scripts/airport_schema.sql
DATABASE_URL="$DB" bash scripts/seed-all-airports.sh
DATABASE_URL="$DB" bash scripts/seed-reference-data.sh
```

## Commands

### Development

```bash
bash start.sh             # Start all services (Docker + Rust API + Frontend)
# Services:
#   Postgres:       localhost:5433
#   Rust API:       localhost:3001
#   Frontend:       localhost:3000
#   Google Scraper: localhost:8000
```

### Rust pipeline (run from repo root)

```bash
cargo run -- serve                           # Start API server on :3001
cargo run -- BER --source wikipedia          # Single airport, single source
cargo run -- --all --source reviews          # All airports, reviews (Skytrax + Google)
cargo run -- --all --full-refresh --score    # Full refresh + recompute scores
cargo test                                   # Run Rust tests
```

### Valid `--source` values

`ourairports`, `wikipedia`, `eurocontrol`, `eurostat`, `routes`, `metar`, `reviews` (Skytrax + Google), `skytrax`, `google_reviews`, `sentiment`, `opensky`, `caa`, `aena`, `carbon_accreditation`, `priority_pass`

### Web (run from `web/`)

```bash
pnpm dev              # Dev server on http://localhost:3000
pnpm build            # Production build
pnpm db:seed          # Seed countries, regions, organisations
```

### TypeScript checking

```bash
cd web && npx tsc --noEmit
```

## Conventions

### Frontend

- Types centralized in `web/app/utils/types.ts` — import from there, don't inline
- `scoreColor()` and `scoreBg()` live in `web/app/utils/scoring.ts` — don't duplicate
- Score/sentiment explanations in `web/app/utils/constants.ts`
- Chart tooltip styles shared via `web/app/utils/styles.ts`
- Use `<Link>` from TanStack Router, not `<a>` tags for internal navigation
- Use `value != null && value > 0 &&` for numeric conditional rendering, NOT `value &&`
- Memoize expensive computations in hooks with `useMemo`
- Reviews are **anonymous** — never show reviewer names or photos
- Use shadcn/ui components (`~/components/ui/*`) for admin pages — Button, Card, Table, Dialog, Badge, Input, etc.
- Admin auth uses shared `useAdminAuth()` hook from `~/hooks/use-admin-auth`
- All admin API calls go through server functions in `~/server/admin.ts` (not direct browser fetch)

### Rust

- Every fetcher follows the same signature: `pub async fn fetch(pool, airport, full_refresh) -> Result<FetchResult>`
- `FetchResult { records_processed: i32, last_record_date: Option<NaiveDate> }`
- Graceful degradation: if a source is unavailable, log warning and return 0 records
- API server runs migrations from `migrations/` on startup via `sqlx::migrate!`
- SSE log streaming at `/api/admin/logs/stream` uses `tokio::sync::broadcast` + custom tracing Layer

### Database

- Schema lives in `scripts/airport_schema.sql` — run once for fresh databases
- Migrations in `migrations/` run automatically on Rust API server startup
- `all_airports` table seeded by `scripts/seed-all-airports.sh` (29K global airports for batch import)
- PostGIS `location` column managed by Rust CLI, not in any ORM
- `.env` must be in repo root (Vite config uses `envDir: '..'`)

### Sentiment Pipeline

- Empty-text reviews still contribute ratings (derived sentiment from rating alone)
- Rating 7-10: positive, 4-6: neutral, 1-3: negative
- Google ratings stored as `rating * 2` (1-5 → 1-10 scale) in `reviews_raw`
- Sentiment snapshots tagged by source (`skytrax` / `google`)
- Commentary templates in `python/commentary.toml`

### Scoring

- `avg_rating` in sentiment_snapshots is on 1-10 scale — do NOT multiply by 2
- Sentiment score formula: `((rating - 1) / 9) * 100`
- Sub-score color thresholds (1-5 scale): >=3.5 green, 2.5-3.5 yellow, <2.5 red

## Environment Variables

```
DATABASE_URL=postgres://...                  # Required
ADMIN_PASSWORD=...                           # Admin panel password
API_KEY=...                                  # API key for public endpoints
HF_TOKEN=hf_...                              # HuggingFace (for sentiment ML models)
GOOGLE_SCRAPER_URL=http://localhost:8000      # Optional, skips gracefully
VITE_API_URL=http://localhost:3001            # Rust API URL (server-side)
VITE_PUBLIC_API_URL=http://localhost:3001     # Rust API URL (browser-side, for SSE logs)
OPENSKY_ID=...                               # Optional
OPENSKY_SECRET=...                           # Optional
```

## Deployment (Coolify)

Production runs via `docker-compose.yml` on Coolify with 4 services:

| Service | Image | Port | Public |
|---------|-------|------|--------|
| `postgres` | `postgis/postgis:16-3.4` | 5432 (internal) | No |
| `api` | Built from `./Dockerfile` (Rust + Python) | 8080 (internal) | Yes — proxied as `airports-api.deploy.iamsaravieira.com` |
| `web` | Built from `./web/` (TanStack Start) | 3000 (internal) | Yes — proxied as main domain |
| `google-scraper` | Built from `google-scraper.Dockerfile` | 8000 (internal) | No |

### Coolify Environment Variables

Set these in the Coolify dashboard (stack-level env vars):

```
API_KEY=<your-api-key>            # Protects all public API endpoints
ADMIN_PASSWORD=<your-password>    # Admin panel login
HF_TOKEN=hf_...                   # HuggingFace (sentiment ML pipeline)
OPENSKY_ID=...                    # Optional
OPENSKY_SECRET=...                # Optional
```

### Web Service Build Args

The web service needs build-time args (set in Coolify build settings):

```
VITE_API_URL=http://api:8080              # Internal API URL (server functions use this)
VITE_API_KEY=<same-api-key>               # Embedded at build time for server functions
VITE_PUBLIC_API_URL=https://airports-api.deploy.iamsaravieira.com  # Browser-side API URL (for SSE logs)
```

### How It Works

- `api` runs the Rust binary (`airport-fetch serve --port 8080`) — serves the REST API, runs migrations on startup, handles pipeline jobs
- `web` runs the TanStack Start frontend — proxies API calls through server functions (so the browser never talks to the API directly, except SSE logs)
- SSE log streaming is the one exception: `EventSource` connects directly from browser → public API URL (can't go through server functions). Requires `VITE_PUBLIC_API_URL` to be the publicly accessible API domain
- Google scraper is internal-only, called by the Rust API during review fetching

### Database Access

Postgres is internal-only. To run SQL in production:

```bash
# From Coolify server
docker exec -i <postgres-container> psql -U airports -d airports
```

### First Deploy / Fresh Database

After first deploy, seed the reference data:

```bash
# From inside the api container (or via Coolify terminal)
# The all_airports table is created by the seed script, not by migrations
# You need curl and jq available — or run from a machine with psql access

# Option 1: From a machine with psql access to prod DB
DATABASE_URL=<prod-db-url> bash scripts/seed-all-airports.sh

# Option 2: Run the old Drizzle seeds for countries/regions (if not already seeded)
cd web && pnpm db:seed
```

## Gotchas

- Eurocontrol `apt_dly` files must be manually downloaded from https://ansperformance.eu/csv/ into `data/aena/ert_dly/`
- Google scraper runs as a separate local service (sibling directory `../google-reviews-scraper-pro`). Setup: `bash scripts/setup-google-scraper.sh`
- Google scraper needs Python <=3.13 (pydantic-core wheel compatibility)
- Google scraper config: `use_mongodb: false` (we read from API, not MongoDB)
- `scroll_idle_limit: 30` in scraper config for larger airports
- The `reviews` source runs Skytrax then Google in sequence. `skytrax` and `google_reviews` work as standalone aliases
- Pipeline with `--score` runs ALL sources before scoring — use `--source sentiment --score` for faster scoring-only runs
- SSE log streaming connects directly from browser to Rust API (EventSource can't use server functions) — requires `VITE_PUBLIC_API_URL` in production
