use std::ops::Range;

/// Splits a given range into two parts:
/// one that fits within the bounds of `len`,
/// and one that overflows past `len`.
///
/// # Returns
///
/// A tuple `(in_bounds, overflow)`:
/// - `in_bounds`: the portion of the range that fits within `0..len`
/// - `overflow`: the portion of the range that exceeds `len`
///
/// # Examples
///
/// ```
/// use std::ops::Range;
/// use json_patch::range::split_range::split_range_at_len;
///
/// let (in_bounds, overflow) = split_range_at_len(2..6, 4);
/// assert_eq!(in_bounds, Some(2..4));
/// assert_eq!(overflow, Some(4..6));
/// ```
///
/// ```
/// let (in_bounds, overflow) = split_range_at_len(5..8, 5);
/// assert_eq!(in_bounds, None);
/// assert_eq!(overflow, Some(5..8));
/// ```
///
/// ```
/// let (in_bounds, overflow) = split_range_at_len(1..3, 10);
/// assert_eq!(in_bounds, Some(1..3));
/// assert_eq!(overflow, None);
/// ```
pub const fn split_range_at_len(
    range: Range<usize>,
    len: usize,
) -> (Option<Range<usize>>, Option<Range<usize>>) {
    if range.start >= len {
        // Entirely out of bounds
        (None, Some(range))
    } else if range.end <= len {
        // Entirely within bounds
        (Some(range), None)
    } else {
        // Partially in bounds, partially overflowing
        (Some(range.start..len), Some(len..range.end))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_within_bounds() {
        let (in_bounds, overflow) = split_range_at_len(1..4, 10);
        assert_eq!(in_bounds, Some(1..4));
        assert_eq!(overflow, None);
    }

    #[test]
    fn test_entirely_overflowing() {
        let (in_bounds, overflow) = split_range_at_len(5..8, 5);
        assert_eq!(in_bounds, None);
        assert_eq!(overflow, Some(5..8));
    }

    #[test]
    fn test_partially_overflowing() {
        let (in_bounds, overflow) = split_range_at_len(2..6, 4);
        assert_eq!(in_bounds, Some(2..4));
        assert_eq!(overflow, Some(4..6));
    }

    #[test]
    fn test_edge_case_start_equals_len() {
        let (in_bounds, overflow) = split_range_at_len(4..6, 4);
        assert_eq!(in_bounds, None);
        assert_eq!(overflow, Some(4..6));
    }

    #[test]
    fn test_exact_match_to_len() {
        let (in_bounds, overflow) = split_range_at_len(2..5, 5);
        assert_eq!(in_bounds, Some(2..5));
        assert_eq!(overflow, None);
    }

    #[test]
    fn test_empty_range() {
        let (in_bounds, overflow) = split_range_at_len(3..3, 5);
        assert_eq!(in_bounds, Some(3..3));
        assert_eq!(overflow, None);
    }
}
