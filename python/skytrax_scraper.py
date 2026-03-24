#!/usr/bin/env python3
"""
Skytrax airport review scraper using Playwright.

Scrapes reviews from airlinequality.com and star ratings from skytraxratings.com.
Outputs JSON to stdout; all logs go to stderr.

Usage:
    python skytrax_scraper.py --airport LTN --since 2024-01-01
"""

import argparse
import asyncio
import json
import logging
import random
import re
import sys
from datetime import datetime
from pathlib import Path

from bs4 import BeautifulSoup
from playwright.async_api import async_playwright

# ---------------------------------------------------------------------------
# Logging – everything goes to stderr so stdout stays clean for JSON output
# ---------------------------------------------------------------------------
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(message)s",
    stream=sys.stderr,
)
logger = logging.getLogger(__name__)

# ---------------------------------------------------------------------------
# IATA -> Skytrax slug mapping loaded from airports.json
# ---------------------------------------------------------------------------

def _load_slugs() -> tuple[dict[str, str], dict[str, str]]:
    """Load Skytrax slugs from airports.json (fallback if --review-slug/--rating-slug not provided)."""
    airports_path = Path(__file__).parent.parent / "airports.json"
    try:
        with open(airports_path) as f:
            airports = json.load(f)
    except (FileNotFoundError, json.JSONDecodeError):
        return {}, {}
    review_slugs = {}
    rating_slugs = {}
    for a in airports:
        iata = a["iata"]
        if a.get("skytrax_review_slug"):
            review_slugs[iata] = a["skytrax_review_slug"]
        if a.get("skytrax_rating_slug"):
            rating_slugs[iata] = a["skytrax_rating_slug"]
    return review_slugs, rating_slugs

AIRPORT_SLUGS, RATING_SLUGS = _load_slugs()

# Sub-rating labels as they appear on the page -> output field names.
# Skytrax uses slightly different labels across versions of their site.
SUB_SCORE_MAP = {
    "Queuing Times":       "score_queuing",
    "Terminal Cleanliness": "score_cleanliness",
    "Staff Service":       "score_staff",
    "Airport Staff":       "score_staff",
    "Food & Beverages":    "score_food_bev",
    "Food Beverages":      "score_food_bev",
    "Wifi & Connectivity": "score_wifi",
    "Wifi Connectivity":   "score_wifi",
    "Airport Wayfinding":  "score_wayfinding",
    "Airport Signs":       "score_wayfinding",
    "Terminal Signs":      "score_wayfinding",
    "Transport Links":     "score_transport",
}


def _parse_date(text: str) -> datetime | None:
    """Try common Skytrax date formats."""
    text = text.strip()
    cleaned = re.sub(r"(\d+)(st|nd|rd|th)", r"\1", text)
    for fmt in ("%d %B %Y", "%B %d, %Y", "%Y-%m-%d"):
        try:
            return datetime.strptime(cleaned, fmt)
        except ValueError:
            pass
    # Fallback: try dateutil if available
    try:
        from dateutil.parser import parse as du_parse
        return du_parse(text)
    except Exception:
        return None


def _extract_star_count(cell) -> int | None:
    """Count filled stars in a rating cell.

    New format: <span class="star fill">1</span><span class="star">2</span>...
    Old format: <img src="...star-fill..."> images
    """
    if cell is None:
        return None
    # New format: span elements with class "fill"
    stars = cell.find_all("span", class_="star")
    if stars:
        filled = sum(1 for s in stars if "fill" in s.get("class", []))
        return filled if filled > 0 else None
    # Old format: img elements
    imgs = cell.find_all("img", src=True)
    if imgs:
        filled = sum(1 for s in imgs if "star-fill" in s.get("src", ""))
        return filled if filled > 0 else None
    return None


def _parse_review(article, page_url: str) -> dict:
    """Parse a single <article> block into a review dict."""
    # Extract unique review ID from article class (e.g. "review-939112")
    classes = article.get("class", [])
    review_id = next((c for c in classes if c.startswith("review-")), None)
    source_url = f"{page_url}#{review_id}" if review_id else page_url

    review: dict = {
        "review_date": None,
        "overall_rating": None,
        "score_queuing": None,
        "score_cleanliness": None,
        "score_staff": None,
        "score_food_bev": None,
        "score_wifi": None,
        "score_wayfinding": None,
        "score_transport": None,
        "recommended": None,
        "verified": False,
        "trip_type": None,
        "review_title": None,
        "review_text": None,
        "source_url": source_url,
    }

    # --- Review title ---
    title_tag = article.find("h2", class_="text_header") or article.find("h3", class_="text_sub_header")
    if title_tag:
        title_text = title_tag.get_text(strip=True)
        # Strip leading quotes
        review["review_title"] = title_text.strip('"').strip('\u201c').strip('\u201d').strip()

    # --- Review text ---
    text_content = article.find("div", class_="text_content")
    if text_content:
        full_text = text_content.get_text(separator=" ", strip=True)
        # Remove the "Not Verified |" or "Trip Verified |" prefix
        full_text = re.sub(r"^(\u2714\s*)?(Not Verified|Trip Verified)\s*\|\s*", "", full_text)
        review["review_text"] = full_text

    # --- Verified ---
    if text_content:
        verified_marker = text_content.get_text()
        if "Trip Verified" in verified_marker or "\u2714" in verified_marker:
            review["verified"] = True

    # --- Date ---
    date_tag = article.find("time", itemprop="datePublished") or article.find("meta", itemprop="datePublished")
    if date_tag:
        date_str = date_tag.get("datetime") or date_tag.get("content") or date_tag.get_text(strip=True)
        parsed = _parse_date(date_str)
        if parsed:
            review["review_date"] = parsed.strftime("%Y-%m-%d")

    # Fallback: date from header text like "John Smith (United Kingdom) 15th June 2024"
    if not review["review_date"]:
        header = article.find("h3", class_="text_sub_header")
        if header:
            header_text = header.get_text(strip=True)
            # Try to parse a date from the header text
            parsed_fallback = _parse_date(header_text)
            if parsed_fallback:
                review["review_date"] = parsed_fallback.strftime("%Y-%m-%d")

    # --- Overall rating (1-10) ---
    rating_tag = article.find("span", itemprop="ratingValue")
    if rating_tag:
        try:
            review["overall_rating"] = int(rating_tag.get_text(strip=True))
        except ValueError:
            pass
    # Alternative: look for the big rating circle
    if review["overall_rating"] is None:
        rating_div = article.find("div", class_="rating-10")
        if rating_div:
            inner = rating_div.find("span")
            if inner:
                try:
                    review["overall_rating"] = int(inner.get_text(strip=True))
                except ValueError:
                    pass

    # --- Sub-scores (star ratings in table rows) ---
    rows = article.find_all("tr")
    for row in rows:
        header_cell = row.find("td", class_="review-rating-header")
        value_cell = row.find("td", class_="review-rating-stars")
        if header_cell and value_cell:
            label = header_cell.get_text(strip=True)
            field = SUB_SCORE_MAP.get(label)
            if field:
                review[field] = _extract_star_count(value_cell)

    # --- Recommended ---
    for row in rows:
        header_cell = row.find("td", class_="review-rating-header")
        value_cell = row.find("td", class_="review-value")
        if header_cell and value_cell:
            label = header_cell.get_text(strip=True)
            if "Recommended" in label:
                val = value_cell.get_text(strip=True).lower()
                review["recommended"] = val == "yes"

    # --- Trip type ---
    for row in rows:
        header_cell = row.find("td", class_="review-rating-header")
        value_cell = row.find("td", class_="review-value")
        if header_cell and value_cell:
            label = header_cell.get_text(strip=True)
            if "Type Of Traveller" in label or "Trip Type" in label:
                review["trip_type"] = value_cell.get_text(strip=True)

    return review


async def scrape_star_rating(page, iata: str) -> int | None:
    """Scrape the overall star rating from skytraxratings.com.

    The most reliable method: parse the page title which contains
    e.g. "London Luton Airport is a 3-Star Regional Airport".
    """
    rating_slug = RATING_SLUGS.get(iata)
    if not rating_slug:
        logger.warning("No rating slug for %s", iata)
        return None

    # Try multiple URL patterns — some airports use -rating, others -quality-rating
    urls = [
        f"https://skytraxratings.com/airports/{rating_slug}-rating",
        f"https://skytraxratings.com/airports/{rating_slug}-quality-rating",
        f"https://skytraxratings.com/airports/{rating_slug}",
    ]
    resp = None
    for url in urls:
        logger.info("Trying star rating URL: %s", url)
        try:
            resp = await page.goto(url, timeout=30000)
            await page.wait_for_load_state("domcontentloaded")
            if resp and resp.status != 404:
                break
        except Exception:
            continue

    if not resp or resp.status == 404:
        logger.warning("Rating page not found for %s (tried %d URLs)", iata, len(urls))
        return None

    try:
        title = await page.title()
        # Title format: "London Luton Airport is a 3-Star Regional Airport | Skytrax"
        match = re.search(r"(\d)\s*-?\s*[Ss]tar", title)
        if match:
            stars = int(match.group(1))
            logger.info("Extracted %d stars from page title for %s", stars, iata)
            return stars

        # Fallback: check page text for "certified as a N-Star Airport"
        content = await page.content()
        text_match = re.search(r"certified as a\s+(\d)\s*-?\s*[Ss]tar", content)
        if text_match:
            stars = int(text_match.group(1))
            logger.info("Extracted %d stars from page text for %s", stars, iata)
            return stars
    except Exception as exc:
        logger.warning("Could not fetch star rating for %s: %s", iata, exc)

    return None


async def scrape_reviews(airport: str, since: datetime, max_pages: int = 50) -> dict:
    """Main scraping routine. Returns the full result dict."""
    iata = airport.upper()
    slug = AIRPORT_SLUGS.get(iata)
    if slug is None:
        logger.warning("No Skytrax review slug for %s, skipping reviews", iata)
        return {"airport": iata, "star_rating": None, "reviews": []}

    result = {
        "airport": iata,
        "star_rating": None,
        "reviews": [],
    }

    async with async_playwright() as pw:
        browser = await pw.chromium.launch(headless=True)
        context = await browser.new_context(
            user_agent=(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) "
                "AppleWebKit/537.36 (KHTML, like Gecko) "
                "Chrome/120.0.0.0 Safari/537.36"
            )
        )
        page = await context.new_page()

        # --- Star rating ---
        result["star_rating"] = await scrape_star_rating(page, iata)

        # --- Reviews pagination ---
        page_num = 1
        stop = False
        while not stop and page_num <= max_pages:
            url = (
                f"https://www.airlinequality.com/airport-reviews/{slug}"
                f"/page/{page_num}/"
            )
            logger.info("Fetching reviews page %d: %s", page_num, url)
            try:
                await page.goto(url, timeout=30000)
                await page.wait_for_load_state("domcontentloaded")
            except Exception as exc:
                logger.warning("Failed to load page %d: %s", page_num, exc)
                break

            content = await page.content()
            soup = BeautifulSoup(content, "html.parser")
            articles = soup.find_all("article", itemprop="review")

            if not articles:
                logger.info("No review articles found on page %d, stopping.", page_num)
                break

            logger.info("Found %d reviews on page %d", len(articles), page_num)

            for article in articles:
                review = _parse_review(article, url)
                # Check date cutoff
                if review["review_date"]:
                    review_dt = datetime.strptime(review["review_date"], "%Y-%m-%d")
                    if review_dt < since:
                        logger.info("Review date %s is before --since %s, stopping.",
                                    review["review_date"], since.strftime("%Y-%m-%d"))
                        stop = True
                        break
                result["reviews"].append(review)

            page_num += 1
            # Polite delay: 2-3 seconds between page loads
            delay = 2 + random.random()
            logger.info("Waiting %.1fs before next page…", delay)
            await asyncio.sleep(delay)

        await browser.close()

    logger.info("Scraped %d reviews for %s", len(result["reviews"]), iata)
    return result


def main():
    parser = argparse.ArgumentParser(
        description="Scrape Skytrax airport reviews and ratings."
    )
    parser.add_argument(
        "--airport", required=True,
        help="IATA airport code (e.g. LTN, LHR)"
    )
    parser.add_argument(
        "--since", required=True,
        help="Only include reviews on or after this date (YYYY-MM-DD)"
    )
    parser.add_argument(
        "--max-pages", type=int, default=50,
        help="Maximum number of review pages to scrape (default: 50, ~500 reviews at 10/page)"
    )
    parser.add_argument(
        "--review-slug", required=False,
        help="Skytrax review slug (e.g. london-heathrow-airport). Overrides airports.json lookup."
    )
    parser.add_argument(
        "--rating-slug", required=False,
        help="Skytrax rating slug (e.g. london-heathrow-airport). Overrides airports.json lookup."
    )
    args = parser.parse_args()

    # Override global slug dicts if CLI args provided
    iata = args.airport.upper()
    if args.review_slug:
        AIRPORT_SLUGS[iata] = args.review_slug
    if args.rating_slug:
        RATING_SLUGS[iata] = args.rating_slug

    try:
        since_date = datetime.strptime(args.since, "%Y-%m-%d")
    except ValueError:
        logger.error("Invalid --since date format. Use YYYY-MM-DD.")
        sys.exit(1)

    result = asyncio.run(scrape_reviews(args.airport, since_date, args.max_pages))
    json.dump(result, sys.stdout, indent=2, ensure_ascii=False)
    sys.stdout.write("\n")


if __name__ == "__main__":
    main()
