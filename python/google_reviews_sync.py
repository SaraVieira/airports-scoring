#!/usr/bin/env python3
"""
Google Reviews bootstrap sync helper.

Standalone script for initial large-airport bootstraps (BER, LHR, CDG).
Calls the google-reviews-scraper-pro REST API, polls for completion with
progress reporting, and inserts reviews directly into Postgres.

Usage:
    python google_reviews_sync.py --airport BER --limit 2000
    python google_reviews_sync.py --airport BER  # no limit = fetch all

Requires:
    - google-reviews-scraper-pro running at GOOGLE_SCRAPER_URL (default: http://localhost:8000)
    - GOOGLE_SCRAPER_API_KEY env var
    - DATABASE_URL env var
    - pip install psycopg2-binary
"""

import argparse
import hashlib
import json
import logging
import os
import sys
import time

import psycopg2
import psycopg2.extras

# ---------------------------------------------------------------------------
# Logging
# ---------------------------------------------------------------------------
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(message)s",
    stream=sys.stderr,
)
logger = logging.getLogger(__name__)

# ---------------------------------------------------------------------------
# Config
# ---------------------------------------------------------------------------

def load_airports_json() -> dict[str, dict]:
    """Load airports.json and return a dict keyed by IATA code."""
    path = os.path.join(os.path.dirname(__file__), "..", "airports.json")
    with open(path) as f:
        airports = json.load(f)
    return {a["iata"]: a for a in airports}


def synthetic_source_url(iata: str, reviewer_name: str, date_iso: str, text: str) -> str:
    """Generate deterministic source_url matching the Rust fetcher's logic."""
    name = reviewer_name or ""
    date = date_iso or ""
    snippet = (text or "")[:100]
    payload = f"{name}|{date}|{snippet}"
    h = hashlib.sha256(payload.encode("utf-8")).hexdigest()
    return f"google://{iata}/{h}"


# ---------------------------------------------------------------------------
# Scraper API
# ---------------------------------------------------------------------------

def check_scraper_health(base_url: str) -> bool:
    """Check if the scraper service is reachable."""
    import urllib.request
    try:
        req = urllib.request.Request(f"{base_url}/", method="GET")
        with urllib.request.urlopen(req, timeout=2):
            return True
    except Exception:
        return False


def submit_scrape_job(base_url: str, api_key: str, google_maps_url: str) -> str:
    """Submit a scrape job and return the job_id."""
    import urllib.request
    data = json.dumps({
        "url": google_maps_url,
        "headless": True,
        "sort_by": "newest",
        "download_images": False,
    }).encode("utf-8")
    req = urllib.request.Request(
        f"{base_url}/scrape",
        data=data,
        headers={
            "Content-Type": "application/json",
            "X-API-Key": api_key,
        },
        method="POST",
    )
    with urllib.request.urlopen(req) as resp:
        result = json.loads(resp.read())
    return result["job_id"]


def poll_job(base_url: str, api_key: str, job_id: str, timeout: int = 600) -> list[dict]:
    """Poll for job completion with exponential backoff. Returns reviews list."""
    import urllib.request
    interval = 5
    max_interval = 30
    deadline = time.time() + timeout

    while time.time() < deadline:
        time.sleep(interval)
        interval = min(interval * 2, max_interval)

        req = urllib.request.Request(
            f"{base_url}/jobs/{job_id}",
            headers={"X-API-Key": api_key},
            method="GET",
        )
        with urllib.request.urlopen(req) as resp:
            status = json.loads(resp.read())

        if status["status"] == "completed":
            return status.get("reviews", [])
        elif status["status"] == "failed":
            raise RuntimeError(f"Scrape job {job_id} failed")
        else:
            elapsed = timeout - (deadline - time.time())
            logger.info("Job %s still %s (%.0fs elapsed)...", job_id, status["status"], elapsed)

    raise TimeoutError(f"Scrape job {job_id} timed out after {timeout}s")


# ---------------------------------------------------------------------------
# Database
# ---------------------------------------------------------------------------

def get_airport_id(conn, iata: str) -> int:
    """Look up airport ID by IATA code."""
    with conn.cursor() as cur:
        cur.execute("SELECT id FROM airports WHERE iata_code = %s", (iata.upper(),))
        row = cur.fetchone()
        if not row:
            raise ValueError(f"Airport {iata} not found in database")
        return row[0]


def upsert_reviews(conn, airport_id: int, iata: str, reviews: list[dict], limit: int | None) -> int:
    """Insert Google reviews into reviews_raw. Returns count of records upserted."""
    count = 0
    with conn.cursor() as cur:
        for review in reviews:
            if limit is not None and count >= limit:
                break

            reviewer_name = review.get("reviewer_name")
            date_iso = review.get("date_iso")
            text = review.get("text")
            rating = review.get("rating")

            # Normalize rating: Google 1-5 → 1-10 scale
            overall_rating = rating * 2 if rating else None

            # Parse date
            review_date = None
            if date_iso:
                # Strip time component if present
                review_date = date_iso[:10] if len(date_iso) >= 10 else date_iso

            source_url = synthetic_source_url(iata, reviewer_name or "", date_iso or "", text or "")

            try:
                cur.execute(
                    """
                    INSERT INTO reviews_raw (
                        airport_id, source, review_date, author,
                        overall_rating, review_text, source_url
                    ) VALUES (
                        %s, 'google', %s, %s, %s, %s, %s
                    )
                    ON CONFLICT (source_url) DO UPDATE SET
                        overall_rating = EXCLUDED.overall_rating,
                        review_text = EXCLUDED.review_text
                    """,
                    (airport_id, review_date, reviewer_name, overall_rating, text, source_url),
                )
                count += 1
            except Exception as e:
                logger.warning("Failed to upsert review: %s", e)
                conn.rollback()
                continue

            if count % 100 == 0:
                conn.commit()
                logger.info("Progress: %d reviews upserted...", count)

    conn.commit()
    return count


def update_pipeline_run(conn, airport_id: int, records: int, last_date: str | None):
    """Record the pipeline run in pipeline_runs."""
    with conn.cursor() as cur:
        cur.execute(
            """
            INSERT INTO pipeline_runs (airport_id, source, status, records_processed, last_record_date)
            VALUES (%s, 'google_reviews', 'success', %s, %s)
            """,
            (airport_id, records, last_date),
        )
    conn.commit()


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    parser = argparse.ArgumentParser(
        description="Bootstrap Google Reviews for an airport via the scraper REST API."
    )
    parser.add_argument(
        "--airport", required=True,
        help="IATA airport code (e.g. BER, LHR)"
    )
    parser.add_argument(
        "--limit", type=int, default=None,
        help="Maximum number of reviews to insert (default: no limit)"
    )
    parser.add_argument(
        "--timeout", type=int, default=600,
        help="Max seconds to wait for scrape job (default: 600)"
    )
    args = parser.parse_args()

    iata = args.airport.upper()

    # Load config
    airports = load_airports_json()
    if iata not in airports:
        logger.error("Airport %s not found in airports.json", iata)
        sys.exit(1)

    google_maps_url = airports[iata].get("google_maps_url")
    if not google_maps_url:
        logger.error("No google_maps_url configured for %s in airports.json", iata)
        sys.exit(1)

    # Env vars
    base_url = os.environ.get("GOOGLE_SCRAPER_URL", "http://localhost:8000")
    api_key = os.environ.get("GOOGLE_SCRAPER_API_KEY", "")
    db_url = os.environ.get("DATABASE_URL")
    if not db_url:
        logger.error("DATABASE_URL env var not set")
        sys.exit(1)

    # Health check
    if not check_scraper_health(base_url):
        logger.error(
            "Google Reviews scraper not reachable at %s. "
            "Start it with: bash scripts/start-google-scraper.sh",
            base_url,
        )
        sys.exit(1)

    # Connect to DB
    logger.info("Connecting to database...")
    conn = psycopg2.connect(db_url)
    airport_id = get_airport_id(conn, iata)

    # Submit scrape job
    logger.info("Submitting scrape job for %s (%s)...", iata, google_maps_url)
    job_id = submit_scrape_job(base_url, api_key, google_maps_url)
    logger.info("Job submitted: %s", job_id)

    # Poll for completion
    logger.info("Polling for completion (timeout: %ds)...", args.timeout)
    reviews = poll_job(base_url, api_key, job_id, timeout=args.timeout)
    logger.info("Scrape completed: %d reviews returned", len(reviews))

    # Insert reviews
    count = upsert_reviews(conn, airport_id, iata, reviews, limit=args.limit)
    logger.info("Upserted %d reviews for %s", count, iata)

    # Find latest date for pipeline_runs
    last_date = None
    for review in reviews[:count] if args.limit else reviews:
        d = review.get("date_iso", "")[:10] if review.get("date_iso") else None
        if d and (last_date is None or d > last_date):
            last_date = d

    # Record pipeline run
    update_pipeline_run(conn, airport_id, count, last_date)
    logger.info("Pipeline run recorded. Done.")

    conn.close()


if __name__ == "__main__":
    main()
