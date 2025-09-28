//! Line comments parsing (`' comment`)

use winnow::ascii::{line_ending, space1, till_line_ending};
use winnow::combinator::{alt, opt, preceded, repeat, seq};
use winnow::{ModalResult, Parser};

/// Skip any amount of spaces, newlines, and `' comment` lines.
/// This is like `multispace0`, but also removes comments.
///
/// # Examples
/// - `"   foo"` → leaves `"foo"`
/// - `"' comment\nfoo"` → leaves `"foo"`
/// - `"   ' comment\n   ' another\nfoo"` → leaves `"foo"`
pub fn skip_ws_and_comments<'a>(input: &mut &'a str) -> ModalResult<()> {
    repeat(0.., alt((space1, line_ending, comment_line))).parse_next(input)?;
    Ok(())
}

/// Parses a single `' comment` line until line ending (without consuming it).
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
        assert!(skip_ws_and_comments(&mut input).is_ok());
        assert_eq!(input, "");
    }

    #[test]
    fn test_parse_opt_comment_line_with_comment_and_eof() {
        let mut input = "   ' hello world";
        assert!(skip_ws_and_comments(&mut input).is_ok());
        assert_eq!(input, "");
    }

    #[test]
    fn test_parse_opt_comment_line_with_only_whitespace() {
        let mut input = "   ";
        assert!(skip_ws_and_comments(&mut input).is_ok());
        assert_eq!(input, "");
    }

    #[test]
    fn test_parse_opt_comment_line_with_only_newline() {
        let mut input = "\n";
        assert!(skip_ws_and_comments(&mut input).is_ok());
        assert_eq!(input, "");
    }

    #[test]
    fn test_skip_ws_and_comments_only_space() {
        let mut input = "   foo";
        skip_ws_and_comments(&mut input).unwrap();
        assert_eq!(input, "foo");
    }

    #[test]
    fn test_skip_ws_and_comments_single_comment() {
        let mut input = "' hello\nfoo";
        skip_ws_and_comments(&mut input).unwrap();
        assert_eq!(input, "foo");
    }

    #[test]
    fn test_skip_ws_and_comments_multiple_comments_and_spaces() {
        let mut input = "   ' one\n   ' two\nfoo";
        skip_ws_and_comments(&mut input).unwrap();
        assert_eq!(input, "foo");
    }

    #[test]
    fn test_skip_ws_and_comments_empty() {
        let mut input = "";
        skip_ws_and_comments(&mut input).unwrap();
        assert_eq!(input, "");
    }
}
