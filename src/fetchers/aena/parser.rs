use calamine::Data;

/// Check if a cell string matches any of the search terms.
/// Uses "contains" matching so "ADOLFO SUÁREZ MADRID-BARAJAS" matches
/// the search term "madrid-barajas".
pub(super) fn matches_search_terms(cell_text: &str, search_terms: &[String]) -> bool {
    let lower = cell_text.trim().to_lowercase();
    search_terms.iter().any(|term| lower.contains(term.as_str()))
}

/// Extract passenger count for a specific airport from an AENA worksheet.
///
/// AENA annual reports have a consistent structure:
/// - Rows are sorted by passenger volume (busiest airport first)
/// - Each row has: airport name, total passengers, % change, ...
///   (repeated for passengers, operations, cargo in adjacent column groups)
/// - The airport name may be in column 0 or column 1 (varies by year)
/// - The total passengers is always the first large integer after the name
pub fn extract_airport_pax(range: &calamine::Range<Data>, search_terms: &[String]) -> Option<i64> {
    for row in range.rows() {
        // Collect all string cells to check for airport name match.
        let row_strings: Vec<String> = row
            .iter()
            .filter_map(|cell| match cell {
                Data::String(s) => Some(s.trim().to_lowercase()),
                _ => None,
            })
            .collect();

        let matches_airport = row_strings.iter().any(|s| {
            matches_search_terms(s, search_terms)
        });

        if !matches_airport {
            continue;
        }

        // Make sure the match is in the passenger column group (first string cell),
        // not only in operations/cargo columns further right.
        let first_string = row_strings.first()?;
        let is_pax_row = matches_search_terms(first_string, search_terms);
        if !is_pax_row {
            // The first string doesn't match — this might be an ops/cargo row
            // where the same airport name appears in a later column group.
            let match_count = row_strings.iter()
                .filter(|s| matches_search_terms(s, search_terms))
                .count();
            if match_count > 1 {
                // Multi-section row (pax + ops + cargo) — the first match IS the pax section.
                // Fall through to extract the number.
            } else {
                continue;
            }
        }

        // Extract the first large number from the row — this is the passenger total.
        // Skip percentage values (floats between -1 and 1) and small numbers.
        for cell in row {
            match cell {
                Data::Float(f) => {
                    let n = *f as i64;
                    if n > 100_000 {
                        return Some(n);
                    }
                }
                Data::Int(n) => {
                    if *n > 100_000 {
                        return Some(*n);
                    }
                }
                _ => {}
            }
        }
    }

    None
}
