-- Add slug column to organisations for URL-friendly lookups.
CREATE EXTENSION IF NOT EXISTS unaccent;

ALTER TABLE organisations ADD COLUMN IF NOT EXISTS slug TEXT;
CREATE UNIQUE INDEX IF NOT EXISTS organisations_slug_idx ON organisations (slug);

-- Backfill existing rows.
UPDATE organisations
SET slug = LOWER(TRIM(BOTH '-' FROM REGEXP_REPLACE(
    REGEXP_REPLACE(unaccent(name), '-+', '-', 'g'),
    '[^a-zA-Z0-9]+', '-', 'g'
)))
WHERE slug IS NULL;
