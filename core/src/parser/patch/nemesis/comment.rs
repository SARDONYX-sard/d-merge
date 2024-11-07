use crate::parser::patch::helpers::delimited_multispace0;
use winnow::{
    ascii::Caseless,
    combinator::{alt, delimited, terminated},
    error::{StrContext, StrContextValue},
    token::take_until,
    PResult, Parser,
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

pub(crate) fn close_parser<'a>(input: &mut &'a str) -> PResult<&'a str> {
    delimited_multispace0(Caseless("CLOSE")).parse_next(input)
}

pub(crate) fn take_till_close<'a>(input: &mut &'a str) -> PResult<&'a str> {
    // NOTE: The comment `<! -- UNKNOWN BITS -->` in hkFlags,
    //       so the only way is to match the comment exactly.
    terminated(
        take_until(0.., "<!-- CLOSE -->"),
        Caseless("<!-- CLOSE -->"),
    )
    .parse_next(input)
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
            close_comment.parse("<!--CLOSE  -->"),
            Ok(CommentKind::Close)
        );

        assert_eq!(
            close_comment.parse("<!-- memSizeAndFlags SERIALIZE_IGNORED -->"),
            Ok(CommentKind::Unknown("memSizeAndFlags SERIALIZE_IGNORED"))
        );
    }
}
