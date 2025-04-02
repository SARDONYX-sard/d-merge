use simd_json::BorrowedValue;
use winnow::{
    combinator::alt,
    error::{StrContext, StrContextValue},
    token::take_till,
    ModalResult, Parser,
};

/// Parse `#0000`, `#0500`
/// # Errors
/// Parse failed.
pub fn pointer<'a>(input: &mut &'a str) -> ModalResult<BorrowedValue<'a>> {
    alt((
        "null".value(BorrowedValue::String("#0000".into())),
        // '\n', '\t', ' ' => Array elements
        // `<` => end tag of array or field
        take_till(0.., |c| matches!(c, '\r' | '\n' | '\t' | ' ' | '<'))
            .map(|s: &str| BorrowedValue::String(s.trim().into())), // Double cut off because winnow doesn't omit `\r` in release builds for some reason.
    ))
    .context(StrContext::Expected(StrContextValue::Description(
        r#"Pointer(e.g. `#0050`)"#,
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
            pointer.parse_next(&mut "$turn$12\r\n</hkparam>"),
            Ok(BorrowedValue::String("$turn$12".into()))
        );
    }
}
