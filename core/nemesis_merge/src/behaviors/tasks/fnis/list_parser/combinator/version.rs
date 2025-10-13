//! Version line parsing: `Version V<n>.<m>`

use winnow::ascii::{digit1, space0, Caseless};
use winnow::combinator::{opt, preceded, seq};
use winnow::error::{StrContext, StrContextValue};
use winnow::{ModalResult, Parser};

use crate::behaviors::tasks::fnis::list_parser::combinator::comment::skip_ws_and_comments;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
}

/// Version line parsing: `Version V<n>.<m>`
pub fn parse_version_line(input: &mut &str) -> ModalResult<Version> {
    seq! {
        Version {
            _: Caseless("Version"),
            _: space0,
            _: opt(Caseless("v")),
            _: space0,
            major: digit1.parse_to(),
            _: space0,
            minor: opt(preceded(".", digit1.parse_to())).map(|n| n.unwrap_or(0)),
            _: skip_ws_and_comments,
        }
    }
    .context(StrContext::Label("Version"))
    .context(StrContext::Expected(StrContextValue::Description(
        "Format: Version V<n>.<m> (e.g. Version V7.3)",
    )))
    .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behaviors::tasks::fnis::list_parser::test_helpers::{must_fail, must_parse};

    #[test]
    fn test_parse_version_line_ok() {
        let parsed = must_parse(parse_version_line, "Version 7.2\n");
        assert_eq!(parsed.major, 7);
        assert_eq!(parsed.minor, 2);
    }

    #[test]
    fn test_parse_version_line_invalid() {
        must_fail(parse_version_line, "Ver 7.2\n");
    }
}
