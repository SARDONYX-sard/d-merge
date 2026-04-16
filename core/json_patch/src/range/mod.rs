pub mod error;
pub(crate) mod parse;
pub mod split_range;

use core::fmt;

/// Represents either an index or a range in a patch operation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Range {
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
            Self::Index(index) => write!(f, "[{index}]"),
            Self::FromTo(range) => write!(f, "[{}..{}]", range.start, range.end),
            Self::From(range) => write!(f, "[..{}]", range.start),
            Self::To(range) => write!(f, "[..{}]", range.end),
            Self::Full => write!(f, "[..]"),
        }
    }
}
