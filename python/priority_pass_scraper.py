#!/usr/bin/env python3
"""
Priority Pass lounge scraper.

Fetches lounge data for a given airport IATA code from the Priority Pass website.
Outputs a JSON array to stdout; all logs go to stderr.

Usage:
    python priority_pass_scraper.py --airport LHR
"""

import argparse
import json
import logging
import sys
import time
from pathlib import Path

import requests
from bs4 import BeautifulSoup

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
# Country code -> Priority Pass URL slug mapping
# ---------------------------------------------------------------------------
COUNTRY_SLUG_MAP: dict[str, str] = {
    "GB": "united-kingdom",
    "PT": "portugal",
    "ES": "spain",
    "DE": "germany",
    "FR": "france",
    "NL": "netherlands",
    "DK": "denmark",
    "IT": "italy",
    "PL": "poland",
    "HU": "hungary",
    "RO": "romania",
}

BASE_URL = "https://www.prioritypass.com"
HEADERS = {
    "User-Agent": "AirportIntelligencePlatform/1.0 (research project)",
    "Accept": "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
    "Accept-Language": "en-GB,en;q=0.5",
}


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def load_country_code(iata: str) -> str | None:
    """Load the country code for an airport from airports.json."""
    airports_path = Path(__file__).parent.parent / "airports.json"
    try:
        with open(airports_path) as f:
            airports = json.load(f)
    except (FileNotFoundError, json.JSONDecodeError) as exc:
        logger.error("Failed to load airports.json: %s", exc)
        return None

    iata_upper = iata.upper()
    for airport in airports:
        if airport.get("iata", "").upper() == iata_upper:
            return airport.get("country")

    logger.error("IATA code %s not found in airports.json", iata)
    return None


def fetch_page(session: requests.Session, url: str) -> BeautifulSoup | None:
    """Fetch a URL and return a BeautifulSoup object, or None on error."""
    try:
        resp = session.get(url, headers=HEADERS, timeout=30)
        resp.raise_for_status()
        return BeautifulSoup(resp.text, "html.parser")
    except requests.RequestException as exc:
        logger.error("Failed to fetch %s: %s", url, exc)
        return None


def find_lounge_links(soup: BeautifulSoup, iata: str, base_url: str) -> list[str]:
    """Find lounge detail page links for a given IATA code.

    Priority Pass country pages list cities, and city pages list lounges.
    The individual lounge URL slugs contain the IATA code (e.g. bcn1-sala-vip),
    but the city-level URLs use city names (e.g. /lounges/spain/barcelona/).

    Strategy: collect ALL links under /lounges/ from the country page (these are
    city-level links), then fetch each city page and look for lounge links whose
    slug starts with the IATA code.
    """
    iata_lower = iata.lower()

    # Step 1: Collect city-level links from the country page
    city_links: list[str] = []
    for anchor in soup.find_all("a", href=True):
        href: str = anchor["href"]
        # City links look like /en-GB/lounges/spain/barcelona
        # Skip links that already point to a specific lounge (have 5+ path segments)
        if "/lounges/" not in href:
            continue
        if href.startswith("http"):
            full_url = href
        elif href.startswith("/"):
            full_url = base_url + href
        else:
            continue
        # Count path segments to distinguish city pages from lounge pages
        path = full_url.split("/lounges/")[-1].rstrip("/")
        segments = [s for s in path.split("/") if s]
        # City pages have 2 segments: country/city. Lounge pages have 3: country/city/lounge
        if len(segments) == 2 and full_url not in city_links:
            city_links.append(full_url)

    logger.info("Found %d city link(s) on country page", len(city_links))

    # Step 2: Also check for direct lounge links on the country page
    # (some country pages list lounges directly)
    lounge_links: list[str] = []
    for anchor in soup.find_all("a", href=True):
        href: str = anchor["href"]
        if href.startswith("http"):
            full_url = href
        elif href.startswith("/"):
            full_url = base_url + href
        else:
            continue
        # Lounge slugs start with the IATA code, e.g. /bcn1-sala-vip
        slug = full_url.rstrip("/").rsplit("/", 1)[-1].lower()
        if slug.startswith(iata_lower) and full_url not in lounge_links:
            lounge_links.append(full_url)

    if lounge_links:
        return lounge_links

    # Step 3: Fetch each city page and look for lounge links with IATA in slug
    session = requests.Session()
    for city_url in city_links:
        time.sleep(0.5)
        city_soup = fetch_page(session, city_url)
        if city_soup is None:
            continue
        for anchor in city_soup.find_all("a", href=True):
            href = anchor["href"]
            if href.startswith("http"):
                full_url = href
            elif href.startswith("/"):
                full_url = base_url + href
            else:
                continue
            slug = full_url.rstrip("/").rsplit("/", 1)[-1].lower()
            if slug.startswith(iata_lower) and full_url not in lounge_links:
                lounge_links.append(full_url)

    return lounge_links


def extract_lounge_data(soup: BeautifulSoup, url: str) -> dict | None:
    """Extract lounge details from a lounge detail page.

    Priority Pass uses Next.js with heavy client-side rendering, so most page
    content is in RSC payloads not visible to a plain HTML parser.  We extract
    what we reliably can: the lounge name (h1) and the terminal number from
    the <title> tag (which IS server-rendered for SEO).
    """
    import re

    # Name from h1
    h1 = soup.find("h1")
    if not h1:
        logger.warning("No h1 found on %s", url)
        return None
    name = h1.get_text(strip=True)

    # Terminal from <title> — e.g. "Sala VIP Pau Casals BCN AIRPORT Terminal 1 ..."
    terminal: str | None = None
    title_tag = soup.find("title")
    if title_tag:
        title_text = title_tag.get_text(strip=True)
        m = re.search(r"Terminal\s+(\S+)", title_text, re.IGNORECASE)
        if m:
            terminal = m.group(1)  # e.g. "1", "2"

    # Opening hours and amenities are in RSC payloads / JS-rendered content,
    # not reliably extractable with plain HTML parsing. Store as null.

    return {
        "name": name,
        "terminal": terminal,
        "opening_hours": None,
        "amenities": [],
        "url": url,
    }


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main() -> None:
    parser = argparse.ArgumentParser(description="Priority Pass lounge scraper")
    parser.add_argument("--airport", required=True, help="IATA airport code (e.g. LHR)")
    args = parser.parse_args()

    iata = args.airport.upper()

    # Look up country code
    country_code = load_country_code(iata)
    if not country_code:
        print(f"ERROR: Could not determine country code for {iata}", file=sys.stderr)
        sys.exit(1)

    # Map to Priority Pass URL slug
    country_slug = COUNTRY_SLUG_MAP.get(country_code)
    if not country_slug:
        print(
            f"ERROR: Country code {country_code!r} is not mapped to a Priority Pass slug",
            file=sys.stderr,
        )
        sys.exit(1)

    country_url = f"{BASE_URL}/en-GB/lounges/{country_slug}"
    logger.info("Fetching Priority Pass country page for %s (%s): %s", iata, country_code, country_url)

    session = requests.Session()

    # Fetch country listing page
    soup = fetch_page(session, country_url)
    if soup is None:
        print(f"ERROR: Failed to fetch country page {country_url}", file=sys.stderr)
        sys.exit(1)

    # Find lounge detail links matching the IATA code
    lounge_links = find_lounge_links(soup, iata, BASE_URL)
    logger.info("Found %d lounge link(s) for %s", len(lounge_links), iata)

    if not lounge_links:
        print(f"ERROR: No lounge links found for {iata} on {country_url}", file=sys.stderr)
        sys.exit(1)

    # Fetch each lounge detail page
    lounges: list[dict] = []
    for url in lounge_links:
        logger.info("Fetching lounge page: %s", url)
        time.sleep(1)  # polite crawling

        detail_soup = fetch_page(session, url)
        if detail_soup is None:
            logger.warning("Skipping lounge at %s (fetch failed)", url)
            continue

        lounge_data = extract_lounge_data(detail_soup, url)
        if lounge_data is None:
            logger.warning("Skipping lounge at %s (parse failed)", url)
            continue

        lounges.append(lounge_data)
        logger.info("Extracted lounge: %s", lounge_data["name"])

    if not lounges:
        print(f"ERROR: Could not extract data for any lounges at {iata}", file=sys.stderr)
        sys.exit(1)

    json.dump(lounges, sys.stdout, indent=2, ensure_ascii=False)
    print()  # trailing newline


if __name__ == "__main__":
    main()
