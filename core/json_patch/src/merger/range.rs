use super::error::PatchError;

/// Represents either an index or a range in a patch operation.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum RangeOrIndex {
    /// Represents a single index.
    Index(usize),
    /// Represents a range of indices.
    Range(std::ops::Range<usize>),
}

/// Parses a string segment to determine if it represents an index or a range.
///
/// e.g. `[1:3]`
///
/// # Errors
/// Returns `PatchError::InvalidOperation` if the segment does not conform to the
/// expected format or contains invalid numeric values.
pub(crate) fn parse_index_or_range(segment: &str) -> Result<RangeOrIndex, PatchError> {
    if segment.starts_with('[') && segment.ends_with(']') {
        let inner = &segment[1..segment.len() - 1]; // Strip the brackets
        if let Some((start, end)) = inner.split_once(':') {
            let start: usize = start.parse().map_err(|_| PatchError::InvalidOperation {
                path: segment.to_string(),
            })?;
            let end: usize = end.parse().map_err(|_| PatchError::InvalidOperation {
                path: segment.to_string(),
            })?;
            return Ok(RangeOrIndex::Range(start..end));
        } else {
            let index: usize = inner.parse().map_err(|_| PatchError::InvalidOperation {
                path: segment.to_string(),
            })?;
            return Ok(RangeOrIndex::Index(index));
        }
    }
    Err(PatchError::InvalidOperation {
        path: segment.to_string(),
    })?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_index_or_range_index() {
        let result = parse_index_or_range("[5]").unwrap();
        assert_eq!(result, RangeOrIndex::Index(5));
    }

    #[test]
    fn test_parse_index_or_range_range() {
        let result = parse_index_or_range("[1:10]").unwrap();
        assert_eq!(result, RangeOrIndex::Range(1..10));
    }

    #[test]
    fn test_parse_index_or_range_invalid_index() {
        let result = parse_index_or_range("abc");
        assert!(matches!(result, Err(PatchError::InvalidOperation { .. })));
    }

    #[test]
    fn test_parse_index_or_range_invalid_range() {
        let result = parse_index_or_range("[1:abc]");
        assert!(matches!(result, Err(PatchError::InvalidOperation { .. })));
    }

    #[test]
    fn test_parse_index_or_range_invalid_format() {
        let result = parse_index_or_range("[1-3]");
        assert!(matches!(result, Err(PatchError::InvalidOperation { .. })));
    }

    #[test]
    fn test_parse_index_or_range_empty_range() {
        let result = parse_index_or_range("[]");
        assert!(matches!(result, Err(PatchError::InvalidOperation { .. })));
    }
}
