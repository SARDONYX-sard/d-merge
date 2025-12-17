use winnow::ascii::{line_ending, till_line_ending};
use winnow::error::{StrContext, StrContextValue};
use winnow::{combinator::opt, Parser as _};

use crate::common_parser::comment::take_till_original;

pub(crate) fn version_v3<'a>(input: &mut &'a str) -> winnow::ModalResult<&'a str> {
    let line = till_line_ending
        .verify(|s: &str| s.trim().eq_ignore_ascii_case("V3"))
        .context(StrContext::Label("Version"))
        .context(StrContext::Expected(StrContextValue::StringLiteral("V3")))
        .parse_next(input)?;
    line_ending.parse_next(input)?; // skip line end
    Ok(line)
}

/// If the next token is a Nemesis diff block, consume it entirely
/// and return the raw source slice.
///
/// This function does NOT attempt to interpret the diff content.
/// It only advances the input cursor.
///
/// # Returns
/// diff, has original
pub(crate) fn take_raw_diff<'a>(
    input: &mut &'a str,
) -> winnow::ModalResult<Option<(&'a str, bool)>> {
    use crate::common_parser::comment::{open_comment, take_till_close};

    // Fast path: no OPEN comment ahead
    if opt(open_comment).parse_next(input)?.is_none() {
        return Ok(None);
    }

    #[cfg(feature = "tracing")]
    tracing::debug!("Open diff");

    let (diff, has_original) = if let Some(diff) = opt(take_till_original).parse_next(input)? {
        let (_remain, original) = opt(take_till_close).parse_peek(input)?;
        (diff, original.is_some())
    } else {
        let diff = take_till_close.parse_next(input)?;
        (diff, false)
    };

    #[cfg(feature = "tracing")]
    tracing::debug!(?diff, has_original);
    Ok(Some((diff, has_original)))
}
