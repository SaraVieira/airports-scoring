-- Add unique constraint needed for ON CONFLICT upsert
CREATE UNIQUE INDEX IF NOT EXISTS metar_daily_airport_date_uniq
    ON metar_daily (airport_id, observation_date);
