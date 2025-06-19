use winnow::{
    ascii::Caseless,
    combinator::{alt, delimited, terminated},
    error::{StrContext, StrContextValue},
    token::take_until,
    ModalResult, Parser,
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum CommentKind<'a> {
    ModCode(&'a str),
    Original,
    Close,
    Unknown(&'a str),
}

/// # Errors
/// Parse failed.
pub(crate) fn comment_kind<'a>(input: &mut &'a str) -> ModalResult<CommentKind<'a>> {
    let kind_parser = {
        let mod_code_parser = {
            let id_parser = delimited('~', take_until(0.., '~'), '~');
            delimited(
                Caseless("MOD_CODE"),
                delimited_multispace0(id_parser),
                Caseless("OPEN"),
            )
        };

        let mod_code_parser = delimited_multispace0(mod_code_parser);
        // let original_parser = delimited_multispace0(Caseless("ORIGINAL"));
        // let close_parser = delimited_multispace0(Caseless("CLOSE"));

        alt((
            mod_code_parser.map(CommentKind::ModCode),
            // original_parser.value(CommentKind::Original),
            // close_parser.value(CommentKind::Close),
            take_until(0.., "-->").map(CommentKind::Unknown),
        ))
    };
    let comment_parser = delimited("<!--", kind_parser, "-->");

    delimited_multispace0(comment_parser)
        .context(StrContext::Expected(StrContextValue::Description(
            "Comment(e.g. `<!-- MOD_CODE ~id~ OPEN -->`, `<!-- ORIGINAL -->`, `<!-- CLOSE -->`)",
        )))
        .parse_next(input)
}

/// ORIGINAL or CLOSE
/// # Errors
/// Parse failed.
pub(crate) fn close_comment<'a>(input: &mut &'a str) -> ModalResult<CommentKind<'a>> {
    let kind_parser = {
        let original_parser = delimited_multispace0(Caseless("ORIGINAL"));

        let ignore_parser = delimited_multispace0(
            (take_until(0.., "SERIALIZE_IGNORED"), "SERIALIZE_IGNORED").take(),
        );

        alt((
            original_parser.value(CommentKind::Original),
            close_parser.value(CommentKind::Close),
            ignore_parser.map(CommentKind::Unknown),
        ))
    };
    let comment_parser = delimited("<!--", kind_parser, "-->");

    delimited_multispace0(comment_parser)
        .context(StrContext::Expected(StrContextValue::Description(
            "Comment(e.g. `<!-- ORIGINAL -->`, `<!-- CLOSE -->`)",
        )))
        .parse_next(input)
}

pub(crate) fn close_parser<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
    delimited_multispace0(Caseless("CLOSE")).parse_next(input)
}

pub(crate) fn take_till_close<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
    // NOTE: The comment `<! -- UNKNOWN BITS -->` in hkFlags,
    //       so the only way is to match the comment exactly.
    terminated(
        take_until(0.., "<!-- CLOSE -->"),
        Caseless("<!-- CLOSE -->"),
    )
    .context(StrContext::Expected(StrContextValue::Description(
        "Comment(e.g. `<!-- CLOSE -->`)",
    )))
    .parse_next(input)
}

pub fn delimited_multispace0<Input, Output, Error, ParseNext>(
    mut parser: ParseNext,
) -> impl Parser<Input, Output, Error>
where
    Input: winnow::stream::StreamIsPartial + winnow::stream::Stream,
    Error: winnow::error::ParserError<Input>,
    ParseNext: Parser<Input, Output, Error>,
    <Input as winnow::stream::Stream>::Token: winnow::stream::AsChar + Clone,
{
    use winnow::ascii::multispace0;

    winnow::combinator::trace("delimited_multispace0", move |input: &mut Input| {
        let _ = multispace0.parse_next(input)?;
        let o2 = parser.parse_next(input)?;
        multispace0.parse_next(input).map(|_| o2)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_comment_kind() {
        assert_eq!(
            comment_kind.parse("<!-- MOD_CODE ~hi!~ OPEN -->"),
            Ok(CommentKind::ModCode("hi!"))
        );

        assert_eq!(
            close_comment.parse("<!-- ORIGINAL -->"),
            Ok(CommentKind::Original)
        );

        assert_eq!(
            close_comment.parse("<!-- CLOSE -->"),
            Ok(CommentKind::Close)
        );

        assert_eq!(
            close_comment.parse("<!-- memSizeAndFlags SERIALIZE_IGNORED -->"),
            Ok(CommentKind::Unknown("memSizeAndFlags SERIALIZE_IGNORED"))
        );
    }
}
