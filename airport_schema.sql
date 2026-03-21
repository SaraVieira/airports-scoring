-- ============================================================
-- AIRPORT INTELLIGENCE DATABASE
-- PostgreSQL Schema v1.0
-- Seed: 15 European airports
-- ============================================================

-- Extensions
CREATE EXTENSION IF NOT EXISTS postgis;
CREATE EXTENSION IF NOT EXISTS pg_trgm; -- trigram index for fuzzy full-text search

-- ============================================================
-- REFERENCE / LOOKUP TABLES
-- ============================================================

CREATE TABLE countries (
    iso_code        CHAR(2) PRIMARY KEY,       -- ISO 3166-1 alpha-2
    name            TEXT NOT NULL,
    continent       CHAR(2) NOT NULL           -- EU, NA, AS etc
);

CREATE TABLE regions (
    id              SERIAL PRIMARY KEY,
    iso_code        TEXT UNIQUE NOT NULL,      -- e.g. GB-ENG
    name            TEXT NOT NULL,
    country_code    CHAR(2) NOT NULL REFERENCES countries(iso_code)
);

-- ============================================================
-- OPERATORS & OWNERS
-- ============================================================

-- An operator is the entity that manages day-to-day airport operations.
-- An owner is the entity that holds the asset.
-- These are often different (e.g. Ferrovial operates Heathrow T2/T5,
-- but Heathrow Airport Holdings owns the airport).

CREATE TABLE organisations (
    id                  SERIAL PRIMARY KEY,
    name                TEXT NOT NULL,
    short_name          TEXT,                  -- e.g. "AENA", "VINCI", "Ferrovial"
    country_code        CHAR(2) REFERENCES countries(iso_code),
    org_type            TEXT NOT NULL          -- 'operator', 'owner', 'both'
        CHECK (org_type IN ('operator', 'owner', 'both')),
    ownership_model     TEXT                   -- 'public', 'private', 'mixed'
        CHECK (ownership_model IN ('public', 'private', 'mixed')),
    public_share_pct    NUMERIC(5,2),          -- % publicly owned, if known
    founded_year        SMALLINT,
    notes               TEXT,
    created_at          TIMESTAMPTZ DEFAULT NOW()
);

-- ============================================================
-- CORE AIRPORTS TABLE
-- ============================================================

CREATE TABLE airports (
    id                  SERIAL PRIMARY KEY,

    -- Identifiers (from OurAirports)
    iata_code           CHAR(3) UNIQUE,        -- e.g. LHR
    icao_code           CHAR(4) UNIQUE,        -- e.g. EGLL
    ourairports_id      INTEGER UNIQUE,        -- OurAirports internal id

    -- Basic info
    name                TEXT NOT NULL,
    short_name          TEXT,                  -- e.g. "Heathrow"
    city                TEXT NOT NULL,
    country_code        CHAR(2) NOT NULL REFERENCES countries(iso_code),
    region_code         TEXT REFERENCES regions(iso_code),

    -- Geography (PostGIS point for geo queries)
    location            GEOGRAPHY(POINT, 4326) NOT NULL,
    elevation_ft        INTEGER,
    timezone            TEXT,                  -- e.g. 'Europe/London'

    -- Classification
    airport_type        TEXT NOT NULL          -- from OurAirports
        CHECK (airport_type IN ('large_airport', 'medium_airport', 'small_airport', 'heliport', 'seaplane_base', 'closed')),
    scheduled_service   BOOLEAN DEFAULT TRUE,

    -- Infrastructure (high level — detail is in runways table)
    terminal_count      SMALLINT,
    total_gates         SMALLINT,
    opened_year         SMALLINT,
    last_major_reno     SMALLINT,              -- year of last major renovation/expansion

    -- Ownership & operation
    operator_id         INTEGER REFERENCES organisations(id),
    owner_id            INTEGER REFERENCES organisations(id),
    ownership_notes     TEXT,                  -- e.g. "51% state via AENA, 49% free float"

    -- Capacity
    annual_capacity_m   NUMERIC(6,2),          -- theoretical max, millions of pax
    annual_pax_2019_m   NUMERIC(6,2),          -- pre-covid baseline
    annual_pax_latest_m NUMERIC(6,2),          -- most recent full year
    latest_pax_year     SMALLINT,

    -- Links
    wikipedia_url       TEXT,
    website_url         TEXT,
    skytrax_url         TEXT,

    -- Metadata
    in_seed_set         BOOLEAN DEFAULT FALSE, -- true for our 15 initial airports
    created_at          TIMESTAMPTZ DEFAULT NOW(),
    updated_at          TIMESTAMPTZ DEFAULT NOW()
);

-- Spatial index for geo queries (nearest airport, radius search etc.)
CREATE INDEX airports_location_gix ON airports USING GIST (location);

-- Trigram indexes for fuzzy name/city search
CREATE INDEX airports_name_trgm ON airports USING GIN (name gin_trgm_ops);
CREATE INDEX airports_city_trgm ON airports USING GIN (city gin_trgm_ops);

-- Standard lookup indexes
CREATE INDEX airports_iata_idx ON airports (iata_code);
CREATE INDEX airports_icao_idx ON airports (icao_code);
CREATE INDEX airports_country_idx ON airports (country_code);
CREATE INDEX airports_type_idx ON airports (airport_type);

-- ============================================================
-- RUNWAYS
-- ============================================================

CREATE TABLE runways (
    id                  SERIAL PRIMARY KEY,
    airport_id          INTEGER NOT NULL REFERENCES airports(id) ON DELETE CASCADE,

    -- Identification
    ident               TEXT,                  -- e.g. "09L/27R"
    le_ident            TEXT,                  -- low end designator e.g. "09L"
    he_ident            TEXT,                  -- high end designator e.g. "27R"

    -- Physical
    length_ft           INTEGER,
    width_ft            INTEGER,
    surface             TEXT,                  -- ASP, CON, GRS, etc.
    lighted             BOOLEAN,
    closed              BOOLEAN DEFAULT FALSE,

    -- Low end
    le_latitude_deg     DOUBLE PRECISION,
    le_longitude_deg    DOUBLE PRECISION,
    le_elevation_ft     INTEGER,
    le_heading_degT     NUMERIC(6,2),
    le_displaced_threshold_ft INTEGER,

    -- High end
    he_latitude_deg     DOUBLE PRECISION,
    he_longitude_deg    DOUBLE PRECISION,
    he_elevation_ft     INTEGER,
    he_heading_degT     NUMERIC(6,2),
    he_displaced_threshold_ft INTEGER,

    created_at          TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX runways_airport_idx ON runways (airport_id);

-- ============================================================
-- RADIO FREQUENCIES
-- ============================================================

CREATE TABLE frequencies (
    id                  SERIAL PRIMARY KEY,
    airport_id          INTEGER NOT NULL REFERENCES airports(id) ON DELETE CASCADE,
    freq_type           TEXT,                  -- TWR, GND, APP, DEP, ATIS, etc.
    description         TEXT,
    frequency_mhz       NUMERIC(7,3) NOT NULL,
    created_at          TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX frequencies_airport_idx ON frequencies (airport_id);

-- ============================================================
-- PASSENGER TRAFFIC (time series)
-- ============================================================

-- Yearly traffic per airport — lets us plot growth trajectories
CREATE TABLE pax_yearly (
    id                  SERIAL PRIMARY KEY,
    airport_id          INTEGER NOT NULL REFERENCES airports(id) ON DELETE CASCADE,
    year                SMALLINT NOT NULL,
    total_pax           BIGINT,
    domestic_pax        BIGINT,
    international_pax   BIGINT,
    aircraft_movements  INTEGER,
    cargo_tonnes        NUMERIC(10,2),
    source              TEXT,                  -- e.g. 'ACI', 'CAA', 'airport_report'
    created_at          TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE (airport_id, year)
);

CREATE INDEX pax_yearly_airport_idx ON pax_yearly (airport_id);
CREATE INDEX pax_yearly_year_idx ON pax_yearly (year);

-- ============================================================
-- DELAYS & OPERATIONAL PERFORMANCE (time series)
-- ============================================================

CREATE TABLE operational_stats (
    id                      SERIAL PRIMARY KEY,
    airport_id              INTEGER NOT NULL REFERENCES airports(id) ON DELETE CASCADE,

    -- Time period
    period_year             SMALLINT NOT NULL,
    period_month            SMALLINT,          -- NULL = full year aggregate
    period_type             TEXT NOT NULL      -- 'annual', 'monthly'
        CHECK (period_type IN ('annual', 'monthly')),

    -- Delay metrics
    total_flights           INTEGER,
    delayed_flights         INTEGER,
    delay_pct               NUMERIC(5,2),      -- % of flights delayed >15min
    avg_delay_minutes       NUMERIC(6,2),
    cancelled_flights       INTEGER,
    cancellation_pct        NUMERIC(5,2),

    -- Cause breakdown (where available)
    delay_weather_pct       NUMERIC(5,2),
    delay_carrier_pct       NUMERIC(5,2),
    delay_atc_pct           NUMERIC(5,2),
    delay_security_pct      NUMERIC(5,2),
    delay_airport_pct       NUMERIC(5,2),

    -- Baggage
    mishandled_bags_per_1k  NUMERIC(6,3),

    source                  TEXT,
    created_at              TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE (airport_id, period_year, period_month, source)
);

CREATE INDEX ops_stats_airport_idx ON operational_stats (airport_id);
CREATE INDEX ops_stats_period_idx ON operational_stats (period_year, period_month);

-- ============================================================
-- SENTIMENT (time series, multi-source)
-- ============================================================

-- Aggregated sentiment snapshots per airport per time period per source.
-- We don't store individual reviews — just aggregated signals.
-- This makes the table compact and queryable without scraping complexity.

CREATE TABLE sentiment_snapshots (
    id                  SERIAL PRIMARY KEY,
    airport_id          INTEGER NOT NULL REFERENCES airports(id) ON DELETE CASCADE,

    -- Source
    source              TEXT NOT NULL          -- 'google', 'skytrax', 'trustpilot', 'tripadvisor', 'airhelp'
        CHECK (source IN ('google', 'skytrax', 'trustpilot', 'tripadvisor', 'airhelp')),

    -- Time period this snapshot covers
    snapshot_year       SMALLINT NOT NULL,
    snapshot_quarter    SMALLINT               -- 1-4, NULL = full year
        CHECK (snapshot_quarter BETWEEN 1 AND 4),

    -- Core metrics
    avg_rating          NUMERIC(3,2),          -- normalised to 0-5 scale
    review_count        INTEGER,
    positive_pct        NUMERIC(5,2),          -- % reviews positive
    negative_pct        NUMERIC(5,2),
    neutral_pct         NUMERIC(5,2),

    -- Skytrax-specific sub-scores (NULL for other sources)
    -- All normalised to 0-5
    score_queuing       NUMERIC(3,2),
    score_cleanliness   NUMERIC(3,2),
    score_staff         NUMERIC(3,2),
    score_food_bev      NUMERIC(3,2),
    score_shopping      NUMERIC(3,2),
    score_wifi          NUMERIC(3,2),
    score_wayfinding    NUMERIC(3,2),
    score_transport     NUMERIC(3,2),

    -- Official ratings (point in time)
    skytrax_stars       SMALLINT               -- 1-5, recorded at snapshot time
        CHECK (skytrax_stars BETWEEN 1 AND 5),

    notes               TEXT,
    created_at          TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE (airport_id, source, snapshot_year, snapshot_quarter)
);

CREATE INDEX sentiment_airport_idx ON sentiment_snapshots (airport_id);
CREATE INDEX sentiment_source_idx ON sentiment_snapshots (source);
CREATE INDEX sentiment_period_idx ON sentiment_snapshots (snapshot_year, snapshot_quarter);

-- ============================================================
-- SCORES (composite, versioned)
-- ============================================================

-- The computed score for an airport at a point in time.
-- Versioned so we can recompute with different weights and keep history.
-- All sub-scores normalised 0-100.

CREATE TABLE airport_scores (
    id                      SERIAL PRIMARY KEY,
    airport_id              INTEGER NOT NULL REFERENCES airports(id) ON DELETE CASCADE,
    score_version           TEXT NOT NULL DEFAULT 'v1',  -- schema version
    scored_at               TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    reference_year          SMALLINT NOT NULL,

    -- Dimension scores (0-100)
    score_infrastructure    NUMERIC(5,2),      -- runways, capacity, age, terminals
    score_operational       NUMERIC(5,2),      -- delays, cancellations, baggage
    score_sentiment         NUMERIC(5,2),      -- aggregated across sources
    score_sentiment_velocity NUMERIC(5,2),     -- is it improving or declining? (50 = flat)
    score_connectivity      NUMERIC(5,2),      -- routes, airlines, international mix
    score_operator          NUMERIC(5,2),      -- operator portfolio average

    -- Composite
    score_total             NUMERIC(5,2),

    -- Weights used (stored so we can reproduce)
    weight_infrastructure   NUMERIC(3,2),
    weight_operational      NUMERIC(3,2),
    weight_sentiment        NUMERIC(3,2),
    weight_sentiment_velocity NUMERIC(3,2),
    weight_connectivity     NUMERIC(3,2),
    weight_operator         NUMERIC(3,2),

    -- Snarky commentary (generated by local ML pipeline)
    commentary              TEXT,

    -- Flags
    is_latest               BOOLEAN DEFAULT TRUE,
    notes                   TEXT,
    created_at              TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX scores_airport_idx ON airport_scores (airport_id);
CREATE INDEX scores_latest_idx ON airport_scores (airport_id, is_latest) WHERE is_latest = TRUE;
CREATE INDEX scores_version_idx ON airport_scores (score_version);

-- ============================================================
-- OPERATOR PORTFOLIO SCORES
-- ============================================================

-- Average scores across all airports an operator manages.
-- Recomputed whenever airport scores update.

CREATE TABLE operator_scores (
    id                  SERIAL PRIMARY KEY,
    organisation_id     INTEGER NOT NULL REFERENCES organisations(id),
    score_version       TEXT NOT NULL DEFAULT 'v1',
    scored_at           TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    reference_year      SMALLINT NOT NULL,
    airport_count       SMALLINT,
    avg_score_total     NUMERIC(5,2),
    avg_score_sentiment NUMERIC(5,2),
    avg_score_operational NUMERIC(5,2),
    is_latest           BOOLEAN DEFAULT TRUE,
    created_at          TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX operator_scores_org_idx ON operator_scores (organisation_id);

-- ============================================================
-- SEED DATA — ORGANISATIONS
-- ============================================================

INSERT INTO countries (iso_code, name, continent) VALUES
    ('GB', 'United Kingdom', 'EU'),
    ('ES', 'Spain', 'EU'),
    ('DE', 'Germany', 'EU'),
    ('FR', 'France', 'EU'),
    ('NL', 'Netherlands', 'EU'),
    ('DK', 'Denmark', 'EU'),
    ('IT', 'Italy', 'EU'),
    ('PL', 'Poland', 'EU'),
    ('HU', 'Hungary', 'EU'),
    ('PT', 'Portugal', 'EU');

INSERT INTO organisations (name, short_name, country_code, org_type, ownership_model, public_share_pct, notes) VALUES
    ('AENA', 'AENA', 'ES', 'both', 'mixed', 51.0, 'Spanish state holds 51%, remainder free float on Madrid stock exchange. Operates 46 Spanish airports.'),
    ('VINCI Airports', 'VINCI', 'FR', 'operator', 'private', 0.0, 'Subsidiary of VINCI Group. Operates 70+ airports globally including Porto, Lyon, Belgrade.'),
    ('Ferrovial Airports', 'Ferrovial', 'ES', 'both', 'private', 0.0, 'Spanish infrastructure group. Major shareholder in Heathrow. Operates several terminals.'),
    ('Heathrow Airport Holdings', 'HAH', 'GB', 'owner', 'private', 0.0, 'Ferrovial 25%, Qatar Investment Authority 20%, others. Owns LHR.'),
    ('Gatwick Airport Ltd', 'Gatwick', 'GB', 'both', 'private', 0.0, 'VINCI Airports 50.01% since 2019. Global Infrastructure Partners minority.'),
    ('Luton Rising', 'Luton Rising', 'GB', 'owner', 'public', 100.0, 'Wholly owned by Luton Borough Council. Contracts operations to private partners.'),
    ('Flughafen München GmbH', 'MUC GmbH', 'DE', 'both', 'mixed', 100.0, 'Free State of Bavaria 51%, Federal Republic of Germany 26%, City of Munich 23%.'),
    ('Flughafen Berlin Brandenburg GmbH', 'FBB', 'DE', 'both', 'mixed', 100.0, 'State of Brandenburg 37%, State of Berlin 37%, Federal Republic 26%.'),
    ('Groupe ADP', 'ADP', 'FR', 'both', 'mixed', 50.6, 'French state holds majority. Operates CDG, Orly, and international concessions.'),
    ('Schiphol Group', 'Schiphol', 'NL', 'both', 'mixed', 69.8, 'Dutch state 69.8%, City of Amsterdam 20.03%, City of Rotterdam 2.87%.'),
    ('Copenhagen Airports A/S', 'CPH Airports', 'DK', 'both', 'mixed', 39.2, 'Danish state 39.2% via Steen & Strøm, Macquarie 29.4%, Ontario Teachers 27.2%.'),
    ('Aeroporti di Roma', 'ADR', 'IT', 'both', 'mixed', 0.0, 'Atlantia (Benetton family) majority. Operates FCO and CIA.'),
    ('Polska Agencja Żeglugi Powietrznej', 'PPL', 'PL', 'both', 'public', 100.0, 'Polish state enterprise. Operates Warsaw Chopin and other Polish airports.'),
    ('Budapest Airport Zrt', 'BUD Airport', 'HU', 'both', 'public', 100.0, 'Renationalised by Hungarian state in 2023 for €3.1bn after years of private ownership under AviAlliance.');

-- ============================================================
-- USEFUL VIEWS
-- ============================================================

-- Latest score per airport with key metadata
CREATE VIEW v_airport_scores_latest AS
SELECT
    a.iata_code,
    a.name,
    a.city,
    a.country_code,
    op.short_name AS operator,
    op.ownership_model,
    op.public_share_pct,
    s.score_total,
    s.score_infrastructure,
    s.score_operational,
    s.score_sentiment,
    s.score_sentiment_velocity,
    s.score_connectivity,
    s.score_operator,
    s.reference_year,
    s.scored_at
FROM airports a
LEFT JOIN airport_scores s ON s.airport_id = a.id AND s.is_latest = TRUE
LEFT JOIN organisations op ON op.id = a.operator_id
ORDER BY s.score_total DESC NULLS LAST;

-- Sentiment trajectory — how is each airport trending?
CREATE VIEW v_sentiment_trajectory AS
SELECT
    a.iata_code,
    a.name,
    ss.source,
    ss.snapshot_year,
    ss.snapshot_quarter,
    ss.avg_rating,
    ss.review_count,
    ss.positive_pct,
    ss.skytrax_stars,
    -- Year-over-year delta in avg rating
    ss.avg_rating - LAG(ss.avg_rating) OVER (
        PARTITION BY a.id, ss.source
        ORDER BY ss.snapshot_year, ss.snapshot_quarter
    ) AS rating_delta
FROM airports a
JOIN sentiment_snapshots ss ON ss.airport_id = a.id
ORDER BY a.iata_code, ss.source, ss.snapshot_year, ss.snapshot_quarter;

-- Operator portfolio comparison
CREATE VIEW v_operator_comparison AS
SELECT
    o.short_name AS operator,
    o.ownership_model,
    o.public_share_pct,
    COUNT(a.id) AS airports_in_dataset,
    ROUND(AVG(s.score_total), 1) AS avg_score,
    ROUND(AVG(s.score_sentiment), 1) AS avg_sentiment,
    ROUND(AVG(s.score_operational), 1) AS avg_operational,
    ROUND(AVG(s.score_sentiment_velocity), 1) AS avg_velocity
FROM organisations o
JOIN airports a ON a.operator_id = o.id
LEFT JOIN airport_scores s ON s.airport_id = a.id AND s.is_latest = TRUE
GROUP BY o.id, o.short_name, o.ownership_model, o.public_share_pct
ORDER BY avg_score DESC NULLS LAST;

-- Nearest airports to a point (example: 50km radius around Madrid)
-- Usage: SELECT * FROM airports WHERE ST_DWithin(location, ST_MakePoint(-3.7038, 40.4168)::geography, 50000);

-- ============================================================
-- DAILY WEATHER (METAR aggregates)
-- ============================================================

CREATE TABLE metar_daily (
    id                  SERIAL PRIMARY KEY,
    airport_id          INTEGER NOT NULL REFERENCES airports(id) ON DELETE CASCADE,
    observation_date    DATE NOT NULL,
    avg_temp_c          NUMERIC(5,2),
    min_temp_c          NUMERIC(5,2),
    max_temp_c          NUMERIC(5,2),
    avg_visibility_m    NUMERIC(8,2),
    min_visibility_m    NUMERIC(8,2),
    avg_wind_speed_kt   NUMERIC(5,2),
    max_wind_speed_kt   NUMERIC(5,2),
    max_wind_gust_kt    NUMERIC(5,2),
    precipitation_flag  BOOLEAN DEFAULT FALSE,
    thunderstorm_flag   BOOLEAN DEFAULT FALSE,
    fog_flag            BOOLEAN DEFAULT FALSE,
    low_ceiling_flag    BOOLEAN DEFAULT FALSE,  -- ceiling < 1000ft
    metar_count         INTEGER,                -- number of METARs aggregated
    created_at          TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE (airport_id, observation_date)
);

CREATE INDEX metar_airport_idx ON metar_daily (airport_id);
CREATE INDEX metar_date_idx ON metar_daily (observation_date);

-- ============================================================
-- ROUTES (from OPDI + OpenSky)
-- ============================================================

CREATE TABLE routes (
    id                  SERIAL PRIMARY KEY,
    origin_id           INTEGER NOT NULL REFERENCES airports(id) ON DELETE CASCADE,
    destination_id      INTEGER REFERENCES airports(id) ON DELETE SET NULL,
    destination_icao    CHAR(4),               -- kept even if destination not in our DB
    destination_iata    CHAR(3),
    airline_icao        TEXT,
    airline_iata        TEXT,
    airline_name        TEXT,
    flights_per_month   INTEGER,
    first_observed      DATE,
    last_observed       DATE,
    data_source         TEXT NOT NULL           -- 'opdi', 'opensky', 'openflights'
        CHECK (data_source IN ('opdi', 'opensky', 'openflights')),
    created_at          TIMESTAMPTZ DEFAULT NOW(),
    updated_at          TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX routes_origin_idx ON routes (origin_id);
CREATE INDEX routes_dest_idx ON routes (destination_id);
CREATE INDEX routes_airline_idx ON routes (airline_icao);
CREATE UNIQUE INDEX routes_unique_idx ON routes (origin_id, destination_icao, airline_icao, data_source);

-- ============================================================
-- REVIEWS RAW (staging for ML pipeline)
-- ============================================================

CREATE TABLE reviews_raw (
    id                  SERIAL PRIMARY KEY,
    airport_id          INTEGER NOT NULL REFERENCES airports(id) ON DELETE CASCADE,
    source              TEXT NOT NULL DEFAULT 'skytrax',
    review_date         DATE,
    author              TEXT,
    author_country      TEXT,
    overall_rating      SMALLINT               -- 1-10 for Skytrax
        CHECK (overall_rating BETWEEN 1 AND 10),
    score_queuing       SMALLINT CHECK (score_queuing BETWEEN 1 AND 5),
    score_cleanliness   SMALLINT CHECK (score_cleanliness BETWEEN 1 AND 5),
    score_staff         SMALLINT CHECK (score_staff BETWEEN 1 AND 5),
    score_food_bev      SMALLINT CHECK (score_food_bev BETWEEN 1 AND 5),
    score_wifi          SMALLINT CHECK (score_wifi BETWEEN 1 AND 5),
    score_wayfinding    SMALLINT CHECK (score_wayfinding BETWEEN 1 AND 5),
    score_transport     SMALLINT CHECK (score_transport BETWEEN 1 AND 5),
    recommended         BOOLEAN,
    verified            BOOLEAN DEFAULT FALSE,
    trip_type           TEXT,                  -- 'Solo Leisure', 'Business', 'Family Leisure', etc.
    review_title        TEXT,
    review_text         TEXT,
    source_url          TEXT,
    processed           BOOLEAN DEFAULT FALSE, -- flag for ML pipeline
    created_at          TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX reviews_airport_idx ON reviews_raw (airport_id);
CREATE INDEX reviews_date_idx ON reviews_raw (review_date);
CREATE INDEX reviews_unprocessed_idx ON reviews_raw (processed) WHERE processed = FALSE;

-- ============================================================
-- AIRPORT SLUGS (source-specific identifiers)
-- ============================================================

CREATE TABLE airport_slugs (
    airport_id          INTEGER NOT NULL REFERENCES airports(id) ON DELETE CASCADE,
    source              TEXT NOT NULL,          -- 'skytrax', 'skytrax_ratings', 'trustpilot', 'eurocontrol'
    slug                TEXT NOT NULL,
    PRIMARY KEY (airport_id, source)
);

CREATE INDEX slugs_source_idx ON airport_slugs (source);

-- ============================================================
-- PIPELINE RUNS (tracking/audit)
-- ============================================================

CREATE TABLE pipeline_runs (
    id                  SERIAL PRIMARY KEY,
    airport_id          INTEGER REFERENCES airports(id) ON DELETE CASCADE,
    source              TEXT NOT NULL,          -- matches data source name
    started_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at        TIMESTAMPTZ,
    status              TEXT NOT NULL DEFAULT 'running'
        CHECK (status IN ('running', 'success', 'failed')),
    records_processed   INTEGER DEFAULT 0,
    last_record_date    DATE,                  -- drives incremental: next run fetches since this date
    error_message       TEXT,
    created_at          TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX pipeline_airport_idx ON pipeline_runs (airport_id);
CREATE INDEX pipeline_source_idx ON pipeline_runs (source);
CREATE INDEX pipeline_status_idx ON pipeline_runs (status);
