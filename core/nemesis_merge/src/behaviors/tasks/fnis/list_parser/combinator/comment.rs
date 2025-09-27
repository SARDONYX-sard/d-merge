//! Line comments parsing (`' comment`)

use winnow::ascii::{line_ending, space0, till_line_ending};
use winnow::combinator::{alt, eof, opt, preceded, seq};
use winnow::{ModalResult, Parser};

/// Parses an optional line comment and consumes the trailing newline or end-of-input.
///
/// # Examples of accepted input
/// - `"   ' hello world\r\n"`
/// - `"   ' hello world"`
/// - `"   "`
/// - `""`
pub fn take_till_line_or_eof<'a>(input: &mut &'a str) -> ModalResult<Option<&'a str>> {
    let (comment,) = seq! {
        _: space0,
        opt(comment_line),
        _: alt((line_ending, eof))
    }
    .parse_next(input)?;
    Ok(comment)
}

/// Parses a single comment line until line ending.
fn comment_line<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
    preceded('\'', till_line_ending).parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_comment_line_only() {
        let mut input = "' comment\n";
        let result = comment_line(&mut input).unwrap();
        assert_eq!(result, " comment");
        assert_eq!(input, "\n"); // line ending remains
    }

    #[test]
    fn test_parse_opt_comment_line_with_comment_and_newline() {
        let mut input = "   ' hello world\n";
        assert!(take_till_line_or_eof(&mut input).is_ok());
        assert_eq!(input, "");
    }

    #[test]
    fn test_parse_opt_comment_line_with_comment_and_eof() {
        let mut input = "   ' hello world";
        assert!(take_till_line_or_eof(&mut input).is_ok());
        assert_eq!(input, "");
    }

    #[test]
    fn test_parse_opt_comment_line_with_only_whitespace() {
        let mut input = "   ";
        assert!(take_till_line_or_eof(&mut input).is_ok());
        assert_eq!(input, "");
    }

    #[test]
    fn test_parse_opt_comment_line_with_only_newline() {
        let mut input = "\n";
        assert!(take_till_line_or_eof(&mut input).is_ok());
        assert_eq!(input, "");
    }
}
