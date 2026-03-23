#!/usr/bin/env python3
"""
carbon_accreditation.py
Look up ACI Airport Carbon Accreditation level for a given IATA code.

Usage:
    python carbon_accreditation.py --airport LHR

Outputs JSON to stdout on success:
    {"level_name": "Transition", "report_year": 2024}

Exits with code 1 and a message on stderr if the airport is not found.
"""

import argparse
import json
import sys
from pathlib import Path


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Look up ACI Carbon Accreditation level for an airport."
    )
    parser.add_argument("--airport", required=True, help="IATA airport code (e.g. LHR)")
    args = parser.parse_args()

    iata = args.airport.strip().upper()

    # Resolve data file relative to this script's location (project root / data /)
    script_dir = Path(__file__).resolve().parent
    data_file = script_dir.parent / "data" / "carbon_accreditation.json"

    if not data_file.exists():
        print(
            f"ERROR: Data file not found: {data_file}",
            file=sys.stderr,
        )
        sys.exit(1)

    with data_file.open("r", encoding="utf-8") as fh:
        data: dict = json.load(fh)

    if iata not in data:
        print(
            f"ERROR: Airport {iata!r} not found in carbon accreditation data",
            file=sys.stderr,
        )
        sys.exit(1)

    print(json.dumps(data[iata]))


if __name__ == "__main__":
    main()
