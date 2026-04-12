-- avg_rating is on a 1-10 scale per convention, but was declared NUMERIC(3,2)
-- which overflows at exactly 10.00 (all-5-star quarters from Google).
ALTER TABLE sentiment_snapshots
    ALTER COLUMN avg_rating TYPE NUMERIC(4,2);
