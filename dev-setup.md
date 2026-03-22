# Dev Setup

## Prerequisites

- **Rust** (stable toolchain)
- **Node.js** 20+ with **pnpm**
- **Python** 3.10+ (for sentiment pipeline)
- **PostgreSQL** with PostGIS extension
- A **HuggingFace** account (free) for the sentiment model

## Environment

Create a `.env` file in the repo root:

```
DATABASE_URL=postgres://user:pass@host:port/dbname
HF_TOKEN=hf_your_huggingface_token
OPENSKY_ID=your_opensky_client_id
OPENSKY_SECRET=your_opensky_client_secret
```

- **HF_TOKEN**: Get from https://huggingface.co/settings/tokens (free account, needed for sentiment pipeline)
- **OPENSKY_ID / OPENSKY_SECRET**: Register at https://opensky-network.org/ and create OAuth2 credentials (needed for `--source opensky`)

## Database Setup

```bash
cd web
pnpm install
pnpm db:push          # Push Drizzle schema to Postgres
pnpm db:seed          # Seed countries, regions, organisations, and initial airport data
pnpm db:seed-airports # Seed 29K global airports for route destination lookups
```

## Python Venv (for sentiment)

```bash
python3 -m venv .venv
.venv/bin/pip install -r python/requirements.txt
```

## Running the Web App

```bash
cd web
pnpm dev              # Starts Vite dev server on http://localhost:3000
```

## Data Pipeline

The Rust CLI (`airport-fetch`) orchestrates all data fetching. It reads the seed airport list from `airports.json` in the repo root.

### Basic Usage

```bash
# Fetch a single source for one airport
cargo run -- BER --source wikipedia

# Fetch all sources for one airport
cargo run -- BER

# Fetch all airports for one source
cargo run -- --all --source eurocontrol

# Fetch everything for all airports
cargo run -- --all

# Force re-fetch (ignore incremental state)
cargo run -- --all --source wikipedia --full-refresh

# Compute scores after fetching
cargo run -- --all --score
```

### Data Sources

| Source | Flag | What it does | Data pulled |
|--------|------|-------------|-------------|
| **ourairports** | `--source ourairports` | Downloads OurAirports CSVs | Runways, frequencies, navaids, basic airport info |
| **wikipedia** | `--source wikipedia` | Fetches Wikipedia articles via API | Passenger stats, opened year, operator, terminal count, renovation notes, Skytrax history, ACI awards |
| **eurocontrol** | `--source eurocontrol` | Downloads Eurocontrol CSVs + local apt_dly bz2 files | Monthly flight counts, avg delay minutes, ATFM delay % and cause breakdown (weather/carrier/ATC/airport) |
| **opdi** | `--source opdi` | Downloads OPDI parquet files via Python helper | Route network (destination ICAO, flights per month) |
| **eurostat** | `--source eurostat` | Downloads Eurostat passenger data | Historical passenger traffic by year |
| **metar** | `--source metar` | Downloads IEM ASOS weather observations | Daily weather stats (temp, wind, visibility, precipitation) |
| **skytrax** | `--source skytrax` | Scrapes Skytrax reviews via Python | Raw review text, ratings, sub-scores into `reviews_raw` |
| **sentiment** | `--source sentiment` | Runs RoBERTa + NLI on unprocessed reviews | Quarterly sentiment snapshots (avg rating, sub-scores, positive/negative/neutral %) |
| **opensky** | `--source opensky` | Fetches OpenSky Network flight data | Flight movements (currently limited by API) |
| **caa** | `--source caa` | Downloads UK CAA passenger statistics | UK airport passenger data |

### Eurocontrol Delay Files

The ATFM delay cause breakdown comes from `apt_dly` CSV files that are behind antibot protection. Download them manually from the Eurocontrol portal:

1. Go to https://ansperformance.eu/csv/
2. Download all `apt_dly_{year}.csv.bz2` files
3. Place them in `data/aena/ert_dly/`

The fetcher reads these local bz2 files automatically during `--source eurocontrol`.

### Pipeline Order

For a fresh setup, run sources in this order:

```bash
cargo run -- --all --source ourairports     # Base airport data + runways
cargo run -- --all --source wikipedia --full-refresh  # Pax stats, metadata
cargo run -- --all --source eurostat        # Historical pax
cargo run -- --all --source eurocontrol --full-refresh  # Ops stats + delays
cargo run -- --all --source opdi            # Route network
cargo run -- --all --source skytrax         # Scrape reviews
cargo run -- --all --source sentiment       # Analyze reviews (needs skytrax first)
cargo run -- --all --score                  # Compute composite scores
```

## Adding a New Airport

All airport configuration lives in `airports.json` in the repo root. To add a new airport:

1. **Edit `airports.json`** — add an entry:

```json
{
  "iata": "JFK",
  "country": "US",
  "name": "New York JFK",
  "skytrax_review_slug": "new-york-jfk-airport",
  "skytrax_rating_slug": "new-york-jfk-airport"
}
```

  - `iata` (required): 3-letter IATA code
  - `country` (required): 2-letter ISO country code
  - `name` (required): Display name
  - `skytrax_review_slug` (optional): Slug from `airlinequality.com/airport-reviews/{slug}`
  - `skytrax_rating_slug` (optional): Slug from `skytraxratings.com/airports/{slug}`

  To find the Skytrax slugs, search on [airlinequality.com](https://www.airlinequality.com/) and [skytraxratings.com](https://skytraxratings.com/).

2. **Run the pipeline** for the new airport:

```bash
cargo run -- JFK --source ourairports
cargo run -- JFK --source wikipedia --full-refresh
cargo run -- JFK --source eurocontrol --full-refresh
cargo run -- JFK --source opdi
cargo run -- JFK --source skytrax
cargo run -- JFK --source sentiment
```

3. **Re-run route linking** (so existing airports' routes resolve the new destination):

```bash
cd web && pnpm db:seed-airports
```

That's it. The web app will pick up the new airport at `/airport/jfk`.

## Web Scripts Reference

Run from the `web/` directory:

| Script | Command | What it does |
|--------|---------|-------------|
| `dev` | `pnpm dev` | Start Vite dev server |
| `build` | `pnpm build` | Production build |
| `start` | `pnpm start` | Preview production build |
| `db:push` | `pnpm db:push` | Push Drizzle schema to DB |
| `db:seed` | `pnpm db:seed` | Seed initial data (countries, orgs, airports) |
| `db:seed-airports` | `pnpm db:seed-airports` | Seed 29K global airports for route lookups |
| `db:studio` | `pnpm db:studio` | Open Drizzle Studio (DB browser) |

## Project Structure

```
airports-scoring/
├── airports.json          # Seed airport list (add new airports here)
├── .env                   # DATABASE_URL + HF_TOKEN
├── src/                   # Rust data pipeline
│   ├── main.rs            # CLI entry point
│   ├── config.rs          # Loads airports.json
│   ├── pipeline.rs        # Orchestrates fetch → DB
│   ├── scoring.rs         # Composite score computation
│   ├── fetchers/          # One module per data source
│   └── models/            # DB row types
├── python/                # Python helpers
│   ├── skytrax_scraper.py # Skytrax review scraper
│   ├── sentiment_pipeline.py  # RoBERTa sentiment analysis
│   └── requirements.txt
├── data/aena/ert_dly/     # Local apt_dly bz2 files (manual download)
└── web/                   # TanStack Start web app
    ├── app/
    │   ├── db/             # Drizzle schema + relations
    │   ├── routes/         # Page components
    │   └── styles.css      # Tailwind + fonts
    ├── vite.config.ts
    └── package.json
```
