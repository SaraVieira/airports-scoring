#!/usr/bin/env python3
"""
Sentiment analysis pipeline for airport reviews.

Uses RoBERTa GoEmotions for emotion classification and a cross-encoder NLI
model for zero-shot topic classification. Reads unprocessed reviews from
Postgres, writes sentiment snapshots as JSON to stdout.

Usage:
    python sentiment_pipeline.py --airport LTN
    python sentiment_pipeline.py --airport LTN --db-url postgres://user:pass@host/db
"""

import argparse
import json
import logging
import math
import os
import random
import sys
from collections import defaultdict
from datetime import datetime

import psycopg2
import psycopg2.extras
import torch
from transformers import (
    AutoModelForSequenceClassification,
    AutoTokenizer,
    pipeline,
)

# ---------------------------------------------------------------------------
# Logging – everything to stderr; stdout is reserved for JSON
# ---------------------------------------------------------------------------
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(message)s",
    stream=sys.stderr,
)
logger = logging.getLogger(__name__)

# ---------------------------------------------------------------------------
# Emotion -> sentiment mapping
# ---------------------------------------------------------------------------
POSITIVE_EMOTIONS = {
    "joy", "admiration", "amusement", "approval", "caring", "desire",
    "excitement", "gratitude", "love", "optimism", "pride", "relief",
}
NEGATIVE_EMOTIONS = {
    "anger", "annoyance", "disappointment", "disapproval", "disgust",
    "embarrassment", "fear", "grief", "nervousness", "remorse", "sadness",
}
NEUTRAL_EMOTIONS = {
    "neutral", "confusion", "curiosity", "realization", "surprise",
}

# ---------------------------------------------------------------------------
# Zero-shot topic labels
# ---------------------------------------------------------------------------
TOPIC_LABELS = [
    "queuing & security",
    "staff & service",
    "cleanliness",
    "food & beverage",
    "wayfinding & signage",
    "transport links",
]

# Map topic labels to output field names
TOPIC_FIELD_MAP = {
    "queuing & security":   "score_queuing",
    "staff & service":      "score_staff",
    "cleanliness":          "score_cleanliness",
    "food & beverage":      "score_food_bev",
    "wayfinding & signage": "score_wayfinding",
    "transport links":      "score_transport",
}

# ---------------------------------------------------------------------------
# Snarky commentary templates
# ---------------------------------------------------------------------------
COMMENTARY_TEMPLATES = {
    # High anger + queuing
    "angry_queuing": [
        "Passengers would rather queue at the DMV than at {airport}'s security. At least the DMV has chairs.",
        "{airport} security lines: where dreams of catching your flight go to die.",
        "If {airport} charged by the minute spent queuing, they'd outperform most airlines.",
    ],
    # High anger + staff
    "angry_staff": [
        "{airport} staff have perfected the art of looking busy while doing absolutely nothing.",
        "Customer service at {airport}? More like customer survival.",
    ],
    # High anger + cleanliness
    "angry_cleanliness": [
        "{airport}: where the floors are as sticky as the situation.",
        "The cleanest thing at {airport} is the passengers' disappointment.",
    ],
    # Generally negative
    "general_negative": [
        "{airport} makes a strong case for teleportation research funding.",
        "{airport}: the airport equivalent of a participation trophy.",
        "If airports had a Yelp, {airport} would be fighting for two stars.",
    ],
    # Improving sentiment
    "improving": [
        "{airport}: proof that rock bottom is a solid foundation.",
        "{airport} is improving — like a student who discovered studying exists.",
        "Things are looking up at {airport}. Admittedly, the bar was underground.",
    ],
    # Great scores
    "great": [
        "{airport} is so good it makes other airports look like bus stations.",
        "{airport}: where even the delays feel civilized.",
        "Passengers at {airport} are suspiciously happy. Someone check the water.",
    ],
    # Neutral/mixed
    "neutral_mixed": [
        "{airport}: aggressively adequate. Not great, not terrible — the airport equivalent of a shrug.",
        "{airport} is the human trafficking-free airport equivalent of 'it's fine.'",
        "{airport}: solidly in the 'could be worse' category.",
    ],
}


def _pick_commentary(airport: str, positive_pct: float, negative_pct: float,
                     top_negative_topic: str | None,
                     prev_positive_pct: float | None) -> str:
    """Pick a snarky commentary template based on sentiment patterns."""
    # Check if sentiment is improving
    if prev_positive_pct is not None and positive_pct > prev_positive_pct + 10:
        category = "improving"
    elif positive_pct >= 65:
        category = "great"
    elif negative_pct >= 45:
        # High negativity — pick topic-specific snark if available
        if top_negative_topic == "queuing & security":
            category = "angry_queuing"
        elif top_negative_topic == "staff & service":
            category = "angry_staff"
        elif top_negative_topic == "cleanliness":
            category = "angry_cleanliness"
        else:
            category = "general_negative"
    elif negative_pct >= 30:
        category = "general_negative"
    else:
        category = "neutral_mixed"

    templates = COMMENTARY_TEMPLATES[category]
    template = random.choice(templates)
    return template.format(airport=airport)


# ---------------------------------------------------------------------------
# Model loading helpers
# ---------------------------------------------------------------------------

def load_emotion_pipeline():
    """Load the GoEmotions RoBERTa pipeline."""
    logger.info("Loading emotion classification model…")
    emotion_pipe = pipeline(
        "text-classification",
        model="SamLowe/roberta-base-go_emotions",
        top_k=None,
        device=0 if torch.cuda.is_available() else -1,
        truncation=True,
    )
    logger.info("Emotion model loaded.")
    return emotion_pipe


def load_nli_model():
    """Load the cross-encoder NLI model for zero-shot topic classification."""
    logger.info("Loading NLI cross-encoder model…")
    from transformers import AutoModelForSequenceClassification, AutoTokenizer
    model_name = "cross-encoder/nli-distilroberta-base"
    tokenizer = AutoTokenizer.from_pretrained(model_name)
    model = AutoModelForSequenceClassification.from_pretrained(model_name)
    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    model.to(device)
    model.eval()
    logger.info("NLI model loaded on %s.", device)
    return tokenizer, model, device


def classify_topics(text: str, tokenizer, model, device, labels: list[str]) -> dict[str, float]:
    """Score a review text against each topic label using NLI entailment."""
    scores = {}
    for label in labels:
        premise = text[:512]  # truncate long reviews
        hypothesis = f"This review is about {label}."
        inputs = tokenizer(
            premise, hypothesis,
            return_tensors="pt",
            truncation=True,
            max_length=512,
        ).to(device)
        with torch.no_grad():
            logits = model(**inputs).logits
        # NLI labels: 0=contradiction, 1=neutral, 2=entailment
        probs = torch.softmax(logits, dim=-1)[0]
        entailment_score = probs[2].item() if probs.shape[0] > 2 else probs[-1].item()
        scores[label] = entailment_score
    return scores


def classify_sentiment(emotions: list[dict]) -> str:
    """Map GoEmotions output to positive/negative/neutral."""
    if not emotions:
        return "neutral"
    # Sum scores for each sentiment bucket
    pos_score = sum(e["score"] for e in emotions if e["label"] in POSITIVE_EMOTIONS)
    neg_score = sum(e["score"] for e in emotions if e["label"] in NEGATIVE_EMOTIONS)
    neu_score = sum(e["score"] for e in emotions if e["label"] in NEUTRAL_EMOTIONS)
    if pos_score >= neg_score and pos_score >= neu_score:
        return "positive"
    elif neg_score >= pos_score and neg_score >= neu_score:
        return "negative"
    return "neutral"


# ---------------------------------------------------------------------------
# Database helpers
# ---------------------------------------------------------------------------

def fetch_unprocessed_reviews(conn, airport: str) -> list[dict]:
    """Fetch unprocessed reviews for an airport from reviews_raw."""
    with conn.cursor(cursor_factory=psycopg2.extras.RealDictCursor) as cur:
        cur.execute(
            """
            SELECT r.id, r.review_date, r.overall_rating, r.review_text,
                   r.score_queuing, r.score_cleanliness, r.score_staff,
                   r.score_food_bev, r.score_wayfinding, r.score_transport
            FROM reviews_raw r
            JOIN airports a ON r.airport_id = a.id
            WHERE a.iata_code = %s
              AND (r.processed IS NULL OR r.processed = false)
            ORDER BY r.review_date ASC
            """,
            (airport.upper(),),
        )
        rows = cur.fetchall()
    logger.info("Fetched %d unprocessed reviews for %s", len(rows), airport)
    return [dict(r) for r in rows]


def mark_reviews_processed(conn, review_ids: list[int]):
    """Mark reviews as processed in reviews_raw."""
    if not review_ids:
        return
    with conn.cursor() as cur:
        cur.execute(
            "UPDATE reviews_raw SET processed = true WHERE id = ANY(%s)",
            (review_ids,),
        )
    conn.commit()
    logger.info("Marked %d reviews as processed.", len(review_ids))


# ---------------------------------------------------------------------------
# Aggregation
# ---------------------------------------------------------------------------

def _quarter(dt) -> tuple[int, int]:
    """Return (year, quarter) for a date."""
    if isinstance(dt, str):
        dt = datetime.strptime(dt, "%Y-%m-%d")
    return dt.year, math.ceil(dt.month / 3)


def aggregate_snapshots(reviews_with_analysis: list[dict], airport: str) -> list[dict]:
    """Aggregate per-review analysis into quarterly snapshots."""
    buckets: dict[tuple[int, int], list[dict]] = defaultdict(list)
    for r in reviews_with_analysis:
        if r.get("review_date"):
            key = _quarter(r["review_date"])
            buckets[key].append(r)

    snapshots = []
    sorted_keys = sorted(buckets.keys())
    prev_positive_pct = None

    for (year, quarter) in sorted_keys:
        items = buckets[(year, quarter)]
        n = len(items)

        # Average overall rating
        ratings = [r["overall_rating"] for r in items if r.get("overall_rating") is not None]
        avg_rating = round(sum(ratings) / len(ratings), 2) if ratings else None

        # Sentiment percentages
        sentiments = [r["sentiment"] for r in items]
        pos_count = sentiments.count("positive")
        neg_count = sentiments.count("negative")
        neu_count = sentiments.count("neutral")
        positive_pct = round(100.0 * pos_count / n, 1)
        negative_pct = round(100.0 * neg_count / n, 1)
        neutral_pct = round(100.0 * neu_count / n, 1)

        # Average topic scores (entailment probabilities -> 1-5 scale)
        topic_avgs = {}
        for topic in TOPIC_LABELS:
            field = TOPIC_FIELD_MAP[topic]
            scores = [r["topics"].get(topic, 0) for r in items if "topics" in r]
            if scores:
                # Convert 0-1 entailment score to 1-5 scale
                raw_avg = sum(scores) / len(scores)
                topic_avgs[field] = round(1 + 4 * raw_avg, 1)
            else:
                topic_avgs[field] = None

        # Override with explicit sub-scores from reviews if available
        for score_field in ("score_queuing", "score_cleanliness", "score_staff",
                            "score_food_bev", "score_wayfinding", "score_transport"):
            explicit = [r.get(score_field) for r in items if r.get(score_field) is not None]
            if explicit:
                topic_avgs[score_field] = round(sum(explicit) / len(explicit), 1)

        # Find dominant negative topic for commentary
        negative_items = [r for r in items if r.get("sentiment") == "negative"]
        top_negative_topic = None
        if negative_items:
            topic_sums: dict[str, float] = defaultdict(float)
            for r in negative_items:
                for topic, score in r.get("topics", {}).items():
                    topic_sums[topic] += score
            if topic_sums:
                top_negative_topic = max(topic_sums, key=topic_sums.get)

        # Generate snarky commentary
        commentary = _pick_commentary(
            airport, positive_pct, negative_pct,
            top_negative_topic, prev_positive_pct,
        )
        prev_positive_pct = positive_pct

        snapshot = {
            "snapshot_year": year,
            "snapshot_quarter": quarter,
            "avg_rating": avg_rating,
            "review_count": n,
            "positive_pct": positive_pct,
            "negative_pct": negative_pct,
            "neutral_pct": neutral_pct,
            "score_queuing": topic_avgs.get("score_queuing"),
            "score_cleanliness": topic_avgs.get("score_cleanliness"),
            "score_staff": topic_avgs.get("score_staff"),
            "score_food_bev": topic_avgs.get("score_food_bev"),
            "score_wifi": topic_avgs.get("score_wifi"),
            "score_wayfinding": topic_avgs.get("score_wayfinding"),
            "score_transport": topic_avgs.get("score_transport"),
            "commentary": commentary,
        }
        snapshots.append(snapshot)

    return snapshots


# ---------------------------------------------------------------------------
# Main pipeline
# ---------------------------------------------------------------------------

def run_pipeline(airport: str, db_url: str):
    """Run the full sentiment pipeline for an airport."""
    airport = airport.upper()

    # Connect to DB
    logger.info("Connecting to database…")
    try:
        conn = psycopg2.connect(db_url)
    except Exception as exc:
        logger.error("Database connection failed: %s", exc)
        sys.exit(1)

    # Fetch reviews
    reviews = fetch_unprocessed_reviews(conn, airport)
    if not reviews:
        logger.info("No unprocessed reviews for %s. Nothing to do.", airport)
        result = {"airport": airport, "sentiment_snapshots": []}
        json.dump(result, sys.stdout, indent=2)
        sys.stdout.write("\n")
        conn.close()
        return

    # Load models
    emotion_pipe = load_emotion_pipeline()
    nli_tokenizer, nli_model, nli_device = load_nli_model()

    # Process each review
    processed_ids = []
    enriched_reviews = []

    for i, review in enumerate(reviews):
        review_text = review.get("review_text") or ""
        if not review_text.strip():
            logger.debug("Skipping review %s — empty text.", review.get("id"))
            processed_ids.append(review["id"])
            continue

        logger.info("Processing review %d/%d (id=%s)…", i + 1, len(reviews), review.get("id"))

        # Emotion classification
        try:
            emotions = emotion_pipe(review_text[:512])
            if isinstance(emotions, list) and emotions and isinstance(emotions[0], list):
                emotions = emotions[0]
            sentiment = classify_sentiment(emotions)
        except Exception as exc:
            logger.warning("Emotion classification failed for review %s: %s", review.get("id"), exc)
            sentiment = "neutral"

        # Topic classification
        try:
            topics = classify_topics(review_text, nli_tokenizer, nli_model, nli_device, TOPIC_LABELS)
        except Exception as exc:
            logger.warning("Topic classification failed for review %s: %s", review.get("id"), exc)
            topics = {}

        enriched = {
            **review,
            "review_date": (review["review_date"].strftime("%Y-%m-%d")
                            if hasattr(review.get("review_date"), "strftime")
                            else review.get("review_date")),
            "sentiment": sentiment,
            "topics": topics,
        }
        enriched_reviews.append(enriched)
        processed_ids.append(review["id"])

    # Aggregate into quarterly snapshots
    snapshots = aggregate_snapshots(enriched_reviews, airport)

    # Mark reviews as processed
    mark_reviews_processed(conn, processed_ids)
    conn.close()

    # Output
    result = {
        "airport": airport,
        "sentiment_snapshots": snapshots,
    }
    json.dump(result, sys.stdout, indent=2, ensure_ascii=False)
    sys.stdout.write("\n")
    logger.info("Pipeline complete. Generated %d quarterly snapshots.", len(snapshots))


def main():
    parser = argparse.ArgumentParser(
        description="Run sentiment analysis pipeline on airport reviews."
    )
    parser.add_argument(
        "--airport", required=True,
        help="IATA airport code (e.g. LTN, LHR)"
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

    run_pipeline(args.airport, db_url)


if __name__ == "__main__":
    main()
