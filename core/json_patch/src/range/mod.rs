pub mod error;
pub(crate) mod parse;

use self::error::RangeError;
use core::fmt;

/// Represents either an index or a range in a patch operation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum Range {
    /// `[index]`
    Index(usize),
    /// `[start..end]`
    FromTo(std::ops::Range<usize>),
    /// `..end`
    To(std::ops::RangeTo<usize>),
    /// `start..`
    From(std::ops::RangeFrom<usize>),
    /// All elements. `..`
    Full,
}

impl fmt::Display for Range {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Index(index) => write!(f, "[{}]", index),
            Self::FromTo(range) => write!(f, "[{}..{})", range.start, range.end),
            Self::From(range) => write!(f, "[..{})", range.start),
            Self::To(range) => write!(f, "[..{})", range.end),
            Self::Full => write!(f, ".."),
        }
    }
}

impl Range {
    /// Checks if the range is valid for the given array length.
    pub(crate) const fn check_valid_range(&self, array_len: usize) -> Result<(), RangeError> {
        match self {
            Self::Index(index) => {
                if *index >= array_len {
                    return Err(RangeError::IndexOutOfBounds {
                        index: *index,
                        len: array_len,
                    });
                }
            }
            Self::FromTo(range) => {
                if range.start >= range.end || range.start >= array_len || range.end > array_len {
                    return Err(RangeError::FromToOutOfBounds {
                        start: range.start,
                        end: range.end,
                        len: array_len,
                    });
                }
            }
            Self::To(range_to) => {
                if range_to.end > array_len {
                    return Err(RangeError::EndOutOfBounds {
                        end: range_to.end,
                        len: array_len,
                    });
                }
            }
            Self::From(range_from) => {
                if range_from.start >= array_len {
                    return Err(RangeError::StartOutOfBounds {
                        start: range_from.start,
                        len: array_len,
                    });
                }
            }
            Self::Full => {} // The full range is always valid
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_out_of_bounds() {
        let result = Range::Index(6).check_valid_range(5);
        assert_eq!(
            result,
            Err(RangeError::IndexOutOfBounds { index: 6, len: 5 })
        );
    }

    #[test]
    fn test_invalid_range() {
        let result = Range::FromTo(3..6).check_valid_range(5);
        assert_eq!(
            result,
            Err(RangeError::FromToOutOfBounds {
                start: 3,
                end: 6,
                len: 5
            })
        );
    }

    #[test]
    fn test_to_from_out_of_bounds_start() {
        #[allow(clippy::reversed_empty_ranges)]
        let result = Range::FromTo(6..4).check_valid_range(5);
        assert_eq!(
            result,
            Err(RangeError::FromToOutOfBounds {
                start: 6,
                end: 4,
                len: 5
            })
        );
    }

    #[test]
    fn test_to_from_out_of_bounds_end() {
        let result = Range::FromTo(2..6).check_valid_range(5);
        assert_eq!(
            result,
            Err(RangeError::FromToOutOfBounds {
                start: 2,
                end: 6,
                len: 5
            })
        );
    }

    #[test]
    fn test_start_out_of_bounds() {
        let result = Range::From(6..).check_valid_range(5);
        assert_eq!(
            result,
            Err(RangeError::StartOutOfBounds { start: 6, len: 5 })
        );
    }

    #[test]
    fn test_end_out_of_bounds() {
        let result = Range::To(..6).check_valid_range(5);
        assert_eq!(result, Err(RangeError::EndOutOfBounds { end: 6, len: 5 }));
    }

    #[test]
    fn test_valid_range() {
        let result = Range::FromTo(2..4).check_valid_range(5);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn test_full_range() {
        let result = Range::Full.check_valid_range(5);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn test_invalid_pattern() {
        #[allow(clippy::reversed_empty_ranges)]
        let result = Range::FromTo(2..1).check_valid_range(5);
        assert_eq!(
            result,
            Err(RangeError::FromToOutOfBounds {
                start: 2,
                end: 1,
                len: 5
            })
        );
    }
}
