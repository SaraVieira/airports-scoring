# Airport Intelligence Platform — Claude Code Handoff

## What we're building

A web platform that scores and visualizes European airports across multiple dimensions:
infrastructure, operational performance, passenger sentiment, and operator quality. The core
insight is that a single score is meaningless — BER and OPO are both "4-star Skytrax airports"
and that tells you nothing. We want time-series data showing trajectories, not snapshots.

---

## Seed airports (15)

| IATA | ICAO | Airport | Country |
|------|------|---------|---------|
| LHR | EGLL | London Heathrow | UK |
| LGW | EGKK | London Gatwick | UK |
| LTN | EGGW | London Luton | UK |
| OPO | LPPR | Porto | Portugal |
| MAD | LEMD | Madrid Barajas | Spain |
| BCN | LEBL | Barcelona El Prat | Spain |
| BER | EDDB | Berlin Brandenburg | Germany |
| MUC | EDDM | Munich | Germany |
| CDG | LFPG | Paris Charles de Gaulle | France |
| NCE | LFMN | Nice | France |
| AMS | EHAM | Amsterdam Schiphol | Netherlands |
| CPH | EKCH | Copenhagen | Denmark |
| FCO | LIRF | Rome Fiumicino | Italy |
| WAW | EPWA | Warsaw Chopin | Poland |
| BUD | LHBP | Budapest | Hungary |

---

## Tech stack

| Layer | Technology |
|---|---|
| Database | PostgreSQL + PostGIS + pg_trgm |
| Ingestion CLI | Rust (reqwest, csv, sqlx, serde, parquet, arrow, calamine) |
| Playwright scraping | Python (playwright, beautifulsoup4) |
| ML sentiment pipeline | Python (transformers, torch, polars) |
| Web framework | TanStack Start (React, server functions) |
| Route globe | deck.gl GlobeView + ArcLayer |
| Charts | Recharts |
| Styling | Tailwind |

**Rust crates needed:**
- `sqlx` with `postgres` and `runtime-tokio` features — compile-time checked queries
- `reqwest` with `json` feature
- `serde` + `serde_json`
- `csv` — for OurAirports, Eurocontrol, UK CAA, METAR, OpenFlights
- `parquet` + `arrow` — for OPDI Parquet files
- `calamine` — for AENA XLS files
- `tokio` — async runtime
- `clap` — CLI argument parsing
- `chrono` — date handling
- `anyhow` — error handling

**Python packages (requirements.txt):**
```
playwright
beautifulsoup4
transformers
torch
polars
sentencepiece
```

---

## CLI design

The Rust binary is the orchestrator. It calls Python as subprocesses for Skytrax
scraping and ML inference. Python outputs newline-delimited JSON (NDJSON) to stdout,
Rust reads and upserts into Postgres. Python errors go to stderr and are captured
into `pipeline_runs.error_message`.

```bash
airport-fetch LTN                      # fetch/update all sources
airport-fetch LTN --source skytrax    # just one source
airport-fetch LTN --source wikipedia  # just wikipedia
airport-fetch LTN --full-refresh      # ignore incremental, fetch everything
airport-fetch LTN BER OPO             # batch multiple airports
airport-fetch --all                    # all airports in seed set
airport-fetch --all --source metar    # one source, all airports
```

**Valid --source values:**
`ourairports`, `eurocontrol`, `caa`, `metar`, `opdi`, `openflights`,
`opensky`, `eurostat`, `aena`, `wikipedia`, `skytrax`, `sentiment`

**Python subprocess interface:**
```bash
python skytrax_scraper.py --airport LTN --since 2024-01-01
# stdout (NDJSON — one JSON object per line):
# {"type":"review","date":"2024-03-15","rating":3,"queuing":2,...,"text":"..."}
# {"type":"star_rating","stars":3,"sub_scores":{...}}

python sentiment_pipeline.py --airport LTN
# stdout (NDJSON):
# {"type":"snapshot","year":2024,"quarter":1,"avg_rating":3.2,...}
```

Rust reads stdout line by line, deserializes each object by `type` field,
upserts into the appropriate table.

---

## Data sources — URLs and access details

### OurAirports
```
https://davidmegginson.github.io/ourairports-data/airports.csv
https://davidmegginson.github.io/ourairports-data/runways.csv
https://davidmegginson.github.io/ourairports-data/airport-frequencies.csv
https://davidmegginson.github.io/ourairports-data/navaids.csv
```
Public domain. Download full CSV, filter by IATA/ICAO for seed airports.
Updated daily by GitHub Actions from the live OurAirports site.

### Eurocontrol PRU
```
https://ansperformance.eu/csv/    ← index page listing all available CSVs
```
Key datasets (filter rows by ICAO code column after download):
- Airport traffic — daily IFR movements, Jan 2016–present
- Airport arrival ATFM delays by cause — Jan 2019–present
- ASMA additional time — approach congestion proxy, Jan 2018–present
- Taxi-out additional time — ground ops proxy, Jan 2018–present
- ATC pre-departure delays — Jan 2016–present

All free, no registration, direct CSV. Updated monthly with ~1 month lag.

### OPDI
```
https://www.opdi.aero/flight-list-data
# URL pattern for monthly Parquet:
# https://www.eurocontrol.int/performance/data/download/OPDI/v002/flight_list/flight_list_{YYYYMM}.parquet
```
Jan 2022–present. Parse `origin_icao` and `destination_icao` columns to
reconstruct routes. Aggregate unique (origin, destination, airline) pairs
per month per airport. Use `parquet` + `arrow` crates in Rust.

### OpenSky REST API
```
https://opensky-network.org/api/flights/arrival?airport={ICAO}&begin={unix_ts}&end={unix_ts}
https://opensky-network.org/api/flights/departure?airport={ICAO}&begin={unix_ts}&end={unix_ts}
```
Requires free registered account. OAuth2 (basic auth deprecated March 2026).
Max 2-day window per request — iterate in 2-day chunks to build history.
4,000 API credits/day on free tier.

### IEM METAR
```
https://mesonet.agron.iastate.edu/cgi-bin/request/asos.py?station={ICAO}&data=all&sts={YYYY-MM-DD}&ets={YYYY-MM-DD}&format=comma&latlon=no&direct=no
```
Returns CSV with one row per observation (~every 30-60 min). Aggregate to daily.
Fields used: valid (timestamp), sknt (wind knots), vsby (visibility miles),
p01i (precip inches), skyc1/skyl1 (cloud cover/height), wxcodes.
Free, no auth. Data back to ~2000 for all 15 airports.

**BER METAR NOTE — CRITICAL**: Berlin Brandenburg (EDDB) opened October 31, 2020.
For Berlin weather before that date, fetch EDDT (Tegel, closed Nov 8 2020).
The metar fetcher must automatically pull EDDT for any BER date range before
2020-11-01 and store those rows in metar_daily under BER's airport_id.
Verify data continuity around Oct–Nov 2020 in testing.

### UK CAA (LHR, LGW, LTN only)
```
https://www.caa.co.uk/data-and-analysis/uk-aviation-market/flight-punctuality/
```
Navigate to "UK Flight Punctuality Statistics" for monthly CSV downloads.
Per-airline, per-route breakdown with delay minutes and delay bands.
Updated monthly, ~2-3 month lag. Only needed for UK airports — all others
use Eurocontrol PRU.

### Eurostat REST API
```
https://ec.europa.eu/eurostat/api/dissemination/statistics/1.0/data/avia_paoa?format=JSON&airp_icao={ICAO}&time={YYYY}
```
Dataset `avia_paoa` — passenger traffic by airport pair.
EU airports only. UK airports (LHR/LGW/LTN) excluded post-Brexit — use
Wikipedia historical tables or CAA for those.
Data back to 2002, 3-6 month lag.

### AENA DATAESTUR (MAD + BCN only)
```
https://www.aena.es/es/estadisticas/estadisticas-de-trafico-aereo.html
```
Monthly Excel files with passengers, operations, cargo per airport from 2004.
Use `calamine` Rust crate to parse XLS. Filter for LEMD (Madrid) and LEBL (Barcelona).

### OpenFlights (pre-2022 route fallback — visualization only, not scoring)
```
https://raw.githubusercontent.com/jpatokal/openflights/master/data/routes.dat
```
67,663 routes, frozen June 2014. CSV: airline, source_airport, dest_airport,
codeshare, stops, equipment. Import once with source = 'openflights' and
valid_to = '2014-06-01'. Not used in scoring — only for route globe visualization
where OPDI data is absent.

### Wikipedia REST API
```
# Summary + infobox
GET https://en.wikipedia.org/api/rest_v1/page/summary/{article_title}

# Full wikitext for table + infobox parsing
GET https://en.wikipedia.org/w/api.php?action=parse&page={article_title}&prop=wikitext&format=json

# Revision ID for change detection (fetch first, skip re-parse if unchanged)
GET https://en.wikipedia.org/w/api.php?action=query&titles={article_title}&prop=revisions&rvprop=ids&format=json
```
Extract article title from `wikipedia_url` column in airports table.
e.g. `https://en.wikipedia.org/wiki/Berlin_Brandenburg_Airport` → `Berlin_Brandenburg_Airport`

No auth. Add `User-Agent: AirportIntelligencePlatform/1.0` header.
Rate limit to 1 req/second per Wikipedia API etiquette.

What to extract per airport:
- Infobox: opened year, operator, owner, terminal count, runway count
- Passenger statistics table: annual pax going back to 1990s or earlier → pax_yearly
- Renovation/expansion history with dates → wikipedia_snapshots.renovation_notes
- Ownership change history → wikipedia_snapshots.ownership_notes
- Awards: Skytrax star history, ACI ASQ award years → wikipedia_snapshots JSONB fields
- article_revision_id for change detection on re-runs

Wikitext passenger tables vary in structure between articles — some have years
as columns, some as rows. Parser must handle both orientations and fail softly
(log warning, skip, continue) rather than crash.

### Skytrax review pages (Playwright)
```
https://www.airlinequality.com/airport-reviews/{slug}/page/{n}/?sortby=post_date:Desc&pagesize=100
```
Bot protection active (BotStopper/Cloudflare). Use Playwright headless browser.
Rate limit: 2-3 second delay between page loads. Realistic user-agent string.
Stop paginating when review dates go older than `pipeline_runs.last_record_date`.

Sub-scores in server-rendered HTML as star ratings in `.review-ratings` table.
Categories: Queuing Times, Terminal Cleanliness, Terminal Seating,
Terminal Signs & Directions, Food Beverages, Airport Shopping,
Wifi & Connectivity, Airport Staff. All nullable.

### Skytrax star ratings (Playwright)
```
https://skytraxratings.com/airports/{slug}-quality-rating
```
Single page per airport. Extract overall star rating (1-5) and audit text.
Lightweight, no pagination needed.

### Historical Skytrax bootstrap
```
https://github.com/quankiquanki/skytrax-reviews-dataset
```
Pre-scraped CSV to ~2015. Import first into reviews_raw with
source = 'skytrax_historical' before running live Playwright scraper.

---

## Skytrax slugs — seed data for airport_slugs table

Verify each URL manually before first scrape run. If 404, check
`https://www.airlinequality.com/review-pages/a-z-airport-reviews/` for correct slug.

| IATA | source | slug |
|------|--------|------|
| LHR | skytrax | london-heathrow-airport |
| LHR | skytrax_ratings | london-heathrow-airport |
| LGW | skytrax | london-gatwick-airport |
| LGW | skytrax_ratings | london-gatwick-airport |
| LTN | skytrax | london-luton-airport |
| LTN | skytrax_ratings | london-luton-airport |
| OPO | skytrax | porto-airport |
| OPO | skytrax_ratings | porto-airport |
| MAD | skytrax | madrid-barajas-airport |
| MAD | skytrax_ratings | madrid-barajas-airport |
| BCN | skytrax | barcelona-el-prat-airport |
| BCN | skytrax_ratings | barcelona-el-prat-airport |
| BER | skytrax | berlin-brandenburg-airport |
| BER | skytrax_ratings | berlin-brandenburg-airport |
| MUC | skytrax | munich-airport |
| MUC | skytrax_ratings | munich-airport |
| CDG | skytrax | paris-charles-de-gaulle-airport |
| CDG | skytrax_ratings | paris-charles-de-gaulle-airport |
| NCE | skytrax | nice-cote-dazur-airport |
| NCE | skytrax_ratings | nice-cote-dazur-airport |
| AMS | skytrax | amsterdam-schiphol-airport |
| AMS | skytrax_ratings | amsterdam-schiphol-airport |
| CPH | skytrax | copenhagen-airport |
| CPH | skytrax_ratings | copenhagen-airport |
| FCO | skytrax | rome-fiumicino-airport |
| FCO | skytrax_ratings | rome-fiumicino-airport |
| WAW | skytrax | warsaw-chopin-airport |
| WAW | skytrax_ratings | warsaw-chopin-airport |
| BUD | skytrax | budapest-airport |
| BUD | skytrax_ratings | budapest-airport |

---

## Database schema

See `schema.sql` for full DDL. All tables listed below.

### Extensions
```sql
CREATE EXTENSION IF NOT EXISTS postgis;
CREATE EXTENSION IF NOT EXISTS pg_trgm;
```

### All tables

**`countries`** — ISO 3166-1 alpha-2 codes, name, continent

**`regions`** — ISO region codes (e.g. GB-ENG), linked to countries

**`organisations`** — Airport operators and owners. Pre-seeded with 14 orgs:
AENA, VINCI, Ferrovial, Heathrow Airport Holdings, Gatwick Airport Ltd,
Luton Rising, Flughafen München GmbH, FBB, Groupe ADP, Schiphol Group,
Copenhagen Airports A/S, Aeroporti di Roma, PPL Poland, Budapest Airport Zrt.

**`airports`** — IATA/ICAO/OurAirports IDs, name, city, country, PostGIS
GEOGRAPHY(POINT) location, elevation, timezone, type, terminal count, gates,
opened_year, last_major_reno, operator_id + owner_id FKs, annual capacity,
pax figures, wikipedia_url, website_url, skytrax_url, `in_seed_set` boolean.

**`runways`** — Both ends (le/he), length_ft, width_ft, surface, lighted,
closed, headings, displaced thresholds. ON DELETE CASCADE.

**`frequencies`** — Radio frequencies per airport. Type, description, MHz.

**`pax_yearly`** — Annual passenger traffic. total, domestic, international,
aircraft movements, cargo, source.
UNIQUE (airport_id, year).
Source values: 'eurostat', 'wikipedia', 'aena', 'caa', 'airport_report'.

**`operational_stats`** — Delays/cancellations, monthly or annual.
delay_pct, avg_delay_minutes, cancellation_pct, cause breakdown
(weather/carrier/atc/security/airport pct), mishandled_bags_per_1k.
UNIQUE (airport_id, period_year, period_month).

**`metar_daily`** — Daily aggregated weather per airport.
avg_visibility_sm, avg_wind_kt, max_wind_kt, precip_inches,
low_cloud_ceiling_ft (NULL = clear), had_fog BOOL, had_thunder BOOL, had_snow BOOL.
UNIQUE (airport_id, date).

**`routes`** — Reconstructed from OPDI + OpenSky + OpenFlights.
origin_airport_id, destination_airport_id, airline_iata,
flights_per_month, first_observed, last_observed,
source ('opdi', 'opensky', 'openflights').
UNIQUE (origin_airport_id, destination_airport_id, airline_iata, source).

**`reviews_raw`** — Staging table for Skytrax reviews.
date, overall_rating (1-10), sub-scores (queuing/cleanliness/seating/signs/
food/shopping/wifi/staff — all nullable SMALLINT 1-5), verified BOOL,
review_text TEXT, source_url TEXT, source ('skytrax', 'skytrax_historical').
UNIQUE (source_url). Purged after ML aggregation.

**`sentiment_snapshots`** — Aggregated per airport per source per quarter.
avg_rating (0-5 normalised), review_count, positive/negative/neutral pct,
sub-scores (queuing/cleanliness/staff/food_bev/shopping/wifi/wayfinding/transport),
skytrax_stars at snapshot time.
UNIQUE (airport_id, source, snapshot_year, snapshot_quarter).

**`airport_scores`** — Composite scores, versioned. 6 dimension scores + total,
all 0-100. Weights stored alongside. is_latest boolean. score_version field.

**`operator_scores`** — Portfolio averages per organisation.

**`wikipedia_snapshots`** — Per airport per fetch.
opened_year, operator_raw, owner_raw, terminal_count, terminal_names TEXT[],
renovation_notes TEXT, ownership_notes TEXT, milestone_notes TEXT,
skytrax_history JSONB (e.g. {"2019":3,"2023":4}),
aci_awards JSONB, wikipedia_url, article_revision_id BIGINT.

**`airport_slugs`** — (airport_id, source) PK.
Sources: 'skytrax', 'skytrax_ratings', 'eurocontrol'.
(Trustpilot removed from v1 scope.)

**`pipeline_runs`** — Every fetch per airport per source.
started_at, completed_at, status ('running'/'success'/'failed'),
records_processed INT, last_record_date DATE (drives incremental),
error_message TEXT.

---

## Scoring model — exact normalisation formulas

All dimensions 0-100. Higher = better always.

### Infrastructure score (weight: 15%)
```
runway_score     = LEAST(runway_count / 3.0, 1.0) * 100
length_score     = LEAST(longest_runway_ft / 13000.0, 1.0) * 100
age_score        = CASE
                     WHEN last_major_reno IS NOT NULL
                       THEN GREATEST(0, 100 - (current_year - last_major_reno) * 3)
                     ELSE GREATEST(0, 100 - (current_year - opened_year) * 1.5)
                   END
capacity_score   = LEAST((annual_pax_latest_m / annual_capacity_m) * 100, 100)

score_infrastructure = (runway_score * 0.35) + (length_score * 0.25)
                     + (age_score * 0.25) + (capacity_score * 0.15)
```

### Operational score (weight: 25%)
```
delay_score        = GREATEST(0, 100 - (delay_pct * 2.5))
avg_delay_score    = GREATEST(0, 100 - (avg_delay_minutes * 3))
cancellation_score = GREATEST(0, 100 - (cancellation_pct * 10))
taxi_score         = GREATEST(0, 100 - (taxi_out_additional_min * 10))

-- Airport-caused delays penalised more than weather/ATC
attribution_modifier = 1.0 - (airport_delay_pct * 0.003)

score_operational = ((delay_score * 0.35) + (avg_delay_score * 0.25)
                  + (cancellation_score * 0.20) + (taxi_score * 0.20))
                  * attribution_modifier
```

### Sentiment score (weight: 25%)
```
rating_score    = ((avg_rating - 1) / 9.0) * 100   -- normalise 1-10 to 0-100
sub_score_avg   = ((sum of non-null sub-scores - count) / (count * 4.0)) * 100
confidence      = LEAST(review_count / 500.0, 1.0)

score_sentiment = (rating_score * 0.6 + sub_score_avg * 0.4) * confidence
                + rating_score * (1 - confidence) * 0.6
```

### Sentiment velocity score (weight: 15%)
```
yoy_delta = avg_rating_last_4_quarters - avg_rating_prior_4_quarters  -- on 0-5 scale
score_sentiment_velocity = LEAST(100, GREATEST(0, 50 + (yoy_delta * 20)))
-- 50 = flat, 70 = +1.0 rating improvement YoY, 30 = -1.0 YoY decline
```

### Connectivity score (weight: 10%)
```
destination_score = LEAST(unique_destination_count / 100.0, 1.0) * 100
airline_score     = LEAST(airline_count / 30.0, 1.0) * 100
intl_ratio_score  = (international_pax / total_pax) * 100

score_connectivity = (destination_score * 0.4) + (airline_score * 0.3)
                   + (intl_ratio_score * 0.3)
```

### Operator score (weight: 10%)
```
-- Average of sentiment + operational scores across all airports
-- the same operator manages in the dataset.
-- If only 1 airport for this operator, weight 50% with neutral baseline of 50.
score_operator = AVG(score_sentiment + score_operational) / 2
                 OVER (PARTITION BY operator_id)
```

### Composite
```
score_total = (score_infrastructure     * 0.15)
            + (score_operational        * 0.25)
            + (score_sentiment          * 0.25)
            + (score_sentiment_velocity * 0.15)
            + (score_connectivity       * 0.10)
            + (score_operator           * 0.10)

-- Weights stored in airport_scores table alongside scores for reproducibility.
```

---

## ML sentiment pipeline

### Model 1: `cardiffnlp/roberta-base-go_emotions`
RoBERTa-base fine-tuned on 28 emotion categories. Input: review_text.
Key emotions:
- Positive: joy, approval, admiration, gratitude → feeds positive_pct
- Negative: anger, disgust, annoyance, disappointment, disapproval → feeds negative_pct
- Rest → neutral_pct

### Model 2: `cross-encoder/nli-MiniLM2-L6-H768`
Zero-shot topic classification. Labels:
```python
["queuing & security", "staff & service", "cleanliness",
 "food & beverage", "wayfinding & signage", "transport links",
 "facilities & seating", "parking & drop-off"]
```
Score all 8 labels per review, take top-2 as dominant topics.
Aggregate topic salience per airport per quarter → sentiment_snapshots sub-scores.

### Pipeline
```
reviews_raw (per airport, batched by quarter)
  → RoBERTa: emotion distribution per review
  → NLI: topic scores per review
  → aggregate to quarter level
  → upsert sentiment_snapshots
  → update pipeline_runs.last_record_date
```
Use `polars` DataFrames for aggregation. CPU-viable for 15 airports × ~500 reviews.

---

## Build sequence

### Phase 1 — Foundation
1. Postgres + PostGIS setup, run schema.sql
2. Rust binary scaffold: `clap` CLI, `sqlx` pool, error handling
3. `src/fetchers/ourairports.rs` — download all 4 CSVs, filter + upsert
4. Verify all 15 airports have complete runway + frequency data in DB
5. Seed `airport_slugs` table with the full slug table above
6. `src/fetchers/wikipedia.rs` — infobox + pax tables + awards for all 15

### Phase 2 — Operational data
7. `src/fetchers/eurocontrol.rs` — PRU CSVs → operational_stats
8. `src/fetchers/caa.rs` — UK CAA CSVs → operational_stats (LHR/LGW/LTN only)
9. `src/fetchers/metar.rs` — IEM METAR → metar_daily (EDDT fallback for BER pre-2020)
10. `src/fetchers/opdi.rs` — OPDI Parquet → routes
11. `src/fetchers/openflights.rs` — one-time routes.dat import as pre-2022 baseline
12. `src/fetchers/opensky.rs` — arrivals/departures → routes (2-day chunk iteration)
13. `src/fetchers/eurostat.rs` — pax_yearly for EU airports
14. `src/fetchers/aena.rs` — XLS via calamine → pax_yearly (MAD + BCN only)

### Phase 3 — Sentiment
15. `python/import_historical.py` — Skytrax GitHub dataset → reviews_raw
16. `python/skytrax_scraper.py` — Playwright → reviews_raw (2015–present)
17. `python/sentiment_pipeline.py` — RoBERTa + NLI → sentiment_snapshots
18. Verify `v_sentiment_trajectory` shows Luton's improvement arc

### Phase 4 — Scoring
19. Implement scoring SQL using exact formulas above
20. Compute first scores for all 15 airports
21. Sanity check:
    - MUC: highest overall
    - OPO: highest sentiment, high velocity
    - LTN: strong positive velocity despite middling absolute sentiment
    - BER: poor operational + sentiment, near-flat velocity
    - LHR: strong connectivity
    - BUD: unknown trajectory — interesting to see where it lands

### Phase 5 — Web app
22. TanStack Start scaffold in `web/`
23. Server functions for all DB queries + OpenSky live proxy
24. Search UI: full text + country + type + geo radius (PostGIS ST_DWithin)
25. Airport detail page: score cards + dimension breakdown
26. Delay + sentiment time series (Recharts)
27. Route globe: deck.gl GlobeView + ArcLayer
28. Operator comparison view from `v_operator_comparison`

### Phase 6 — Expand
29. Add airports beyond seed 15 (just run `airport-fetch {IATA}`)
30. Scheduled monthly refresh (systemd timer or tokio cron)
31. Pipeline health dashboard in web app (reads pipeline_runs)

---

## Known gaps

- **Skytrax 2015–present**: Playwright scraper fills after first run.
- **Routes pre-2022**: OpenFlights 2014 data used for viz only, not scoring.
- **AirHelp scores**: No API. Manual entry into a future table if needed.
- **OpenSky history depth**: 2-day chunk iteration builds history slowly over time.
  Research/Trino tier requires institutional affiliation — not available.
- **Trustpilot**: Removed from v1. Add later if needed.
- **BER METAR pre-2020**: Handled by EDDT fallback (see metar fetcher notes).
- **Commercial use**: Review OpenSky licensing if platform goes public.

---

## File structure

```
airport-intelligence/
├── Cargo.toml
├── schema.sql                    # Full Postgres schema — source of truth
├── src/
│   ├── main.rs                   # clap CLI + subcommand dispatch
│   ├── db.rs                     # sqlx pool setup
│   ├── pipeline.rs               # pipeline_runs management, incremental logic,
│   │                             # Python subprocess spawning + stdout reading
│   ├── models/
│   │   ├── airport.rs
│   │   ├── operational.rs
│   │   ├── sentiment.rs
│   │   ├── routes.rs
│   │   └── scores.rs
│   └── fetchers/
│       ├── ourairports.rs
│       ├── eurocontrol.rs
│       ├── caa.rs
│       ├── metar.rs              # includes EDDT fallback for BER pre-2020
│       ├── opdi.rs               # arrow + parquet crates
│       ├── openflights.rs        # one-time CSV import
│       ├── opensky.rs            # OAuth2, 2-day chunk iteration
│       ├── eurostat.rs
│       ├── aena.rs               # calamine crate for XLS
│       └── wikipedia.rs          # wikitext parsing, pax tables, awards
├── python/
│   ├── requirements.txt
│   ├── import_historical.py      # one-time Skytrax GitHub dataset import
│   ├── skytrax_scraper.py        # Playwright, NDJSON to stdout
│   └── sentiment_pipeline.py    # RoBERTa + NLI, NDJSON to stdout
└── web/                          # TanStack Start (Phase 5)
    ├── app/
    │   ├── routes/
    │   │   ├── index.tsx         # Search + globe
    │   │   ├── airport.$iata.tsx # Airport detail
    │   │   └── operators.tsx     # Operator comparison
    │   ├── components/
    │   │   ├── Globe.tsx         # deck.gl GlobeView + ArcLayer
    │   │   ├── ScoreCard.tsx
    │   │   ├── SentimentChart.tsx
    │   │   └── DelayChart.tsx
    │   └── server/
    │       └── db.ts             # Server functions → Postgres
    └── package.json
```

---

## Environment variables

```
DATABASE_URL=postgres://user:pass@localhost:5432/airports
OPENSKY_USERNAME=
OPENSKY_PASSWORD=
```

No other API keys needed. All other sources are unauthenticated.
Wikipedia requires only a descriptive User-Agent header, not a key.
