# Airport Intelligence Platform

## Tech Stack

- **Frontend**: React 19 + TanStack Start, Tailwind CSS v4, Recharts, Radix UI, cmdk
- **Backend**: Rust CLI (`airport-fetch`) with Tokio, SQLx, Clap
- **Database**: PostgreSQL + PostGIS, Drizzle ORM (TypeScript side)
- **ML Pipeline**: Python 3.10+ with RoBERTa (GoEmotions) + NLI cross-encoder
- **Package managers**: pnpm (web), cargo (Rust), pip with venv (Python)

## Project Structure

```
airports-scoring/
├── .env                    # DATABASE_URL + API tokens (repo root, NOT web/)
├── airports.json           # Source of truth for airport config
├── src/                    # Rust data pipeline
│   ├── main.rs             # CLI entry point
│   ├── pipeline.rs         # Orchestrates all fetchers
│   ├── scoring/            # Composite score computation
│   ├── fetchers/           # One module per data source
│   └── models/             # DB row types
├── web/                    # TanStack Start frontend
│   └── app/
│       ├── routes/         # File-based routing
│       ├── components/     # UI components (single/, home/)
│       ├── hooks/          # Custom hooks (use-sentiment, use-single-airport)
│       ├── utils/          # Shared utilities (types, scoring, format, snark)
│       ├── server/         # Server functions (search)
│       └── db/             # Drizzle schema + relations
├── python/                 # Sentiment pipeline + scrapers
└── scripts/                # Shell helpers (Google scraper setup)
```

## Commands

### Web (run from `web/`)

```bash
pnpm dev              # Dev server on http://localhost:3000
pnpm build            # Production build
pnpm db:push          # Push Drizzle schema to Postgres
pnpm db:seed          # Seed countries, regions, organisations
pnpm db:seed-airports # Seed 29K global airports for route lookups
pnpm db:studio        # Drizzle Studio on http://localhost:5555
```

### Rust pipeline (run from repo root)

```bash
cargo run -- BER --source wikipedia          # Single airport, single source
cargo run -- --all --source reviews          # All airports, reviews (Skytrax + Google)
cargo run -- --all --full-refresh --score    # Full refresh + recompute scores
cargo test                                   # Run Rust tests
```

### Valid `--source` values

`ourairports`, `wikipedia`, `eurocontrol`, `eurostat`, `routes`, `metar`, `reviews` (Skytrax + Google), `skytrax`, `google_reviews`, `sentiment`, `opensky`, `caa`, `aena`

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

### Rust

- Every fetcher follows the same signature: `pub async fn fetch(pool, airport, full_refresh) -> Result<FetchResult>`
- `FetchResult { records_processed: i32, last_record_date: Option<NaiveDate> }`
- Graceful degradation: if a source is unavailable, log warning and return 0 records

### Database

- `airports.json` is the source of truth for adding airports — add there first
- PostGIS `location` column is NOT in Drizzle schema — managed by Rust CLI
- When running `drizzle-kit push`, abort if prompted to drop `location` column
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
DATABASE_URL=postgres://...          # Required
HF_TOKEN=hf_...                     # HuggingFace (for sentiment ML models)
GOOGLE_SCRAPER_URL=http://localhost:8000  # Optional, skips gracefully
OPENSKY_ID=...                       # Optional
OPENSKY_SECRET=...                   # Optional
```

## Gotchas

- Eurocontrol `apt_dly` files must be manually downloaded from https://ansperformance.eu/csv/ into `data/aena/ert_dly/`
- Google scraper runs as a separate local service (sibling directory `../google-reviews-scraper-pro`). Setup: `bash scripts/setup-google-scraper.sh`
- Google scraper needs Python <=3.13 (pydantic-core wheel compatibility)
- Google scraper config: `use_mongodb: false` (we read from API, not MongoDB)
- `scroll_idle_limit: 30` in scraper config for larger airports
- The `reviews` source runs Skytrax then Google in sequence. `skytrax` and `google_reviews` work as standalone aliases
- Pipeline with `--score` runs ALL sources before scoring — use `--source sentiment --score` for faster scoring-only runs
