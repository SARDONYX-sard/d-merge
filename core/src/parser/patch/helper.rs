use simd_json::BorrowedValue;
use winnow::{
    ascii::{multispace0, Caseless},
    combinator::{alt, delimited, trace},
    error::{ParserError, StrContext, StrContextValue},
    stream::{AsChar, Stream, StreamIsPartial},
    token::{take_till, take_until},
    PResult, Parser,
};

pub fn delimited_multispace0<Input, Output, Error, ParseNext>(
    mut parser: ParseNext,
) -> impl Parser<Input, Output, Error>
where
    Input: StreamIsPartial + Stream,
    Error: ParserError<Input>,
    ParseNext: Parser<Input, Output, Error>,
    <Input as Stream>::Token: AsChar + Clone,
{
    trace("delimited_multispace0", move |input: &mut Input| {
        let _ = multispace0.parse_next(input)?;
        let o2 = parser.parse_next(input)?;
        multispace0.parse_next(input).map(|_| o2)
    })
}

/// Parse `#0000`, `#0500`
/// # Errors
/// Parse failed.
pub fn pointer<'a>(input: &mut &'a str) -> PResult<BorrowedValue<'a>> {
    alt((
        "null".value(BorrowedValue::String("#0000".into())),
        // '\n', '\t', ' ' => Array elements
        // `<` => end tag of array or field
        take_till(0.., |c| matches!(c, '\n' | '\t' | ' ' | '<'))
            .map(|s: &str| BorrowedValue::String(s.into())),
    ))
    .context(StrContext::Expected(StrContextValue::Description(
        r#"Pointer(e.g. `#0050`)"#,
    )))
    .parse_next(input)
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum CommentKind<'a> {
    ModCode(&'a str),
    Original,
    Close,
    Unknown(&'a str),
}

/// # Errors
/// Parse failed.
pub(crate) fn comment_kind<'a>(input: &mut &'a str) -> PResult<CommentKind<'a>> {
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
pub(crate) fn close_comment<'a>(input: &mut &'a str) -> PResult<CommentKind<'a>> {
    let kind_parser = {
        let original_parser = delimited_multispace0(Caseless("ORIGINAL"));
        let close_parser = delimited_multispace0(Caseless("CLOSE"));

        alt((
            original_parser.value(CommentKind::Original),
            close_parser.value(CommentKind::Close),
        ))
    };
    let comment_parser = delimited("<!--", kind_parser, "-->");

    delimited_multispace0(comment_parser)
        .context(StrContext::Expected(StrContextValue::Description(
            "Comment(e.g. `<!-- ORIGINAL -->`, `<!-- CLOSE -->`)",
        )))
        .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pointer() {
        assert_eq!(
            pointer.parse_next(&mut "#0000 #0000"),
            Ok(BorrowedValue::String("#0000".into()))
        );

        assert_eq!(
            pointer.parse_next(&mut "$turn$12</hkparam>"),
            Ok(BorrowedValue::String("$turn$12".into()))
        );
    }

    #[test]
    fn test_comment_kind() {
        assert_eq!(
            comment_kind.parse("<!-- MOD_CODE ~hi!~ OPEN -->"),
            Ok(CommentKind::ModCode("hi!"))
        );

        assert_eq!(
            comment_kind.parse("<!-- ORIGINAL -->"),
            Ok(CommentKind::Original)
        );

        assert_eq!(comment_kind.parse("<!-- CLOSE -->"), Ok(CommentKind::Close));
        assert_eq!(comment_kind.parse("<!--CLOSE  -->"), Ok(CommentKind::Close));

        assert_eq!(
            comment_kind.parse("<!-- memSizeAndFlags SERIALIZE_IGNORED -->"),
            Ok(CommentKind::Unknown(" memSizeAndFlags SERIALIZE_IGNORED "))
        );
    }
}
