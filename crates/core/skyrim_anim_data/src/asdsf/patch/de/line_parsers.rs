use winnow::{
    Parser as _,
    ascii::{line_ending, till_line_ending},
    error::{StrContext, StrContextValue},
};

pub(crate) fn version_v3<'a>(input: &mut &'a str) -> winnow::ModalResult<&'a str> {
    let line = till_line_ending
        .verify(|s: &str| s.trim().eq_ignore_ascii_case("V3"))
        .context(StrContext::Label("Version"))
        .context(StrContext::Expected(StrContextValue::StringLiteral("V3")))
        .parse_next(input)?;
    line_ending.parse_next(input)?; // skip line end
    Ok(line)
}
