-- Replaces airports.json as the config source for tracked airports.

CREATE TABLE IF NOT EXISTS supported_airports (
    iata_code           TEXT PRIMARY KEY,
    country_code        TEXT NOT NULL,
    name                TEXT NOT NULL,
    skytrax_review_slug TEXT,
    skytrax_rating_slug TEXT,
    google_maps_url     TEXT,
    enabled             BOOLEAN NOT NULL DEFAULT TRUE,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Per-airport per-source fetch tracking.
CREATE TABLE IF NOT EXISTS source_status (
    iata_code        TEXT NOT NULL REFERENCES supported_airports(iata_code) ON DELETE CASCADE,
    source           TEXT NOT NULL,
    last_fetched_at  TIMESTAMPTZ,
    last_status      TEXT NOT NULL DEFAULT 'pending',
    last_record_count INTEGER DEFAULT 0,
    last_error       TEXT,
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (iata_code, source)
);

CREATE INDEX IF NOT EXISTS idx_source_status_stale
    ON source_status (iata_code, source, last_fetched_at);
