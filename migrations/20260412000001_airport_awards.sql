-- Airport awards from Skytrax World Airport Awards and ACI ASQ Awards.
-- Stores iata_code directly (not airport_id) so awards exist even before
-- an airport is added to the supported_airports / airports tables.
-- Seeded from data/skytrax_awards.json and data/aci_asq_awards.json via scripts/seed-awards.sh.
CREATE TABLE IF NOT EXISTS airport_awards (
    id              SERIAL PRIMARY KEY,
    iata_code       TEXT NOT NULL,
    source          TEXT NOT NULL CHECK (source IN ('skytrax', 'aci_asq')),
    year            SMALLINT NOT NULL,
    category        TEXT NOT NULL,
    region          TEXT,
    size_bucket     TEXT,
    rank            SMALLINT,
    created_at      TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE (iata_code, source, year, category)
);

CREATE INDEX IF NOT EXISTS airport_awards_iata_idx ON airport_awards (iata_code);
CREATE INDEX IF NOT EXISTS airport_awards_source_year_idx ON airport_awards (source, year);

-- Drop the old Wikipedia-scraped ACI awards column (replaced by airport_awards table).
ALTER TABLE wikipedia_snapshots DROP COLUMN IF EXISTS aci_awards;
