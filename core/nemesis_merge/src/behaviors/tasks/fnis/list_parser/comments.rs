//! Line comments parsing (`' comment`)

use winnow::ascii::multispace0;
use winnow::ascii::till_line_ending;
use winnow::combinator::{preceded, repeat, terminated};
use winnow::error::{StrContext, StrContextValue};
use winnow::{ModalResult, Parser};

/// Parse zero or more comment lines starting with `'`
pub fn line_comments0(input: &mut &str) -> ModalResult<()> {
    fn line_comment<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
        terminated(preceded('\'', till_line_ending), multispace0)
            .context(StrContext::Label("Comment Line"))
            .context(StrContext::Expected(StrContextValue::Description(
                "e.g. `' Any String`",
            )))
            .parse_next(input)
    }
    multispace0.parse_next(input)?;
    let _: () = repeat(0.., line_comment).parse_next(input)?;
    Ok(())
}
