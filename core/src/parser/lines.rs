use core::str::FromStr;
use winnow::{
    ascii::{line_ending, till_line_ending},
    combinator::alt,
    error::{ContextError, StrContext::*, StrContextValue::*},
    PResult, Parser,
};

/// Parse 1 line.
pub(crate) fn one_line<'a>(input: &mut &'a str) -> PResult<&'a str> {
    let line = till_line_ending.parse_next(input)?;
    line_ending.parse_next(input)?; // skip line end
    Ok(line)
}

pub(crate) fn lines<'a>(read_len: usize) -> impl Parser<&'a str, Vec<&'a str>, ContextError> {
    move |input: &mut &'a str| {
        let mut lines = vec![];
        for _ in 0..read_len {
            lines.push(one_line.parse_next(input)?);
        }
        Ok(lines)
    }
}

/// Parse one line and then parse to T.
#[inline]
pub(crate) fn from_one_line<T: FromStr>(input: &mut &str) -> PResult<T> {
    one_line.parse_to().parse_next(input)
}

/// - `'0'` => `false`
/// - `'1'` => `true`
fn num_bool(input: &mut &str) -> PResult<bool> {
    alt(('0'.value(false), '1'.value(true)))
        .context(Expected(CharLiteral('0')))
        .context(Expected(CharLiteral('1')))
        .parse_next(input)
}

/// - `'0'` => `false`
/// - `'1'` => `true`
pub(crate) fn num_bool_line(input: &mut &str) -> PResult<bool> {
    let boolean = num_bool.parse_next(input)?;
    line_ending.parse_next(input)?; // skip line end
    Ok(boolean)
}
