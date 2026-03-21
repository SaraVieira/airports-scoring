#!/usr/bin/env python3
"""
Import historical Skytrax reviews from the quankiquanki GitHub dataset.

Downloads the raw CSV files, maps columns to the reviews_raw table schema,
and inserts reviews for the 15 seed airports.

Usage:
    python import_historical.py --db-url postgres://user:pass@host/db
"""

import argparse
import csv
import io
import logging
import os
import re
import sys
from datetime import datetime

import psycopg2
import psycopg2.extras
import requests

# ---------------------------------------------------------------------------
# Logging – stderr only; stdout reserved for JSON output
# ---------------------------------------------------------------------------
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(message)s",
    stream=sys.stderr,
)
logger = logging.getLogger(__name__)

# ---------------------------------------------------------------------------
# CSV URLs from the quankiquanki/skytrax-reviews-dataset repo
# ---------------------------------------------------------------------------
CSV_URLS = [
    "https://raw.githubusercontent.com/quankiquanki/skytrax-reviews-dataset/master/data/airport.csv",
]

# ---------------------------------------------------------------------------
# Airport name matching: names that appear in the dataset -> IATA codes
# We use fuzzy substring matching on the airport_name column.
# ---------------------------------------------------------------------------
AIRPORT_NAME_MAP = {
    "heathrow":             "LHR",
    "london-heathrow":      "LHR",
    "gatwick":              "LGW",
    "london-gatwick":       "LGW",
    "luton":                "LTN",
    "london-luton":         "LTN",
    "porto":                "OPO",
    "porto-airport":        "OPO",
    "madrid-barajas":       "MAD",
    "madrid":               "MAD",
    "barajas":              "MAD",
    "barcelona-el-prat":    "BCN",
    "barcelona":            "BCN",
    "el-prat":              "BCN",
    "berlin-brandenburg":   "BER",
    "berlin-schoenefeld":   "BER",
    "berlin-schonefeld":    "BER",
    "berlin-tegel":         "BER",
    "brandenburg":          "BER",
    "munich":               "MUC",
    "munich-airport":       "MUC",
    "paris-cdg":            "CDG",
    "charles-de-gaulle":    "CDG",
    "paris-charles-de-gaulle": "CDG",
    "nice-cote-d-azur":     "NCE",
    "nice":                 "NCE",
    "nice-airport":         "NCE",
    "amsterdam-schiphol":   "AMS",
    "schiphol":             "AMS",
    "copenhagen":           "CPH",
    "copenhagen-airport":   "CPH",
    "kastrup":              "CPH",
    "rome-fiumicino":       "FCO",
    "fiumicino":            "FCO",
    "leonardo-da-vinci":    "FCO",
    "warsaw-chopin":        "WAW",
    "warsaw":               "WAW",
    "chopin":               "WAW",
    "budapest":             "BUD",
    "budapest-airport":     "BUD",
    "budapest-liszt-ferenc": "BUD",
    "ferihegy":             "BUD",
}

SEED_IATA_CODES = {
    "LHR", "LGW", "LTN", "OPO", "MAD", "BCN", "BER", "MUC",
    "CDG", "NCE", "AMS", "CPH", "FCO", "WAW", "BUD",
}


def match_airport(name: str) -> str | None:
    """Try to match an airport name string to one of our seed IATA codes."""
    if not name:
        return None
    normalised = name.strip().lower().replace(" ", "-").replace("_", "-")
    # Direct lookup
    if normalised in AIRPORT_NAME_MAP:
        return AIRPORT_NAME_MAP[normalised]
    # Substring matching
    for pattern, iata in AIRPORT_NAME_MAP.items():
        if pattern in normalised or normalised in pattern:
            return iata
    return None


def _parse_date(text: str) -> str | None:
    """Parse various date formats, return YYYY-MM-DD or None."""
    if not text or not text.strip():
        return None
    text = text.strip()
    for fmt in ("%Y-%m-%d", "%d %B %Y", "%B %d, %Y", "%d/%m/%Y", "%m/%d/%Y",
                "%Y-%m-%dT%H:%M:%S", "%d-%b-%Y"):
        try:
            return datetime.strptime(text, fmt).strftime("%Y-%m-%d")
        except ValueError:
            continue
    # Strip ordinal suffixes and try again
    cleaned = re.sub(r"(\d+)(st|nd|rd|th)", r"\1", text)
    for fmt in ("%d %B %Y", "%B %d, %Y"):
        try:
            return datetime.strptime(cleaned, fmt).strftime("%Y-%m-%d")
        except ValueError:
            continue
    return None


def _safe_int(val) -> int | None:
    """Convert to int or return None."""
    if val is None:
        return None
    try:
        return int(float(val))
    except (ValueError, TypeError):
        return None


def _safe_bool(val) -> bool | None:
    """Convert to bool or return None."""
    if val is None or val == "":
        return None
    if isinstance(val, bool):
        return val
    v = str(val).strip().lower()
    if v in ("1", "true", "yes"):
        return True
    if v in ("0", "false", "no"):
        return False
    return None


def download_csv(url: str) -> list[dict]:
    """Download a CSV file and return a list of row dicts."""
    logger.info("Downloading %s …", url)
    resp = requests.get(url, timeout=60)
    resp.raise_for_status()

    # Detect encoding
    content = resp.content.decode("utf-8", errors="replace")
    reader = csv.DictReader(io.StringIO(content))
    rows = list(reader)
    logger.info("Downloaded %d rows from %s", len(rows), url.split("/")[-1])
    return rows


def map_row_to_review(row: dict, iata: str) -> dict:
    """Map a CSV row to the reviews_raw table schema."""
    # The dataset has varying column names; normalise keys
    norm = {k.strip().lower().replace(" ", "_"): v for k, v in row.items()}

    review_date = (_parse_date(norm.get("date"))
                   or _parse_date(norm.get("date_published"))
                   or _parse_date(norm.get("review_date")))

    return {
        "airport_iata":     iata,
        "review_date":      review_date,
        "author":           (norm.get("author") or norm.get("author_name") or "").strip() or None,
        "author_country":   (norm.get("author_country") or norm.get("country") or "").strip() or None,
        "overall_rating":   _safe_int(norm.get("overall_rating") or norm.get("overall")),
        "score_queuing":    _safe_int(norm.get("queuing_rating") or norm.get("terminal_seating_rating")
                                      or norm.get("queuing")),
        "score_cleanliness": _safe_int(norm.get("terminal_cleanliness_rating")
                                       or norm.get("cleanliness") or norm.get("cleanliness_rating")),
        "score_staff":      _safe_int(norm.get("staff_service_rating") or norm.get("staff")
                                      or norm.get("staff_rating")),
        "score_food_bev":   _safe_int(norm.get("food_beverages_rating") or norm.get("food_bev")
                                      or norm.get("food_rating")),
        "score_wifi":       _safe_int(norm.get("wifi_connectivity_rating") or norm.get("wifi")
                                      or norm.get("wifi_rating")),
        "score_wayfinding": _safe_int(norm.get("wayfinding") or norm.get("wayfinding_rating")),
        "score_transport":  None,  # typically not in the historical dataset
        "recommended":      _safe_bool(norm.get("recommended")),
        "verified":         False,
        "trip_type":        (norm.get("type_traveller") or norm.get("traveller_type")
                             or norm.get("trip_type") or "").strip() or None,
        "review_title":     (norm.get("title") or norm.get("review_title")
                             or norm.get("header") or "").strip() or None,
        "review_text":      (norm.get("content") or norm.get("review")
                             or norm.get("review_text") or norm.get("text") or "").strip() or None,
        "source":           "skytrax",
        "source_url":       None,
        "processed":        False,
    }


def _lookup_airport_ids(conn, iata_codes: set[str]) -> dict[str, int]:
    """Look up airport IDs from the airports table by IATA code."""
    if not iata_codes:
        return {}
    with conn.cursor() as cur:
        cur.execute(
            "SELECT iata_code, id FROM airports WHERE iata_code = ANY(%s)",
            (list(iata_codes),),
        )
        return {row[0]: row[1] for row in cur.fetchall()}


def insert_reviews(conn, reviews: list[dict]):
    """Bulk insert reviews into reviews_raw."""
    if not reviews:
        return 0

    # Look up airport_id for each IATA code
    iata_codes = {r["airport_iata"] for r in reviews if r.get("airport_iata")}
    iata_to_id = _lookup_airport_ids(conn, iata_codes)

    columns = [
        "airport_id", "review_date", "author", "author_country",
        "overall_rating", "score_queuing", "score_cleanliness", "score_staff",
        "score_food_bev", "score_wifi", "score_wayfinding", "score_transport",
        "recommended", "verified", "trip_type", "review_title", "review_text",
        "source", "source_url", "processed",
    ]

    template = "(" + ", ".join(["%s"] * len(columns)) + ")"
    query = f"INSERT INTO reviews_raw ({', '.join(columns)}) VALUES {template}"

    inserted = 0
    errors = []
    with conn.cursor() as cur:
        for review in reviews:
            airport_id = iata_to_id.get(review.get("airport_iata"))
            if airport_id is None:
                logger.warning("No airport_id found for IATA code %s, skipping.",
                               review.get("airport_iata"))
                continue
            values = [airport_id] + [review.get(col) for col in columns[1:]]
            try:
                cur.execute("SAVEPOINT insert_row")
                cur.execute(query, values)
                cur.execute("RELEASE SAVEPOINT insert_row")
                inserted += cur.rowcount
            except Exception as exc:
                logger.warning("Failed to insert review: %s", exc)
                cur.execute("ROLLBACK TO SAVEPOINT insert_row")
                errors.append(str(exc))
                continue
    conn.commit()
    if errors:
        logger.warning("Encountered %d insert errors.", len(errors))
    return inserted


def run_import(db_url: str):
    """Main import routine."""
    # Connect to DB
    logger.info("Connecting to database…")
    try:
        conn = psycopg2.connect(db_url)
    except Exception as exc:
        logger.error("Database connection failed: %s", exc)
        sys.exit(1)

    total_downloaded = 0
    total_matched = 0
    total_inserted = 0

    for csv_url in CSV_URLS:
        try:
            rows = download_csv(csv_url)
        except Exception as exc:
            logger.error("Failed to download %s: %s", csv_url, exc)
            continue

        total_downloaded += len(rows)

        # Filter and map to our seed airports
        matched_reviews = []
        # Determine the airport name column
        if rows:
            sample_keys = {k.strip().lower().replace(" ", "_") for k in rows[0].keys()}
            name_col = None
            for candidate in ("airport_name", "airport", "name"):
                if candidate in sample_keys:
                    name_col = candidate
                    break
            if name_col is None:
                logger.warning("Could not find airport name column in %s. Columns: %s",
                               csv_url, list(rows[0].keys()))
                continue

        for row in rows:
            norm_keys = {k.strip().lower().replace(" ", "_"): k for k in row.keys()}
            raw_name = row.get(norm_keys.get(name_col, ""), "")
            iata = match_airport(raw_name)
            if iata and iata in SEED_IATA_CODES:
                review = map_row_to_review(row, iata)
                matched_reviews.append(review)

        total_matched += len(matched_reviews)
        logger.info("Matched %d reviews to seed airports from %s",
                     len(matched_reviews), csv_url.split("/")[-1])

        # Insert
        inserted = insert_reviews(conn, matched_reviews)
        total_inserted += inserted
        logger.info("Inserted %d reviews from %s", inserted, csv_url.split("/")[-1])

    conn.close()

    # Summary to stderr
    logger.info(
        "Import complete. Downloaded: %d, Matched: %d, Inserted: %d",
        total_downloaded, total_matched, total_inserted,
    )

    # Output summary JSON to stdout
    import json
    summary = {
        "status": "complete",
        "rows_downloaded": total_downloaded,
        "rows_matched": total_matched,
        "rows_inserted": total_inserted,
    }
    json.dump(summary, sys.stdout, indent=2)
    sys.stdout.write("\n")


def main():
    parser = argparse.ArgumentParser(
        description="Import historical Skytrax reviews from GitHub dataset."
    )
    parser.add_argument(
        "--db-url", default=None,
        help="Postgres connection string (default: DATABASE_URL env var)"
    )
    args = parser.parse_args()

    db_url = args.db_url or os.environ.get("DATABASE_URL")
    if not db_url:
        logger.error("No database URL provided. Use --db-url or set DATABASE_URL env var.")
        sys.exit(1)

    run_import(db_url)


if __name__ == "__main__":
    main()
