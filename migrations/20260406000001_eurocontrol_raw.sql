-- Eurocontrol raw data cache: stores daily records from all datasets
-- so per-airport fetchers can read locally instead of downloading CSVs each time.

CREATE TABLE eurocontrol_raw (
    id                   BIGSERIAL PRIMARY KEY,
    dataset              TEXT NOT NULL,
    apt_icao             TEXT NOT NULL,
    flight_date          DATE,
    year                 SMALLINT NOT NULL,
    month                SMALLINT NOT NULL,

    -- Common
    total_flights        INTEGER,
    ifr_flights          INTEGER,

    -- ASMA / Taxi (additional time beyond unimpeded reference)
    additional_time_min  NUMERIC(10,2),
    reference_time_min   NUMERIC(10,2),
    reference_flights    INTEGER,

    -- apt_dly (delay cause breakdown)
    arr_flights          INTEGER,
    delayed_flights      INTEGER,
    total_atfm_delay_min NUMERIC(10,2),
    dly_weather_min      NUMERIC(10,2),
    dly_atc_min          NUMERIC(10,2),
    dly_carrier_min      NUMERIC(10,2),
    dly_airport_min      NUMERIC(10,2),

    -- Vertical flight efficiency
    cdo_flights          INTEGER,
    cco_flights          INTEGER,
    total_flights_vfe    INTEGER,
    delta_co2_kg_descent NUMERIC(12,2),
    delta_co2_kg_climb   NUMERIC(12,2),

    -- Slot adherence
    slot_departures      INTEGER,
    slot_early           INTEGER,
    slot_on_time         INTEGER,
    slot_late            INTEGER,

    ingested_at          TIMESTAMPTZ DEFAULT NOW(),

    UNIQUE NULLS NOT DISTINCT (dataset, apt_icao, year, month, flight_date)
);

CREATE INDEX eurocontrol_raw_icao_idx ON eurocontrol_raw (apt_icao);
CREATE INDEX eurocontrol_raw_dataset_year_idx ON eurocontrol_raw (dataset, year);

-- Track sync runs to avoid re-downloading
CREATE TABLE eurocontrol_sync_log (
    id        SERIAL PRIMARY KEY,
    dataset   TEXT NOT NULL,
    year      SMALLINT NOT NULL,
    row_count INTEGER NOT NULL,
    synced_at TIMESTAMPTZ DEFAULT NOW()
);

-- New columns on operational_stats for metrics we weren't tracking
ALTER TABLE operational_stats
    ADD COLUMN IF NOT EXISTS asma_additional_min     NUMERIC(10,2),
    ADD COLUMN IF NOT EXISTS taxi_out_additional_min  NUMERIC(10,2),
    ADD COLUMN IF NOT EXISTS taxi_in_additional_min   NUMERIC(10,2),
    ADD COLUMN IF NOT EXISTS slot_adherence_pct       NUMERIC(5,2),
    ADD COLUMN IF NOT EXISTS cdo_pct                  NUMERIC(5,2),
    ADD COLUMN IF NOT EXISTS cco_pct                  NUMERIC(5,2),
    ADD COLUMN IF NOT EXISTS co2_waste_kg_per_flight  NUMERIC(10,2);
