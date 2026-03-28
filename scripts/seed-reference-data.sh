#!/usr/bin/env bash
# Seed reference data: countries, organisations, airport slugs, views, constraints.
# Replaces the old Drizzle-based seed.ts.
# Usage: bash scripts/seed-reference-data.sh
# Requires: psql, DATABASE_URL env var

set -euo pipefail

if [ -f .env ]; then
  export $(grep -v '^#' .env | xargs)
fi

DB="${DATABASE_URL:?DATABASE_URL not set}"

echo "Seeding reference data..."

psql "$DB" -q <<'SQL'

-- Extensions
CREATE EXTENSION IF NOT EXISTS postgis;
CREATE EXTENSION IF NOT EXISTS pg_trgm;

-- ── Countries ──────────────────────────────────────────────
INSERT INTO countries (iso_code, name, continent) VALUES
  ('GB', 'United Kingdom', 'EU'),
  ('IE', 'Ireland', 'EU'),
  ('FR', 'France', 'EU'),
  ('DE', 'Germany', 'EU'),
  ('NL', 'Netherlands', 'EU'),
  ('BE', 'Belgium', 'EU'),
  ('LU', 'Luxembourg', 'EU'),
  ('AT', 'Austria', 'EU'),
  ('CH', 'Switzerland', 'EU'),
  ('ES', 'Spain', 'EU'),
  ('PT', 'Portugal', 'EU'),
  ('IT', 'Italy', 'EU'),
  ('GR', 'Greece', 'EU'),
  ('MT', 'Malta', 'EU'),
  ('CY', 'Cyprus', 'EU'),
  ('HR', 'Croatia', 'EU'),
  ('SI', 'Slovenia', 'EU'),
  ('ME', 'Montenegro', 'EU'),
  ('AL', 'Albania', 'EU'),
  ('MK', 'North Macedonia', 'EU'),
  ('RS', 'Serbia', 'EU'),
  ('BA', 'Bosnia and Herzegovina', 'EU'),
  ('XK', 'Kosovo', 'EU'),
  ('DK', 'Denmark', 'EU'),
  ('SE', 'Sweden', 'EU'),
  ('NO', 'Norway', 'EU'),
  ('FI', 'Finland', 'EU'),
  ('IS', 'Iceland', 'EU'),
  ('PL', 'Poland', 'EU'),
  ('CZ', 'Czech Republic', 'EU'),
  ('SK', 'Slovakia', 'EU'),
  ('HU', 'Hungary', 'EU'),
  ('RO', 'Romania', 'EU'),
  ('BG', 'Bulgaria', 'EU'),
  ('LT', 'Lithuania', 'EU'),
  ('LV', 'Latvia', 'EU'),
  ('EE', 'Estonia', 'EU'),
  ('UA', 'Ukraine', 'EU'),
  ('MD', 'Moldova', 'EU'),
  ('BY', 'Belarus', 'EU'),
  ('TR', 'Türkiye', 'AS'),
  ('GE', 'Georgia', 'AS'),
  ('AM', 'Armenia', 'AS'),
  ('AZ', 'Azerbaijan', 'AS'),
  ('MA', 'Morocco', 'AF'),
  ('TN', 'Tunisia', 'AF'),
  ('EG', 'Egypt', 'AF'),
  ('AE', 'United Arab Emirates', 'AS'),
  ('QA', 'Qatar', 'AS'),
  ('IL', 'Israel', 'AS'),
  ('RU', 'Russia', 'EU')
ON CONFLICT (iso_code) DO NOTHING;

-- ── Organisations ──────────────────────────────────────────
INSERT INTO organisations (name, short_name, country_code, org_type, ownership_model, public_share_pct, notes) VALUES
  ('AENA', 'AENA', 'ES', 'both', 'mixed', 51.00, 'Spanish state holds 51%. Operates 46 Spanish airports.'),
  ('Groupe ADP', 'ADP', 'FR', 'both', 'mixed', 50.60, 'French state holds majority. Operates CDG, Orly.'),
  ('VINCI Airports', 'VINCI', 'FR', 'operator', 'private', 0.00, 'Operates 70+ airports globally including Porto, Lyon, Gatwick.'),
  ('Edeis', 'Edeis', 'FR', 'operator', 'private', 0.00, 'French airport operator managing 20+ regional airports.'),
  ('Heathrow Airport Holdings', 'HAH', 'GB', 'owner', 'private', 0.00, 'Ferrovial 25%, Qatar Investment Authority 20%. Owns LHR.'),
  ('Gatwick Airport Ltd', 'Gatwick', 'GB', 'both', 'private', 0.00, 'VINCI Airports 50.01% since 2019.'),
  ('Luton Rising', 'Luton Rising', 'GB', 'owner', 'public', 100.00, 'Wholly owned by Luton Borough Council.'),
  ('MAG (Manchester Airports Group)', 'MAG', 'GB', 'both', 'mixed', 64.00, 'Manchester, Stansted, East Midlands.'),
  ('AGS Airports', 'AGS', 'GB', 'both', 'private', 0.00, 'Ferrovial/Macquarie JV. Glasgow, Aberdeen, Southampton.'),
  ('Ferrovial Airports', 'Ferrovial', 'ES', 'both', 'private', 0.00, 'Major shareholder in Heathrow, AGS Airports.'),
  ('Flughafen München GmbH', 'MUC GmbH', 'DE', 'both', 'mixed', 100.00, 'Bavaria 51%, Federal 26%, Munich 23%.'),
  ('Flughafen Berlin Brandenburg GmbH', 'FBB', 'DE', 'both', 'mixed', 100.00, 'Brandenburg 37%, Berlin 37%, Federal 26%.'),
  ('Fraport AG', 'Fraport', 'DE', 'both', 'mixed', 51.40, 'Operates Frankfurt, Lima, Antalya, Greek regionals.'),
  ('Flughafen Düsseldorf GmbH', 'DUS GmbH', 'DE', 'both', 'mixed', 50.00, 'City of Düsseldorf 50%, Airport Partners 50%.'),
  ('Flughafen Hamburg GmbH', 'HAM GmbH', 'DE', 'both', 'mixed', 100.00, 'City of Hamburg 51%, AviAlliance 49%.'),
  ('Schiphol Group', 'Schiphol', 'NL', 'both', 'mixed', 69.80, 'Dutch state 69.8%, Amsterdam 20.03%.'),
  ('Brussels Airport Company', 'BAC', 'BE', 'both', 'private', 25.00, 'Ontario Teachers 39%, Macquarie 36%, Belgian state 25%.'),
  ('Aeroporti di Roma', 'ADR', 'IT', 'both', 'mixed', 0.00, 'Mundys (Benetton family). Operates FCO and CIA.'),
  ('SEA Aeroporti di Milano', 'SEA Milano', 'IT', 'both', 'mixed', 54.81, 'City of Milan 54.81%. Operates MXP and LIN.'),
  ('SAVE S.p.A.', 'SAVE', 'IT', 'both', 'mixed', 0.00, 'Operates Venice, Treviso, Verona.'),
  ('Copenhagen Airports A/S', 'CPH Airports', 'DK', 'both', 'mixed', 39.20, 'Danish state 39.2%, Macquarie 29.4%.'),
  ('Swedavia AB', 'Swedavia', 'SE', 'both', 'public', 100.00, 'Swedish state-owned. 10 airports.'),
  ('Avinor AS', 'Avinor', 'NO', 'both', 'public', 100.00, 'Norwegian state-owned. 43 airports.'),
  ('Finavia Oyj', 'Finavia', 'FI', 'both', 'public', 100.00, 'Finnish state-owned. 21 airports.'),
  ('daa plc', 'daa', 'IE', 'both', 'public', 100.00, 'Irish state-owned. Dublin and Cork.'),
  ('Polska Agencja Żeglugi Powietrznej', 'PPL', 'PL', 'both', 'public', 100.00, 'Polish state. Warsaw Chopin.'),
  ('Budapest Airport Zrt', 'BUD Airport', 'HU', 'both', 'public', 100.00, 'Renationalised 2023 for €3.1bn.'),
  ('Letiště Praha a.s.', 'Prague Airport', 'CZ', 'both', 'public', 100.00, 'Czech state-owned. Operates PRG.'),
  ('Flughafen Wien AG', 'VIE AG', 'AT', 'both', 'mixed', 40.00, 'Lower Austria 20%, Vienna 20%.'),
  ('Flughafen Zürich AG', 'ZRH AG', 'CH', 'both', 'mixed', 33.33, 'Canton of Zürich 33.33%.'),
  ('ANA Aeroportos de Portugal', 'ANA', 'PT', 'both', 'private', 0.00, 'VINCI subsidiary since 2013.'),
  ('Athens International Airport SA', 'AIA', 'GR', 'both', 'mixed', 30.00, 'Greek state 30%, AviAlliance 40%.'),
  ('Fraport Greece', 'Fraport GR', 'GR', 'operator', 'private', 0.00, '14 Greek regional airports, 40-year concession.'),
  ('Bucharest Airports National Company', 'CNAB', 'RO', 'both', 'public', 100.00, 'Romanian state-owned. OTP and BBU.'),
  ('İGA Havalimanı İşletmesi', 'İGA', 'TR', 'operator', 'private', 0.00, 'Operates Istanbul Airport (IST).'),
  ('TAV Airports', 'TAV', 'TR', 'operator', 'private', 0.00, 'Groupe ADP subsidiary. Ankara, Izmir, Tbilisi.'),
  ('Isavia ohf.', 'Isavia', 'IS', 'both', 'public', 100.00, 'Icelandic state-owned. Keflavík.'),
  ('MZLZ (Zagreb Airport)', 'MZLZ', 'HR', 'operator', 'private', 0.00, 'ADP/TAV/ZAIC consortium. 30-year concession.')
ON CONFLICT DO NOTHING;

-- ── Views ──────────────────────────────────────────────────
CREATE OR REPLACE VIEW v_airport_scores_latest AS
SELECT
  a.iata_code, a.name, a.city, a.country_code,
  op.short_name AS operator, op.ownership_model, op.public_share_pct,
  s.score_total, s.score_infrastructure, s.score_operational,
  s.score_sentiment, s.score_sentiment_velocity, s.score_connectivity, s.score_operator,
  s.reference_year, s.created_at AS scored_at
FROM airports a
LEFT JOIN airport_scores s ON s.airport_id = a.id AND s.is_latest = TRUE
LEFT JOIN organisations op ON op.id = a.operator_id
ORDER BY s.score_total DESC NULLS LAST;

CREATE OR REPLACE VIEW v_sentiment_trajectory AS
SELECT
  a.iata_code, a.name, ss.source,
  ss.snapshot_year, ss.snapshot_quarter,
  ss.avg_rating, ss.review_count, ss.positive_pct, ss.skytrax_stars,
  ss.avg_rating - LAG(ss.avg_rating) OVER (
    PARTITION BY a.id, ss.source ORDER BY ss.snapshot_year, ss.snapshot_quarter
  ) AS rating_delta
FROM airports a
JOIN sentiment_snapshots ss ON ss.airport_id = a.id
ORDER BY a.iata_code, ss.source, ss.snapshot_year, ss.snapshot_quarter;

CREATE OR REPLACE VIEW v_operator_comparison AS
SELECT
  o.short_name AS operator, o.ownership_model, o.public_share_pct,
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

-- ── PostGIS location column ────────────────────────────────
DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM information_schema.columns
    WHERE table_name = 'airports' AND column_name = 'location'
  ) THEN
    ALTER TABLE airports ADD COLUMN location geography(POINT, 4326);
  END IF;
END $$;

CREATE INDEX IF NOT EXISTS airports_location_gix ON airports USING GIST (location);
CREATE INDEX IF NOT EXISTS airports_name_trgm ON airports USING GIN (name gin_trgm_ops);
CREATE INDEX IF NOT EXISTS airports_city_trgm ON airports USING GIN (city gin_trgm_ops);

-- ── Unique constraints for ON CONFLICT upserts ─────────────
DO $$ BEGIN
  ALTER TABLE pax_yearly ADD CONSTRAINT pax_yearly_airport_year_unique UNIQUE (airport_id, year);
EXCEPTION WHEN duplicate_object THEN NULL; END $$;

DO $$ BEGIN
  ALTER TABLE metar_daily ADD CONSTRAINT metar_daily_airport_date_unique UNIQUE (airport_id, observation_date);
EXCEPTION WHEN duplicate_object THEN NULL; END $$;

DO $$ BEGIN
  ALTER TABLE operational_stats ADD CONSTRAINT ops_stats_unique UNIQUE (airport_id, period_year, period_month, source);
EXCEPTION WHEN duplicate_object THEN NULL; END $$;

DO $$ BEGIN
  ALTER TABLE sentiment_snapshots ADD CONSTRAINT sentiment_unique UNIQUE (airport_id, source, snapshot_year, snapshot_quarter);
EXCEPTION WHEN duplicate_object THEN NULL; END $$;

CREATE UNIQUE INDEX IF NOT EXISTS routes_icao_unique_idx ON routes (origin_id, destination_icao, airline_icao, data_source)
  WHERE data_source IN ('opdi', 'opensky');

CREATE UNIQUE INDEX IF NOT EXISTS routes_iata_unique_idx ON routes (origin_id, destination_iata, airline_iata, data_source)
  WHERE data_source = 'openflights';

SQL

echo "Reference data seeded (countries, organisations, views, constraints)"
