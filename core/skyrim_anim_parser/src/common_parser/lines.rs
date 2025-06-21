use core::str::FromStr;
use std::borrow::Cow;
use winnow::{
    ascii::{line_ending, space0, till_line_ending},
    combinator::alt,
    error::{ContextError, ErrMode, StrContext::*, StrContextValue::*},
    ModalResult, Parser,
};

pub type Str<'a> = Cow<'a, str>;

/// Parse 1 line.
pub(crate) fn one_line<'a>(input: &mut &'a str) -> ModalResult<Str<'a>> {
    let line = till_line_ending.parse_next(input)?;
    // In the case of patches, this may not be present, so `opt`
    line_ending.parse_next(input)?; // skip line end
    Ok(line.into())
}

pub(crate) fn lines<'a>(
    read_len: usize,
) -> impl Parser<&'a str, Vec<Str<'a>>, ErrMode<ContextError>> {
    move |input: &mut &'a str| {
        let mut lines = vec![];
        for _ in 0..read_len {
            lines.push(one_line.parse_next(input)?);
        }
        Ok(lines)
    }
}

/// Parse txt stem line.
///
/// e.g. `DefaultMale.txt` -> `DefaultMale`
pub(crate) fn txt_one_line<'a>(input: &mut &'a str) -> ModalResult<Str<'a>> {
    space0.parse_next(input)?;
    let name = winnow::token::take_until(0.., ".txt").parse_next(input)?;
    till_line_ending.parse_next(input)?;
    line_ending.parse_next(input)?; // skip line end
    Ok(name.into())
}

/// Parse one line and then parse to T.
#[inline]
pub(crate) fn verify_line_parses_to<'a, T>(input: &mut &'a str) -> ModalResult<Str<'a>>
where
    T: FromStr,
{
    // For some reason, using parse_to for Cow causes an error, so the method chain of the existing parser is used.
    let line = till_line_ending
        .verify(|s: &str| s.parse::<T>().is_ok())
        .parse_next(input)?;
    line_ending.parse_next(input)?; // skip line end
    Ok(line.into())
}

/// Parse one line and then parse to T.
#[inline]
pub(crate) fn parse_one_line<T: FromStr>(input: &mut &str) -> ModalResult<T> {
    // For some reason, using parse_to for Cow causes an error, so the method chain of the existing parser is used.
    let line = till_line_ending.parse_to().parse_next(input)?;
    // In the case of patches, this may not be present, so `opt`
    line_ending.parse_next(input)?; // skip line end
    Ok(line)
}

/// - `'0'` => `false`
/// - `'1'` => `true`
fn num_bool(input: &mut &str) -> ModalResult<bool> {
    alt(('0'.value(false), '1'.value(true)))
        .context(Expected(CharLiteral('0')))
        .context(Expected(CharLiteral('1')))
        .parse_next(input)
}

/// - `'0'` => `false`
/// - `'1'` => `true`
pub(crate) fn num_bool_line(input: &mut &str) -> ModalResult<bool> {
    let boolean = num_bool.parse_next(input)?;
    line_ending.parse_next(input)?; // skip line end
    Ok(boolean)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_one_line() {
        let mut input = "hello world\n";
        let result = one_line(&mut input).unwrap();
        assert_eq!(result, "hello world");

        let mut input_empty = "\n";
        let result_empty = one_line(&mut input_empty).unwrap();
        assert_eq!(result_empty, "");

        let mut input_no_newline = "no newline";
        assert!(one_line(&mut input_no_newline).is_err());
    }

    #[test]
    fn test_lines() {
        let mut input = "line 1\nline 2\nline 3\n";
        let mut parser = lines(2);
        let result = parser.parse_next(&mut input).unwrap();
        assert_eq!(result, vec!["line 1", "line 2"]);

        let mut input_fewer = "only one line\n";
        let mut parser = lines(2);
        assert!(parser.parse_next(&mut input_fewer).is_err());

        let mut input_exact = "line A\nline B\n";
        let mut parser = lines(2);
        let result_exact = parser.parse_next(&mut input_exact).unwrap();
        assert_eq!(result_exact, vec!["line A", "line B"]);
    }

    #[test]
    fn test_from_one_line() {
        let mut input = "123\n";
        let result = parse_one_line::<i32>(&mut input).unwrap();
        assert_eq!(result, 123);

        let mut input_non_numeric = "abc\n";
        let result_non_numeric = verify_line_parses_to::<i32>(&mut input_non_numeric);
        assert!(result_non_numeric.is_err());

        let mut input_empty = "\n";
        let result_empty = verify_line_parses_to::<i32>(&mut input_empty);
        assert!(result_empty.is_err());
    }

    #[test]
    fn test_num_bool() {
        let mut input_false = "0";
        let result_false = num_bool(&mut input_false).unwrap();
        assert!(!result_false);

        let mut input_true = "1";
        let result_true = num_bool(&mut input_true).unwrap();
        assert!(result_true);

        let mut input_invalid = "2";
        assert!(num_bool(&mut input_invalid).is_err());
    }

    #[test]
    fn test_num_bool_line() {
        let mut input_false = "0\n";
        let result_false = num_bool_line(&mut input_false).unwrap();
        assert!(!result_false);

        let mut input_true = "1\n";
        let result_true = num_bool_line(&mut input_true).unwrap();
        assert!(result_true);

        let mut input_invalid = "2\n";
        assert!(num_bool_line(&mut input_invalid).is_err());

        let mut input_newline = "\n";
        assert!(num_bool_line(&mut input_newline).is_err());
    }
}
