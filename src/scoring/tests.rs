use rust_decimal::prelude::*;

use super::data::{
    ScoringData, SentimentRow, YearlyOps,
    weighted_avg_ops, weighted_avg_sentiment, year_weight,
};
use super::dimensions::*;

fn empty_data() -> ScoringData {
    ScoringData {
        runway_count: 0,
        max_runway_length_ft: None,
        annual_capacity_m: None,
        annual_pax_latest_m: None,
        opened_year: None,
        last_major_reno: None,
        avg_delay_pct: None,
        avg_cancellation_pct: None,
        avg_delay_minutes: None,
        delay_airport_pct: None,
        taxi_out_additional_min: None,
        weighted_avg_rating: None,
        total_review_count: None,
        sub_score_count: 0,
        sub_score_sum: 0.0,
        avg_rating_last_8q: None,
        avg_rating_prior_8q: None,
        destination_count: 0,
        airline_count: 0,
        international_pax: None,
        total_pax: None,
        operator_avg_sentiment: None,
        operator_avg_operational: None,
        operator_airport_count: 0,
    }
}

#[test]
fn infrastructure_large_hub() {
    let data = ScoringData {
        runway_count: 4,
        max_runway_length_ft: Some(13000),
        annual_capacity_m: Some(80.0),
        annual_pax_latest_m: Some(70.0),
        last_major_reno: Some(2020),
        ..empty_data()
    };
    let score = score_infrastructure(&data, 2024);
    // runway: min(4/3,1)*100 = 100 * 0.35 = 35
    // length: min(13000/13000,1)*100 = 100 * 0.25 = 25
    // age: 100 - (2024-2020)*3 = 88 * 0.25 = 22
    // capacity: (70/80)*100 = 87.5 * 0.15 = 13.125
    // total = 95.125
    assert!((score - 95.125).abs() < 0.01, "got {}", score);
}

#[test]
fn infrastructure_single_runway_no_data() {
    let data = ScoringData {
        runway_count: 1,
        ..empty_data()
    };
    let score = score_infrastructure(&data, 2024);
    // runway: (1/3)*100 = 33.3 * 0.35 = 11.67
    // length: 0 * 0.25 = 0
    // age: 50 * 0.25 = 12.5
    // capacity: 50 * 0.15 = 7.5
    // total ~ 31.67
    assert!((score - 31.67).abs() < 0.1, "got {}", score);
}

#[test]
fn infrastructure_old_unrenovated_airport() {
    let data = ScoringData {
        runway_count: 2,
        max_runway_length_ft: Some(10000),
        opened_year: Some(1950),
        ..empty_data()
    };
    let score = score_infrastructure(&data, 2024);
    // runway: min(2/3,1)*100 = 66.7 * 0.35 = 23.3
    // length: min(10000/13000,1)*100 = 76.9 * 0.25 = 19.2
    // age: 100 - 74*1.5 = 0 (clamped) * 0.25 = 0
    // capacity: 50 * 0.15 = 7.5
    // total ~ 50.0
    // Age penalty bottoms out, but runways+length keep score around 50.
    assert!(score < 55.0, "old airport should not score high, got {}", score);
    assert!(score > 40.0, "airport has decent runways, got {}", score);
}

#[test]
fn operational_perfect() {
    let data = ScoringData {
        avg_delay_pct: Some(0.0),
        avg_cancellation_pct: Some(0.0),
        avg_delay_minutes: Some(0.0),
        delay_airport_pct: Some(0.0),
        taxi_out_additional_min: Some(0.0),
        ..empty_data()
    };
    let score = score_operational(&data);
    assert!((score - 100.0).abs() < 0.01, "got {}", score);
}

#[test]
fn operational_high_delays() {
    let data = ScoringData {
        avg_delay_pct: Some(40.0),          // 100 - 100 = 0
        avg_cancellation_pct: Some(5.0),    // 100 - 50 = 50
        avg_delay_minutes: Some(30.0),      // 100 - 90 = 10
        delay_airport_pct: Some(50.0),      // modifier: 1 - 0.15 = 0.85
        taxi_out_additional_min: Some(5.0), // 100 - 50 = 50
        ..empty_data()
    };
    let score = score_operational(&data);
    // raw = 0*0.35 + 10*0.25 + 50*0.20 + 50*0.20 = 22.5
    // * 0.85 = 19.125
    assert!((score - 19.125).abs() < 0.01, "got {}", score);
}

#[test]
fn operational_no_data_defaults() {
    let data = empty_data();
    let score = score_operational(&data);
    // 70*0.35 + 70*0.25 + 80*0.20 + 70*0.20 = 72
    assert!((score - 72.0).abs() < 0.01, "got {}", score);
}

#[test]
fn sentiment_high_rating_high_confidence() {
    let data = ScoringData {
        weighted_avg_rating: Some(9.0), // 1-10 scale -> (9-1)/9*100 = 88.89
        total_review_count: Some(1000),     // confidence = 1.0
        sub_score_count: 4,
        sub_score_sum: 18.0,          // (18-4)/(4*4)*100 = 87.5
        ..empty_data()
    };
    let score = score_sentiment(&data);
    // (88.89 * 0.6 + 87.5 * 0.4) * 1.0 + 0 = 88.33
    assert!(score > 85.0 && score < 92.0, "got {}", score);
}

#[test]
fn sentiment_no_data_returns_neutral() {
    let data = empty_data();
    let score = score_sentiment(&data);
    assert!((score - 50.0).abs() < 0.01, "got {}", score);
}

#[test]
fn sentiment_low_confidence() {
    let data = ScoringData {
        weighted_avg_rating: Some(8.0), // 1-10 scale -> (8-1)/9*100 = 77.78
        total_review_count: Some(50),       // confidence = 0.1
        sub_score_count: 0,
        ..empty_data()
    };
    let score = score_sentiment(&data);
    // (77.78*0.6 + 77.78*0.4)*0.1 + 77.78*0.9*0.6 = 7.78 + 42.0 = 49.8
    assert!(score > 45.0 && score < 55.0, "low confidence should temper, got {}", score);
}

#[test]
fn velocity_improving() {
    let data = ScoringData {
        avg_rating_last_8q: Some(4.0),
        avg_rating_prior_8q: Some(3.0),
        ..empty_data()
    };
    let score = score_sentiment_velocity(&data);
    assert!((score - 70.0).abs() < 0.01, "got {}", score);
}

#[test]
fn velocity_declining() {
    let data = ScoringData {
        avg_rating_last_8q: Some(2.5),
        avg_rating_prior_8q: Some(4.0),
        ..empty_data()
    };
    let score = score_sentiment_velocity(&data);
    assert!((score - 20.0).abs() < 0.01, "got {}", score);
}

#[test]
fn velocity_no_data_returns_flat() {
    let data = empty_data();
    let score = score_sentiment_velocity(&data);
    assert!((score - 50.0).abs() < 0.01, "got {}", score);
}

#[test]
fn connectivity_large_hub() {
    let data = ScoringData {
        destination_count: 200,
        airline_count: 50,
        international_pax: Some(60_000_000),
        total_pax: Some(80_000_000),
        ..empty_data()
    };
    let score = score_connectivity(&data);
    // 100*0.4 + 100*0.3 + 75*0.3 = 92.5
    assert!((score - 92.5).abs() < 0.01, "got {}", score);
}

#[test]
fn connectivity_small_airport() {
    let data = ScoringData {
        destination_count: 10,
        airline_count: 3,
        ..empty_data()
    };
    let score = score_connectivity(&data);
    // 10*0.4 + 10*0.3 + 50*0.3 = 22
    assert!((score - 22.0).abs() < 0.01, "got {}", score);
}

#[test]
fn operator_multi_airport() {
    let data = ScoringData {
        operator_avg_sentiment: Some(75.0),
        operator_avg_operational: Some(85.0),
        operator_airport_count: 5,
        ..empty_data()
    };
    let score = score_operator(&data);
    assert!((score - 80.0).abs() < 0.01, "got {}", score);
}

#[test]
fn operator_single_airport_blending() {
    let data = ScoringData {
        operator_avg_sentiment: Some(90.0),
        operator_avg_operational: Some(80.0),
        operator_airport_count: 1,
        ..empty_data()
    };
    let score = score_operator(&data);
    // (90+80)/2 = 85, blended: 85*0.5 + 50*0.5 = 67.5
    assert!((score - 67.5).abs() < 0.01, "got {}", score);
}

#[test]
fn operator_no_data_returns_neutral() {
    let data = empty_data();
    let score = score_operator(&data);
    assert!((score - 50.0).abs() < 0.01, "got {}", score);
}

#[test]
fn all_scores_clamped_0_to_100() {
    let extreme = ScoringData {
        runway_count: 100,
        max_runway_length_ft: Some(99999),
        annual_capacity_m: Some(0.001),
        annual_pax_latest_m: Some(999.0),
        opened_year: Some(1800),
        last_major_reno: None,
        avg_delay_pct: Some(100.0),
        avg_cancellation_pct: Some(100.0),
        avg_delay_minutes: Some(999.0),
        delay_airport_pct: Some(100.0),
        taxi_out_additional_min: Some(999.0),
        weighted_avg_rating: Some(5.0),
        total_review_count: Some(99999),
        sub_score_count: 8,
        sub_score_sum: 40.0,
        avg_rating_last_8q: Some(5.0),
        avg_rating_prior_8q: Some(0.0),
        destination_count: 9999,
        airline_count: 9999,
        international_pax: Some(999_999_999),
        total_pax: Some(1),
        operator_avg_sentiment: Some(100.0),
        operator_avg_operational: Some(100.0),
        operator_airport_count: 100,
    };
    for (name, val) in [
        ("infra", score_infrastructure(&extreme, 2024)),
        ("ops", score_operational(&extreme)),
        ("sent", score_sentiment(&extreme)),
        ("vel", score_sentiment_velocity(&extreme)),
        ("conn", score_connectivity(&extreme)),
        ("oper", score_operator(&extreme)),
    ] {
        assert!(val >= 0.0 && val <= 100.0, "{} = {} out of bounds", name, val);
    }
}

#[test]
fn year_weight_values() {
    assert!((year_weight(2015) - 1.0).abs() < 0.01);
    assert!((year_weight(2020) - 2.5).abs() < 0.01);
    assert!((year_weight(2025) - 4.0).abs() < 0.01);
    // Pre-2015 should clamp to 1.0
    assert!((year_weight(2010) - 1.0).abs() < 0.01);
}

#[test]
fn weighted_ops_recency_bias() {
    // Recent year should dominate over old year
    let ops = vec![
        YearlyOps {
            period_year: 2015,
            avg_delay_pct: Some(Decimal::from(50)),
            avg_cancellation_pct: None,
            avg_delay_minutes: None,
            avg_delay_airport_pct: None,
        },
        YearlyOps {
            period_year: 2025,
            avg_delay_pct: Some(Decimal::from(10)),
            avg_cancellation_pct: None,
            avg_delay_minutes: None,
            avg_delay_airport_pct: None,
        },
    ];
    let (delay, _, _, _) = weighted_avg_ops(&ops);
    let d = delay.unwrap();
    // 2015 weight=1.0, 2025 weight=4.0
    // (50*1 + 10*4) / (1+4) = 90/5 = 18.0
    assert!((d - 18.0).abs() < 0.01, "expected ~18.0, got {}", d);
}

#[test]
fn weighted_sentiment_recency_and_volume() {
    let snapshots = vec![
        SentimentRow {
            snapshot_year: 2015,
            snapshot_quarter: Some(1),
            avg_rating: Some(Decimal::from(2)),
            review_count: Some(100),
            score_queuing: None, score_cleanliness: None, score_staff: None,
            score_food_bev: None, score_shopping: None, score_wifi: None,
            score_wayfinding: None, score_transport: None,
        },
        SentimentRow {
            snapshot_year: 2025,
            snapshot_quarter: Some(1),
            avg_rating: Some(Decimal::from(4)),
            review_count: Some(100),
            score_queuing: None, score_cleanliness: None, score_staff: None,
            score_food_bev: None, score_shopping: None, score_wifi: None,
            score_wayfinding: None, score_transport: None,
        },
    ];
    let (rating, total, _, _) = weighted_avg_sentiment(&snapshots);
    let r = rating.unwrap();
    // 2015: weight=1.0*10=10, 2025: weight=4.0*10=40
    // (2*10 + 4*40) / (10+40) = 180/50 = 3.6
    assert!((r - 3.6).abs() < 0.01, "expected ~3.6, got {}", r);
    assert_eq!(total, Some(200));
}
