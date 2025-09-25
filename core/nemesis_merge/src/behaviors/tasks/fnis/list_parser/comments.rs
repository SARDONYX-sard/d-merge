//! Line comments parsing (`' comment`)

use winnow::ascii::{line_ending, till_line_ending};
use winnow::combinator::{preceded, repeat, terminated};
use winnow::error::{StrContext, StrContextValue};
use winnow::{ModalResult, Parser};

/// Parse zero or more comment lines starting with `'`
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
