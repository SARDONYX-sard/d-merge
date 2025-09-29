//! `FNIS_<mod name>_List.txt` parser
//!
//! See `FNIS for Modders_V6.2.pdf` by fore
pub mod combinator;
pub mod patterns;

/// Common test helpers
#[cfg(test)]
pub mod test_helpers {
    use winnow::error::{ContextError, ErrMode};
    use winnow::Parser;

    /// Must successfully parse or panic
    pub fn must_parse<'a, O>(
        mut parser: impl Parser<&'a str, O, ErrMode<ContextError>>,
        input: &'a str,
    ) -> O {
        parser
            .parse(input)
            .unwrap_or_else(|e| panic!("ERROR:\n{e}"))
    }

    /// Must fail to parse or panic
    pub fn must_fail<'a, O>(
        mut parser: impl Parser<&'a str, O, ErrMode<ContextError>>,
        input: &'a str,
    ) {
        if parser.parse(input).is_ok() {
            panic!("[Must fail!] expected parse to fail, but got OK");
        }
    }
}
