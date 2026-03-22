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
# IATA -> Skytrax slug mapping for the 15 seed airports
# ---------------------------------------------------------------------------
# Slugs for airlinequality.com review pages
AIRPORT_SLUGS = {
    "LHR": "london-heathrow-airport",
    "LGW": "london-gatwick-airport",
    "LTN": "luton-airport",
    "OPO": "porto-airport",
    "MAD": "madrid-barajas-airport",
    "BCN": "barcelona-airport",
    "BER": "berlin-brandenburg-airport",
    "MUC": "munich-airport",
    "CDG": "paris-cdg-airport",
    "NCE": "nice-cote-dazur-airport",
    "AMS": "amsterdam-schiphol-airport",
    "CPH": "copenhagen-airport",
    "FCO": "rome-fiumicino-airport",
    "WAW": "warsaw-chopin-airport",
    "BUD": "budapest-ferihegy-airport",
}

# Slugs for skytraxratings.com star rating pages (different from review slugs)
RATING_SLUGS = {
    "LHR": "london-heathrow-airport",
    "LGW": "london-gatwick-airport",
    "LTN": "london-luton-airport",
    "OPO": "porto-airport",
    "MAD": "madrid-barajas-airport",
    "BCN": "barcelona-el-prat-airport",
    "BER": "berlin-brandenburg-airport",
    "MUC": "munich-airport",
    "CDG": "paris-charles-de-gaulle-airport",
    "NCE": "nice-cote-d-azur-airport",
    "AMS": "amsterdam-schiphol-airport",
    "CPH": "copenhagen-airport",
    "FCO": "rome-fiumicino-airport",
    "WAW": "warsaw-chopin-airport",
    "BUD": "budapest-airport",
}

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
        "author": None,
        "author_country": None,
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

    # --- Author and date ---
    # Author is in <span itemprop="name"> or similar
    author_tag = article.find("span", itemprop="name")
    if author_tag:
        review["author"] = author_tag.get_text(strip=True)

    date_tag = article.find("time", itemprop="datePublished") or article.find("meta", itemprop="datePublished")
    if date_tag:
        date_str = date_tag.get("datetime") or date_tag.get("content") or date_tag.get_text(strip=True)
        parsed = _parse_date(date_str)
        if parsed:
            review["review_date"] = parsed.strftime("%Y-%m-%d")

    # Fallback: date often in a <h3> or <h2> like "John Smith (United Kingdom) 15th June 2024"
    if not review["review_date"] or not review["author"]:
        header = article.find("h3", class_="text_sub_header")
        if header:
            header_text = header.get_text(strip=True)
            # Try to extract country in parentheses
            country_match = re.search(r"\(([^)]+)\)", header_text)
            if country_match:
                review["author_country"] = country_match.group(1).strip()

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
    """Scrape the overall star rating from skytraxratings.com."""
    rating_slug = RATING_SLUGS.get(iata)
    if not rating_slug:
        logger.warning("No rating slug for %s", iata)
        return None

    # Try the direct rating page URL
    url = f"https://skytraxratings.com/airports/{rating_slug}-rating"
    logger.info("Fetching star rating from %s", url)
    try:
        await page.goto(url, timeout=30000)
        await page.wait_for_load_state("domcontentloaded")
        content = await page.content()
        soup = BeautifulSoup(content, "html.parser")

        # New format: star spans in the rating header area
        # Look for filled star SVGs/spans in the page
        star_containers = soup.find_all("div", class_=re.compile(r"star|rating"))
        for container in star_containers:
            filled_stars = container.find_all(
                lambda tag: tag.name in ("span", "div")
                and "star" in " ".join(tag.get("class", []))
            )
            # Count elements that look "filled" vs "empty"
            filled = 0
            for star in filled_stars:
                classes = " ".join(star.get("class", []))
                # Filled stars have specific classes/styles
                if "empty" not in classes and "grey" not in classes and "unfilled" not in classes:
                    filled += 1
            if filled > 0 and filled <= 5:
                return filled

        # Fallback: look for text like "3-Star Airport" or "3 Star"
        text = soup.get_text()
        match = re.search(r"(\d)\s*-?\s*[Ss]tar", text)
        if match:
            return int(match.group(1))

    except Exception as exc:
        logger.warning("Could not fetch star rating from %s: %s", url, exc)

    # Fallback: try the search page which shows stars in results
    try:
        search_url = f"https://skytraxratings.com/airports?s={rating_slug.replace('-airport', '').replace('-', '+')}"
        logger.info("Trying search fallback: %s", search_url)
        await page.goto(search_url, timeout=30000)
        await page.wait_for_load_state("domcontentloaded")
        content = await page.content()
        soup = BeautifulSoup(content, "html.parser")

        # Search results show stars as filled/empty spans
        first_result = soup.find("a", href=re.compile(rating_slug))
        if first_result:
            stars = first_result.find_all(
                lambda tag: tag.name in ("span", "div")
                and "star" in " ".join(tag.get("class", []))
            )
            if stars:
                # Stars without "empty" class are filled
                filled = sum(1 for s in stars
                    if "empty" not in " ".join(s.get("class", []))
                    and "grey" not in " ".join(s.get("class", [])))
                if 0 < filled <= 5:
                    return filled
    except Exception as exc:
        logger.warning("Search fallback failed: %s", exc)

    return None


async def scrape_reviews(airport: str, since: datetime, max_pages: int = 50) -> dict:
    """Main scraping routine. Returns the full result dict."""
    iata = airport.upper()
    slug = AIRPORT_SLUGS.get(iata)
    if slug is None:
        logger.error("Unknown airport IATA code: %s. Supported: %s",
                      iata, ", ".join(sorted(AIRPORT_SLUGS)))
        sys.exit(1)

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
    args = parser.parse_args()

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
