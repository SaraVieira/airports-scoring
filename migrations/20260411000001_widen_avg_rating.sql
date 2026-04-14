-- avg_rating is on a 1-10 scale per convention, but was declared NUMERIC(3,2)
-- which overflows at exactly 10.00 (all-5-star quarters from Google).
-- Drop and recreate the dependent view since Postgres won't let us alter
-- a column type while a view references it.

DROP VIEW IF EXISTS v_sentiment_trajectory;

ALTER TABLE sentiment_snapshots
    ALTER COLUMN avg_rating TYPE NUMERIC(4,2);

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
    ss.avg_rating - LAG(ss.avg_rating) OVER (
        PARTITION BY a.id, ss.source
        ORDER BY ss.snapshot_year, ss.snapshot_quarter
    ) AS rating_delta
FROM airports a
JOIN sentiment_snapshots ss ON ss.airport_id = a.id
ORDER BY a.iata_code, ss.source, ss.snapshot_year, ss.snapshot_quarter;
