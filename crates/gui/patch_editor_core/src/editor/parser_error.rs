//! What d-merge's parsers rejected in a patch, and where: for the editor's underline and the Problems
//! panel.
//!
//! The parsers' errors carry a [`ReadableError`], whose place is taken as it is (no rendered report is
//! read back). Each parser has an error type of its own; `From` makes one [`ParserError`] of any:
//!
//! | Parser                                   | Its error                                   |
//! | ---------------------------------------- | ------------------------------------------- |
//! | a behavior's patch (`nemesis_xml`)       | `nemesis_xml::error::Error`                 |
//! | clip data's anim / motion / header diffs | `skyrim_anim_data::adsf::patch::de::error::Error` |
//! | a new clip / motion                      | [`ReadableError`]                           |
//! | the project lists (`diff_line`)          | `skyrim_anim_data::diff_line::error::Error` |
//! | an anim set's diff                       | `skyrim_anim_data::asdsf::patch::de::error::Error` |
//! | a whole new anim set                     | `winnow`'s `ParseError`                     |
//!
//! ```text
//! 3 |   <hkparam name="typoField">null</hkparam>
//!   |   - Unknown field names: "typoField".
//! ```
//! -> `ParserError { at: Some((2, 2)), message: "Unknown field names: \"typoField\"." }`

use core::fmt;

use skyrim_anim_data::{
    adsf::patch::de::error::Error as AdsfError, asdsf::patch::de::error::Error as AsdsfError,
    diff_line::error::Error as LinesError,
};
use winnow::error::{ContextError, ParseError};
use winnow_ext::ReadableError;

/// See the module docs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParserError {
    /// `(line, column)` in the patch, both 0-based, the column in chars. `None`: the error says no
    /// place (e.g. a file could not be read, an export's own check).
    pub at: Option<(usize, usize)>,
    /// e.g. `expected </hkparam>`
    pub message: String,
}

impl ParserError {
    /// An error that says no place.
    #[inline]
    pub fn without_place(message: impl Into<String>) -> Self {
        Self { at: None, message: message.into() }
    }

    /// The place and message of `error`. One with nothing at its place says its title (e.g.
    /// `Parse error`): a problem must not be listed without a word.
    pub fn of_readable(error: &ReadableError) -> Self {
        let message = match error.message().trim() {
            "" => error.title().trim(),
            message => message,
        };
        Self { at: Some(error.line_column()), message: message.to_owned() }
    }
}

/// `3:5: expected </hkparam>` (1-based, as an editor's status line), or the message alone. For the
/// lists of rejected patches and the preview's hover.
impl fmt::Display for ParserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.at {
            Some((line, column)) => write!(f, "{}:{}: {}", line + 1, column + 1, self.message),
            None => f.write_str(&self.message),
        }
    }
}

impl From<ReadableError> for ParserError {
    #[inline]
    fn from(error: ReadableError) -> Self {
        Self::of_readable(&error)
    }
}

impl From<ParseError<&str, ContextError>> for ParserError {
    #[inline]
    fn from(error: ParseError<&str, ContextError>) -> Self {
        Self::of_readable(&ReadableError::from_parse(error))
    }
}

impl From<nemesis_xml::error::Error> for ParserError {
    fn from(error: nemesis_xml::error::Error) -> Self {
        match error {
            nemesis_xml::error::Error::ReadableError { source } => Self::of_readable(&source),
            other => Self::without_place(other.to_string()),
        }
    }
}

impl From<AdsfError> for ParserError {
    fn from(error: AdsfError) -> Self {
        match error {
            AdsfError::Readable { source } => Self::of_readable(&source),
            other => Self::without_place(other.to_string()),
        }
    }
}

impl From<AsdsfError> for ParserError {
    fn from(error: AsdsfError) -> Self {
        match error {
            AsdsfError::ReadableError { source } => Self::of_readable(&source),
            other => Self::without_place(other.to_string()),
        }
    }
}

impl From<LinesError> for ParserError {
    fn from(error: LinesError) -> Self {
        match error {
            LinesError::Readable { source } => Self::of_readable(&source),
            other => Self::without_place(other.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The module docs' example: the place is the parser's own, not read from its report.
    #[test]
    fn a_behavior_patch_error_has_its_place() {
        let patch = "<hkobject name=\"#0001\" class=\"hkbModifierList\" signature=\"0xa4180ca1\">\n  <hkparam name=\"typoField\">null</hkparam>\n</hkobject>\n";
        let error = ParserError::from(nemesis_xml::patch::parse_nemesis_patch(patch, None).unwrap_err());
        assert_eq!(error.at.map(|(line, _)| line), Some(1), "{error}");
        assert!(error.message.contains("typoField"), "{error}");
    }

    #[test]
    fn an_error_without_a_message_says_its_title() {
        let error = ParserError::of_readable(&ReadableError::from_display("", "ab", 1));
        assert_eq!(error, ParserError { at: Some((0, 1)), message: "Validation Error".to_owned() });
    }

    #[test]
    fn it_is_shown_with_its_place_from_one() {
        let error = ParserError { at: Some((2, 4)), message: "expected </hkparam>".to_owned() };
        assert_eq!(error.to_string(), "3:5: expected </hkparam>");
        assert_eq!(ParserError::without_place("no such file").to_string(), "no such file");
    }
}
