use rust_decimal::Decimal;

/// Find a column index by trying multiple candidate header names (case-insensitive, substring match).
pub(crate) fn find_col(headers: &csv::StringRecord, candidates: &[&str]) -> Option<usize> {
    for (i, h) in headers.iter().enumerate() {
        let lower = h.trim().to_lowercase();
        for &candidate in candidates {
            if lower == candidate || lower.contains(candidate) {
                return Some(i);
            }
        }
    }
    None
}

/// Find a column index by exact header name (case-insensitive).
pub(crate) fn find_col_exact(headers: &csv::StringRecord, name: &str) -> Option<usize> {
    for (i, h) in headers.iter().enumerate() {
        if h.trim().eq_ignore_ascii_case(name) {
            return Some(i);
        }
    }
    None
}

/// Parse a f64 from a CSV cell, returning 0.0 if missing or unparseable.
pub(crate) fn parse_f64_col(record: &csv::StringRecord, col: Option<usize>) -> f64 {
    col.and_then(|c| record.get(c))
        .and_then(|v| v.trim().parse::<f64>().ok())
        .unwrap_or(0.0)
}

/// Compute a cause percentage (cause_minutes / total_minutes * 100).
pub(crate) fn compute_cause_pct(cause_min: f64, total_min: f64) -> Option<Decimal> {
    if total_min > 0.0 && cause_min > 0.0 {
        let pct = (cause_min / total_min) * 100.0;
        let mut d = Decimal::from_f64_retain(pct).unwrap_or_default();
        d.rescale(2);
        Some(d)
    } else {
        None
    }
}
