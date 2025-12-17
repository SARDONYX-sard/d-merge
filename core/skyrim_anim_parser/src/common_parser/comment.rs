use winnow::{
    ascii::{line_ending, multispace0, Caseless},
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

/// `MOD_CODE ~<mod code>~ OPEN`
///
/// # Errors
/// Parse failed.
pub(crate) fn open_comment<'a>(input: &mut &'a str) -> ModalResult<CommentKind<'a>> {
    let kind_parser = {
        let mod_code_parser = {
            let id_parser = delimited('~', take_until(0.., '~'), '~');
            delimited(
                Caseless("MOD_CODE"),
                delimited_multispace0(id_parser),
                Caseless("OPEN"),
            )
        };
        delimited_multispace0(mod_code_parser).map(CommentKind::ModCode)
    };
    let comment_parser = delimited("<!--", kind_parser, "-->");

    delimited_multispace0(comment_parser)
        .context(StrContext::Label("Open diff comment"))
        .context(StrContext::Expected(StrContextValue::Description(
            "<!-- MOD_CODE ~id~ OPEN -->",
        )))
        .parse_next(input)
}

/// `ORIGINAL`
///
/// # Errors
/// Parse failed.
pub(crate) fn take_till_original<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
    let original_parser = delimited_multispace0(Caseless("ORIGINAL"));

    terminated(
        take_until_ext(
            0..,
            delimited("<!--", original_parser.value(CommentKind::Original), "-->"),
        )
        .take(),
        (Caseless("<!-- ORIGINAL -->"), multispace0),
    )
    .context(StrContext::Expected(StrContextValue::Description(
        "Comment(e.g. `<!-- ORIGINAL -->`)",
    )))
    .parse_next(input)
}

/// ORIGINAL or CLOSE
/// # Errors
/// Parse failed.
pub(crate) fn original_or_close_comment<'a>(input: &mut &'a str) -> ModalResult<CommentKind<'a>> {
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

pub(crate) fn close_comment_line(input: &mut &str) -> ModalResult<()> {
    delimited("<!--", close_parser, "-->").parse_next(input)?;
    line_ending.parse_next(input)?;
    Ok(())
}

/// take until `<!-- CLOSE -->` & multispace0 (trim close comment.)
pub(crate) fn take_till_close<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
    // NOTE: The comment `<! -- UNKNOWN BITS -->` in hkFlags,
    //       so the only way is to match the comment exactly.
    terminated(
        take_until_ext(
            0..,
            delimited("<!--", close_parser.value(CommentKind::Close), "-->"),
        ),
        (Caseless("<!-- CLOSE -->"), multispace0),
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

/// take_until implementation using only winnow
pub fn take_until_ext<Input, Output, Error, ParseNext>(
    occurrences: impl Into<winnow::stream::Range>,
    parser: ParseNext,
) -> impl Parser<Input, Input::Slice, Error>
where
    Input: winnow::stream::StreamIsPartial + winnow::stream::Stream,
    Error: winnow::error::ParserError<Input>,
    ParseNext: Parser<Input, Output, Error>,
{
    use winnow::combinator::{not, peek, repeat, trace};
    use winnow::token::any;

    trace(
        "take_until_ext",
        repeat::<_, _, (), _, _>(occurrences, (peek(not(parser)), any)).take(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_comment_kind() {
        assert_eq!(
            open_comment.parse("<!-- MOD_CODE ~hi!~ OPEN -->"),
            Ok(CommentKind::ModCode("hi!"))
        );

        assert_eq!(
            original_or_close_comment.parse("<!-- ORIGINAL -->"),
            Ok(CommentKind::Original)
        );

        assert_eq!(
            original_or_close_comment.parse("<!-- CLOSE -->"),
            Ok(CommentKind::Close)
        );

        assert_eq!(
            original_or_close_comment.parse("<!-- memSizeAndFlags SERIALIZE_IGNORED -->"),
            Ok(CommentKind::Unknown("memSizeAndFlags SERIALIZE_IGNORED"))
        );
    }

    #[test]
    fn test_take_till_close_basic() {
        let mut input = "\
Some data here
<!-- something -->
More data
<!-- CLoSE -->After";
        take_till_close.parse_next(&mut input).unwrap();
        assert_eq!(input, "After");
    }
}
