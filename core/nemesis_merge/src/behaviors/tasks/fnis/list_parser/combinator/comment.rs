//! Line comments parsing (`' comment`)

use winnow::ascii::{line_ending, space0, till_line_ending};
use winnow::combinator::{alt, preceded, repeat, terminated};
use winnow::error::{StrContext, StrContextValue};
use winnow::{ModalResult, Parser};

pub fn comment_line0(input: &mut &str) -> ModalResult<()> {
    let _: () = repeat(0.., comment_line).parse_next(input)?;
    Ok(())
}

fn comment_line<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
    terminated(preceded('\'', till_line_ending), line_ending)
        .context(StrContext::Label("Comment Line"))
        .context(StrContext::Expected(StrContextValue::Description(
            "e.g. `' Any String`",
        )))
        .parse_next(input)
}

/// space 0 or more, opt(comment) line ending
pub fn comment_line_ending<'a>(input: &mut &'a str) -> ModalResult<()> {
    (space0, alt((comment_line, line_ending))).parse_next(input)?;
    Ok(())
}
