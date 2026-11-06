//! The comments that mark a changed block of a patch.
//!
//! ```text
//! <!-- MOD_CODE ~id~ OPEN -->     CommentKind::ModCode("id")
//! <!-- ORIGINAL -->               CommentKind::Original
//! <!-- CLOSE -->                  CommentKind::Close
//! <!-- x SERIALIZE_IGNORED -->    CommentKind::Unknown("x SERIALIZE_IGNORED")
//! ```

use winnow::{
    ModalResult, Parser,
    ascii::{Caseless, line_ending, multispace0},
    combinator::{alt, delimited, opt, terminated, trace},
    error::{ParserError, StrContext, StrContextValue},
    stream::{AsChar, Stream, StreamIsPartial},
    token::take_until,
};
use winnow_ext::take_until_ext;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum CommentKind<'a> {
    ModCode(&'a str),
    Original,
    Close,
    Unknown(&'a str),
}

/// `~id~ OPEN` with `MOD_CODE` before it: the id.
fn mod_code<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
    let id = delimited('~', take_until(0.., '~'), '~');
    delimited_multispace0(delimited(Caseless("MOD_CODE"), delimited_multispace0(id), Caseless("OPEN")))
        .parse_next(input)
}

/// `<!-- MOD_CODE ~<mod code>~ OPEN -->`
///
/// # Errors
/// Parse failed.
pub fn open_comment<'a>(input: &mut &'a str) -> ModalResult<CommentKind<'a>> {
    let comment = delimited("<!--", mod_code.map(CommentKind::ModCode), "-->");
    delimited_multispace0(comment)
        .context(StrContext::Label("Open diff comment"))
        .context(StrContext::Expected(StrContextValue::Description("<!-- MOD_CODE ~id~ OPEN -->")))
        .parse_next(input)
}

/// `<!-- MOD_CODE ~<mod code>~ OPEN -->`, or any other comment as [`CommentKind::Unknown`].
///
/// # Errors
/// Parse failed.
pub fn mod_code_or_other_comment<'a>(input: &mut &'a str) -> ModalResult<CommentKind<'a>> {
    let kind = alt((mod_code.map(CommentKind::ModCode), take_until(0.., "-->").map(CommentKind::Unknown)));
    delimited_multispace0(delimited("<!--", kind, "-->"))
        .context(StrContext::Expected(StrContextValue::Description(
            "Comment(e.g. `<!-- MOD_CODE ~id~ OPEN -->`, `<!-- ORIGINAL -->`, `<!-- CLOSE -->`)",
        )))
        .parse_next(input)
}

/// `<!-- ORIGINAL -->`, `<!-- CLOSE -->`, or a `SERIALIZE_IGNORED` one.
///
/// # Errors
/// Parse failed.
pub fn original_or_close_comment<'a>(input: &mut &'a str) -> ModalResult<CommentKind<'a>> {
    let kind = {
        let original = delimited_multispace0(Caseless("ORIGINAL"));
        let ignored = delimited_multispace0((take_until(0.., "SERIALIZE_IGNORED"), "SERIALIZE_IGNORED").take());
        alt((
            original.value(CommentKind::Original),
            close_parser.value(CommentKind::Close),
            ignored.map(CommentKind::Unknown),
        ))
    };
    delimited_multispace0(delimited("<!--", kind, "-->"))
        .context(StrContext::Expected(StrContextValue::Description(
            "Comment(e.g. `<!-- ORIGINAL -->`, `<!-- CLOSE -->`)",
        )))
        .parse_next(input)
}

/// `CLOSE`, the inside of its comment.
///
/// # Errors
/// Parse failed.
pub fn close_parser<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
    delimited_multispace0(Caseless("CLOSE")).parse_next(input)
}

/// `<!-- CLOSE -->` and its line end.
///
/// # Errors
/// Parse failed.
pub fn close_comment_line(input: &mut &str) -> ModalResult<()> {
    delimited("<!--", close_parser, "-->").parse_next(input)?;
    line_ending.parse_next(input)?;
    Ok(())
}

/// Up to `<!-- ORIGINAL -->`, which is passed with the whitespace after it.
///
/// # Errors
/// Parse failed.
pub fn take_till_original<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
    let original = delimited_multispace0(Caseless("ORIGINAL"));
    terminated(
        take_until_ext(0.., delimited("<!--", original.value(CommentKind::Original), "-->")).take(),
        (Caseless("<!-- ORIGINAL -->"), multispace0),
    )
    .context(StrContext::Expected(StrContextValue::Description("Comment(e.g. `<!-- ORIGINAL -->`)")))
    .parse_next(input)
}

/// Up to `<!-- CLOSE -->`, which is passed with the whitespace after it.
///
/// # Errors
/// Parse failed.
pub fn take_till_close<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
    // NOTE: The comment `<! -- UNKNOWN BITS -->` in hkFlags,
    //       so the only way is to match the comment exactly.
    terminated(
        take_until_ext(0.., delimited("<!--", close_parser.value(CommentKind::Close), "-->")),
        (Caseless("<!-- CLOSE -->"), multispace0),
    )
    .context(StrContext::Expected(StrContextValue::Description("Comment(e.g. `<!-- CLOSE -->`)")))
    .parse_next(input)
}

/// Up to `<!-- CLOSE -->` written exactly so, which is passed. The whitespace after it stays.
///
/// # Errors
/// Parse failed.
pub fn take_till_exact_close<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
    terminated(take_until_ext(0.., Caseless("<!-- CLOSE -->")), Caseless("<!-- CLOSE -->")).parse_next(input)
}

/// The changed block that starts here, if one does: `(its lines, how many lines it has after ORIGINAL)`.
/// The parser goes on with the lines after `ORIGINAL`, which are the template's.
///
/// This does not interpret the block's lines.
///
/// # Errors
/// A block does not end.
pub fn take_raw_block<'a>(input: &mut &'a str) -> ModalResult<Option<(&'a str, Option<usize>)>> {
    // Fast path: no OPEN comment ahead
    if opt(open_comment).parse_next(input)?.is_none() {
        return Ok(None);
    }

    // take_till_original may overshoot into the *next* diff's ORIGINAL
    // when this diff has no ORIGINAL of its own
    // (pattern: OPEN -> CLOSE -> OPEN -> ORIGINAL).
    // Comparing remaining lengths tells us which marker truly comes first.
    let (original_remain, original_found) = opt(take_till_original).parse_peek(input)?;
    let (close_remain, close_found) = opt(take_till_close).parse_peek(input)?;

    let has_original = match (original_found, close_found) {
        // ORIGINAL belongs to this diff only if reaching it consumed
        // *less* input than reaching CLOSE did, i.e. it comes first.
        (Some(_), Some(_)) => original_remain.len() > close_remain.len(),
        (Some(_), None) => true,
        _ => false,
    };

    if has_original {
        let block = take_till_original.parse_next(input)?;
        let (_, original) = take_till_close.parse_peek(input)?;
        let original_len = original.lines().filter(|line| !line.trim().is_empty()).count();
        return Ok(Some((block, Some(original_len))));
    }
    Ok(Some((take_till_close.parse_next(input)?, None)))
}

/// `parser` with any whitespace on both sides.
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
        let output = parser.parse_next(input)?;
        multispace0.parse_next(input).map(|_| output)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tells_the_comments() {
        assert_eq!(open_comment.parse("<!-- MOD_CODE ~hi!~ OPEN -->"), Ok(CommentKind::ModCode("hi!")));
        assert_eq!(mod_code_or_other_comment.parse("<!-- MOD_CODE ~hi!~ OPEN -->"), Ok(CommentKind::ModCode("hi!")));
        assert_eq!(mod_code_or_other_comment.parse("<!-- other -->"), Ok(CommentKind::Unknown(" other ")));
        assert_eq!(original_or_close_comment.parse("<!-- ORIGINAL -->"), Ok(CommentKind::Original));
        assert_eq!(original_or_close_comment.parse("<!-- CLOSE -->"), Ok(CommentKind::Close));
        assert_eq!(
            original_or_close_comment.parse("<!-- memSizeAndFlags SERIALIZE_IGNORED -->"),
            Ok(CommentKind::Unknown("memSizeAndFlags SERIALIZE_IGNORED"))
        );
    }

    #[test]
    fn takes_up_to_the_close() {
        let mut input = "Some data here\n<!-- something -->\nMore data\n<!-- CLoSE -->\n After";
        take_till_close.parse_next(&mut input).unwrap();
        assert_eq!(input, "After");

        let mut input = "data<!-- CLOSE -->\n After";
        assert_eq!(take_till_exact_close.parse_next(&mut input), Ok("data"));
        assert_eq!(input, "\n After");
    }

    fn block_of(mut patch: &str) -> (Option<(&str, Option<usize>)>, &str) {
        let block = take_raw_block(&mut patch).unwrap();
        (block, patch)
    }

    #[test]
    fn a_block_has_its_lines_and_its_original_len() {
        assert_eq!(block_of("no block").0, None);

        let (block, rest) =
            block_of("<!-- MOD_CODE ~m~ OPEN -->\nnew\n<!-- ORIGINAL -->\nold1\n\nold2\n<!-- CLOSE -->\nrest");
        assert_eq!(block, Some(("new\n", Some(2))));
        assert!(rest.starts_with("old1"), "the template's lines are to be parsed: {rest}");
    }

    /// `OPEN` -> `CLOSE` -> `OPEN` -> `ORIGINAL`: that `ORIGINAL` is not of the first block.
    #[test]
    fn the_original_of_the_next_block_is_not_this_ones() {
        let patch = "<!-- MOD_CODE ~m~ OPEN -->\nnew\n<!-- CLOSE -->\na\n<!-- MOD_CODE ~m~ OPEN -->\nb\n<!-- ORIGINAL -->\nc\n<!-- CLOSE -->\n";
        let (block, rest) = block_of(patch);
        assert_eq!(block, Some(("new\n", None)));
        assert!(rest.starts_with("a\n"), "{rest}");
    }

    #[test]
    fn takes_up_to_the_original() {
        let mut input = "new\n<!-- ORIGINAL -->\nold";
        assert_eq!(take_till_original.parse_next(&mut input), Ok("new\n"));
        assert_eq!(input, "old");
    }
}
