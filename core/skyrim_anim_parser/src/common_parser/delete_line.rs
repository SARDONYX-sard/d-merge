use winnow::{
    ascii::{line_ending, space0, Caseless},
    combinator::delimited,
    error::{StrContext, StrContextValue},
    ModalResult, Parser,
};

/// Parse `//* delete this line *//`
/// # Errors
/// Parse failed.
pub(crate) fn delete_this_line(input: &mut &str) -> ModalResult<()> {
    let start_comment = ("//*", space0);
    let end_comment = (space0, "*//");
    let comment_parser = delimited(
        start_comment,
        winnow::seq! {
            Caseless("delete"),
            space0,
            Caseless("this"),
            space0,
            Caseless("line"),

        },
        end_comment,
    );

    (comment_parser, line_ending)
        .context(StrContext::Expected(StrContextValue::StringLiteral(
            "//* delete this line *//",
        )))
        .parse_next(input)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_comment_kind() {
        let input = "//* delete this line *//
";

        delete_this_line
            .parse(input)
            .unwrap_or_else(|e| panic!("{e}"));
    }
}
