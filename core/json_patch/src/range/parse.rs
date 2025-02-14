use super::error::RangeError;
use super::Range;
use winnow::{
    ascii::digit1,
    combinator::{alt, delimited, opt},
    token::take_till,
    PResult, Parser,
};

/// Parses a string segment to determine if it represents an index or a range.
///
/// e.g., `1:3`, `3:`, `:3`, `:`, `*`
///
/// # Errors
/// Returns `PatchError::InvalidOperation` if the segment does not conform to the
/// expected format or contains invalid numeric values.
pub(crate) fn parse_range(mut segment: &str) -> Result<Range, RangeError> {
    let input = &mut segment;

    _parse_range
        .parse_next(input)
        .map_err(|_| RangeError::InvalidRange {
            range: segment.to_string(),
        })
}

fn _parse_range(input: &mut &str) -> PResult<Range> {
    let range = alt((
        "*".value(Range::Full),
        parse_range_inner.map(|range| match range {
            (Some(s), Some(e)) => Range::FromTo(s..e),
            (Some(s), None) => Range::From(s..),
            (None, Some(e)) => Range::To(..e),
            (None, None) => Range::Full,
        }),
        digit1.parse_to().map(Range::Index),
    ));

    delimited("[", range, "]").parse_next(input)
}

/// Parse a range, e.g., "1:3", "3:", ":3", ":"
fn parse_range_inner(input: &mut &str) -> PResult<(Option<usize>, Option<usize>)> {
    let start = opt(digit1.parse_to()).parse_next(input)?;
    ":".parse_next(input)?;
    let end = opt(digit1.parse_to()).parse_next(input)?;

    Ok((start, end))
}

fn _is_range_op<'a>(input: &mut &'a str) -> PResult<&'a str> {
    delimited("[", take_till(0.., |c| c == ']'), "]").parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_range_inner() {
        let result = parse_range_inner(&mut "1:3").unwrap();
        assert_eq!(result, (Some(1), Some(3)));

        let result = parse_range_inner(&mut "1:").unwrap();
        assert_eq!(result, (Some(1), None));

        let result = parse_range_inner(&mut ":3").unwrap();
        assert_eq!(result, (None, Some(3)));
    }

    #[test]
    fn test_parse_full_range() {
        let result = parse_range("[:]").unwrap();
        assert_eq!(result, Range::Full);

        let result = parse_range("[*]").unwrap();
        assert_eq!(result, Range::Full);
    }

    #[test]
    fn test_parse_range_index() {
        let result = parse_range("[5]").unwrap();
        assert_eq!(result, Range::Index(5));
    }

    #[test]
    fn test_parse_range_from() {
        let result = parse_range("[0:]").unwrap();
        assert_eq!(result, Range::From(0..));

        let result = parse_range("[1:]").unwrap();
        assert_eq!(result, Range::From(1..));
    }

    #[test]
    fn test_parse_range_to() {
        let result = parse_range("[:10]");
        assert_eq!(result, Ok(Range::To(..10)));
    }

    #[test]
    fn test_parse_range_slice() {
        let result = parse_range("[1:10]").unwrap();
        assert_eq!(result, Range::FromTo(1..10));
    }

    ////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
    // Fail tests

    #[test]
    fn should_fail_to_parse_range_invalid_index() {
        let result = parse_range("abc");
        assert!(result.is_err());
    }

    #[test]
    fn should_fail_to_parse_range_invalid_slice() {
        let result = parse_range("[1:abc]");
        assert!(result.is_err());
    }

    #[test]
    fn should_fail_to_parse_range_invalid_format() {
        let result = parse_range("[1-3]");
        assert!(result.is_err());
    }

    #[test]
    fn should_fail_to_parse_range_empty_range() {
        let result = parse_range("[]");
        assert!(result.is_err());
    }
}
