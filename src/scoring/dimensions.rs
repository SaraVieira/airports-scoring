use super::data::ScoringData;

/// Infrastructure score (weight: 15%)
///
/// runway_score     = LEAST(runway_count / 3.0, 1.0) * 100
/// length_score     = LEAST(longest_runway_ft / 13000.0, 1.0) * 100
/// age_score        = renovation-aware aging formula
/// capacity_score   = LEAST((annual_pax_latest / capacity) * 100, 100)
/// lounge_score     = LEAST(lounge_count / 8.0, 1.0) * 100
/// carbon_score     = (carbon_level / 7.0) * 100  (0 if no accreditation)
///
/// score = runway_score * 0.25 + length_score * 0.20 + age_score * 0.20
///       + capacity_score * 0.15 + lounge_score * 0.10 + carbon_score * 0.10
pub(crate) fn score_infrastructure(data: &ScoringData, reference_year: i16) -> f64 {
    let runway_score = (data.runway_count as f64 / 3.0).min(1.0) * 100.0;

    let length_score = data
        .max_runway_length_ft
        .map(|len| (len as f64 / 13000.0).min(1.0) * 100.0)
        .unwrap_or(0.0);

    let age_score = if let Some(reno) = data.last_major_reno {
        (100.0 - (reference_year as f64 - reno as f64) * 3.0).max(0.0)
    } else if let Some(opened) = data.opened_year {
        (100.0 - (reference_year as f64 - opened as f64) * 1.5).max(0.0)
    } else {
        50.0
    };

    let capacity_score = match (data.annual_pax_latest_m, data.annual_capacity_m) {
        (Some(pax), Some(cap)) if cap > 0.0 => ((pax / cap) * 100.0).min(100.0),
        _ => 50.0,
    };

    let lounge_score = (data.lounge_count as f64 / 8.0).min(1.0) * 100.0;

    let carbon_score = data.carbon_level
        .map(|level| (level as f64 / 7.0) * 100.0)
        .unwrap_or(0.0);

    let score = runway_score * 0.25
        + length_score * 0.20
        + age_score * 0.20
        + capacity_score * 0.15
        + lounge_score * 0.10
        + carbon_score * 0.10;

    score.clamp(0.0, 100.0)
}

/// Operational score (weight: 30%)
///
/// delay_score        = GREATEST(0, 100 - (delay_pct * 2.5))
/// avg_delay_score    = GREATEST(0, 100 - (avg_delay_minutes * 3))
/// taxi_score         = GREATEST(0, 100 - avg(taxi_out, taxi_in) * 10)
/// asma_score         = GREATEST(0, 100 - asma_additional_min * 15)
/// slot_score         = slot_adherence_pct (already 0-100)
/// cancellation_score = GREATEST(0, 100 - (cancellation_pct * 10))
///
/// attribution_modifier = 1.0 - (airport_delay_pct * 0.003)
/// cdo_bonus = if cdo_pct > 50%, up to +3 points
///
/// score = (delay * 0.25 + avg_delay * 0.20 + taxi * 0.15
///        + asma * 0.15 + slot * 0.15 + cancel * 0.10) * modifier + cdo_bonus
pub(crate) fn score_operational(data: &ScoringData) -> f64 {
    let delay_score = data
        .avg_delay_pct
        .map(|d| (100.0 - d * 2.5).max(0.0))
        .unwrap_or(70.0);

    let avg_delay_score = data
        .avg_delay_minutes
        .map(|d| (100.0 - d * 3.0).max(0.0))
        .unwrap_or(70.0);

    // Combined taxi: average of taxi-out and taxi-in additional time
    let taxi_score = match (data.taxi_out_additional_min, data.taxi_in_additional_min) {
        (Some(out), Some(inn)) => (100.0 - ((out + inn) / 2.0) * 10.0).max(0.0),
        (Some(out), None) => (100.0 - out * 10.0).max(0.0),
        (None, Some(inn)) => (100.0 - inn * 10.0).max(0.0),
        (None, None) => 70.0,
    };

    // ASMA approach congestion: additional minutes per flight
    let asma_score = data
        .asma_additional_min
        .map(|d| (100.0 - d * 15.0).max(0.0))
        .unwrap_or(70.0);

    // Slot adherence: already a percentage (0-100)
    let slot_score = data
        .slot_adherence_pct
        .unwrap_or(70.0);

    let cancellation_score = data
        .avg_cancellation_pct
        .map(|d| (100.0 - d * 10.0).max(0.0))
        .unwrap_or(80.0);

    let attribution_modifier = data
        .delay_airport_pct
        .map(|d| 1.0 - d * 0.003)
        .unwrap_or(1.0);

    let raw = delay_score * 0.25
        + avg_delay_score * 0.20
        + taxi_score * 0.15
        + asma_score * 0.15
        + slot_score * 0.15
        + cancellation_score * 0.10;

    // CDO environmental bonus: up to +3 points if >50% of flights are CDO
    let cdo_bonus = data
        .cdo_pct
        .map(|pct| if pct > 50.0 { ((pct - 50.0) / 50.0) * 3.0 } else { 0.0 })
        .unwrap_or(0.0);

    (raw * attribution_modifier + cdo_bonus).clamp(0.0, 100.0)
}

/// Sentiment score (weight: 25%)
///
/// rating_score    = ((avg_rating - 1) / 9.0) * 100   -- normalise 1-10 to 0-100
/// sub_score_avg   = ((sum_of_non_null_sub_scores - count) / (count * 4.0)) * 100
/// confidence      = LEAST(review_count / 500.0, 1.0)
///
/// score = (rating_score * 0.6 + sub_score_avg * 0.4) * confidence
///       + rating_score * (1 - confidence) * 0.6
pub(crate) fn score_sentiment(data: &ScoringData) -> f64 {
    match data.weighted_avg_rating {
        Some(rating) => {
            // avg_rating from sentiment_snapshots is already on 1-10 scale
            // (Skytrax uses 1-10 natively, Google reviews are stored as rating * 2)
            let rating_score = ((rating - 1.0) / 9.0) * 100.0;

            let sub_score_avg = if data.sub_score_count > 0 {
                // Sub-scores are 0-5 scale. Normalise: (sum - count) / (count * 4) * 100
                let count = data.sub_score_count as f64;
                ((data.sub_score_sum - count) / (count * 4.0)) * 100.0
            } else {
                rating_score // fallback to rating if no sub-scores
            };

            let confidence = data
                .total_review_count
                .map(|c| (c as f64 / 500.0).min(1.0))
                .unwrap_or(0.0);

            let score = (rating_score * 0.6 + sub_score_avg * 0.4) * confidence
                + rating_score * (1.0 - confidence) * 0.6;

            score.clamp(0.0, 100.0)
        }
        None => 50.0,
    }
}

/// Sentiment velocity score (weight: 15%)
///
/// Compares last 2 years (8 quarters) vs prior 2 years (8 quarters).
/// This captures longer improvement arcs (e.g., Luton's renovation journey).
///
/// delta = avg_rating_last_8q - avg_rating_prior_8q  (on 0-5 scale)
/// score = LEAST(100, GREATEST(0, 50 + (delta * 20)))
///
/// 50 = flat, 70 = +1.0 rating improvement, 30 = -1.0 decline
pub(crate) fn score_sentiment_velocity(data: &ScoringData) -> f64 {
    match (data.avg_rating_last_8q, data.avg_rating_prior_8q) {
        (Some(last), Some(prior)) => {
            let delta = last - prior;
            (50.0 + delta * 20.0).clamp(0.0, 100.0)
        }
        _ => 50.0, // no trend data = flat
    }
}

/// Connectivity score (weight: 10%)
///
/// destination_score = LEAST(unique_destination_count / 100.0, 1.0) * 100
/// airline_score     = LEAST(airline_count / 30.0, 1.0) * 100
/// intl_ratio_score  = (international_pax / total_pax) * 100
/// transport_score   = LEAST(transport_modes_count / 4.0, 1.0) * 100, +20% if direct rail
/// hub_score         = LEAST((hub * 3 + focus_city * 1.5 + operating_base) / 10.0, 1.0) * 100
///
/// score = destination_score * 0.30 + airline_score * 0.20 + intl_ratio_score * 0.20
///       + transport_score * 0.15 + hub_score * 0.15
pub(crate) fn score_connectivity(data: &ScoringData) -> f64 {
    let destination_score = (data.destination_count as f64 / 100.0).min(1.0) * 100.0;
    let airline_score = (data.airline_count as f64 / 30.0).min(1.0) * 100.0;

    let intl_ratio_score = match (data.international_pax, data.total_pax) {
        (Some(intl), Some(total)) if total > 0 => (intl as f64 / total as f64) * 100.0,
        _ => 50.0,
    };

    let mut transport_score = (data.transport_modes_count as f64 / 4.0).min(1.0) * 100.0;
    if data.has_direct_rail {
        transport_score = (transport_score * 1.2).min(100.0);
    }

    let hub_value = data.hub_airline_count as f64 * 3.0
        + data.focus_city_count as f64 * 1.5
        + data.operating_base_count as f64;
    let hub_score = (hub_value / 10.0).min(1.0) * 100.0;

    let score = destination_score * 0.30
        + airline_score * 0.20
        + intl_ratio_score * 0.20
        + transport_score * 0.15
        + hub_score * 0.15;

    score.clamp(0.0, 100.0)
}
