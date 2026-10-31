use core::{fmt, str::FromStr};

use winnow::{
    ModalResult, Parser,
    ascii::{line_ending, till_line_ending},
    combinator::opt,
    error::{ContextError, ErrMode},
};
use winnow_ext::ReadableError;

use crate::common_parser::lines::Str;

/// Error returned while parsing an animation data patch.
///
/// Parser errors originating from winnow are stored as [`Error::ContextError`].
/// Errors which require dynamically generated messages are represented by
/// dedicated variants instead of being forced into `ContextError`.
#[allow(clippy::enum_variant_names)]
#[derive(Debug)]
pub(super) enum Error {
    /// A winnow parser failed.
    ContextError {
        /// The original winnow parser error.
        err: ErrMode<ContextError>,
    },

    /// A declared number of entries differs from the number of entries
    /// actually present in the input.
    InvalidLength {
        /// Name of the field containing the declared length.
        field: &'static str,

        /// Number declared by the input.
        expected: usize,

        /// Number of entries actually found.
        actual: usize,
    },

    /// The input ended before the expected number of entries could be read.
    UnexpectedEnd {
        /// Name of the field being parsed.
        field: &'static str,

        /// Number of entries declared by the input.
        expected: usize,

        /// Number of entries successfully read before reaching the end.
        actual: usize,
    },

    TooManyEntries {
        field: &'static str,
        expected: usize,
    },

    /// An error that has already been converted to a readable error.
    ReadableError {
        /// The formatted error.
        source: ReadableError,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ContextError { err } => write!(f, "{err}"),
            Self::InvalidLength { field, expected, actual } => {
                write!(f, "expected {expected} {field}, but got {actual}")
            }
            Self::UnexpectedEnd { field, expected, actual } => {
                write!(f, "expected {expected} {field}, but reached end of input after {actual}")
            }
            Self::TooManyEntries { field, expected } => {
                write!(f, "too many {field}: expected {expected}")
            }

            Self::ReadableError { source } => source.fmt(f),
        }
    }
}

impl std::error::Error for Error {}

/// Deserializer state used while parsing an animation data patch.
///
/// `input` is advanced as values are parsed, while `original` remains
/// unchanged so that errors can be reported with their position in the
/// original input.
#[derive(Debug)]
pub(super) struct PatchDeserializer<'de> {
    /// Remaining input.
    pub(super) input: &'de str,

    /// Original input used for error reporting.
    original: &'de str,
}

impl<'de> PatchDeserializer<'de> {
    /// Creates a deserializer from patch input.
    #[inline]
    pub(super) const fn from_str(input: &'de str) -> Self {
        Self { input, original: input }
    }

    /// Parses the next value using a winnow parser.
    ///
    /// # Errors
    /// Returns [`Error::ContextError`] when the supplied parser fails.
    #[inline]
    pub(super) fn parse_next<O>(
        &mut self,
        mut parser: impl Parser<&'de str, O, ErrMode<ContextError>>,
    ) -> Result<O, Error> {
        parser.parse_next(&mut self.input).map_err(|err| Error::ContextError { err })
    }

    /// Parse by argument parser no consume.
    ///
    /// # Errors
    /// Returns [`Error::ContextError`] when the supplied parser fails.
    #[inline]
    pub(super) fn parse_peek<O>(
        &self,
        mut parser: impl Parser<&'de str, O, ErrMode<ContextError>>,
    ) -> Result<O, Error> {
        let (_, res) = parser.parse_peek(self.input).map_err(|err| Error::ContextError { err })?;
        Ok(res)
    }

    /// Converts an internal error into an error containing source position
    /// information while preserving the internal error type.
    ///
    /// This is used when an error occurs inside another deserializer
    /// operation and must retain its source position before being propagated.
    #[cold]
    pub(super) fn to_readable_err(&self, err: Error) -> Error {
        let readable = match err {
            Error::ContextError { err } => ReadableError::from_context(
                err,
                self.original,
                self.original.len() - self.input.len(),
            ),
            Error::ReadableError { source } => source,
            err => ReadableError::from_display(
                err,
                self.original,
                self.original.len() - self.input.len(),
            ),
        };

        Error::ReadableError { source: readable }
    }

    /// Converts an internal error into the public readable error type.
    ///
    /// # Errors
    ///
    /// Returns the contained [`ReadableError`] after ensuring that the
    /// internal error has source position information.
    #[cold]
    pub(super) fn finish_error(&self, err: Error) -> ReadableError {
        match err {
            Error::ContextError { err } => ReadableError::from_context(
                err,
                self.original,
                self.original.len() - self.input.len(),
            ),
            Error::ReadableError { source } => source,
            err => ReadableError::from_display(
                err,
                self.original,
                self.original.len() - self.input.len(),
            ),
        }
    }

    /// Reads exactly `expected` non-empty lines.
    ///
    /// This is intentionally implemented as a deserializer operation rather
    /// than a winnow parser because length mismatches require a dynamically
    /// generated error message.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidLength`] when an empty line is encountered
    /// before the declared number of entries has been read.
    ///
    /// Returns [`Error::UnexpectedEnd`] when the input ends before the
    /// declared number of entries has been read.
    pub(super) fn read_non_empty_lines(
        &mut self,
        field: &'static str,
        expected: usize,
    ) -> Result<Vec<Str<'de>>, Error> {
        let mut values = Vec::with_capacity(expected);

        while values.len() < expected {
            if self.input.is_empty() {
                return Err(Error::UnexpectedEnd { field, expected, actual: values.len() });
            }

            let line_start = self.input;

            let line = self.parse_next(one_line).map_err(|err| self.to_readable_err(err))?;

            if line.is_empty() {
                return Err(Error::InvalidLength { field, expected, actual: values.len() });
            }

            if self.input.len() >= line_start.len() {
                return Err(Error::UnexpectedEnd { field, expected, actual: values.len() });
            }

            values.push(line);
        }

        Ok(values)
    }

    /// Reads all consecutive non-empty lines until an empty line or end of input.
    ///
    /// The terminating empty line is consumed and is not included in the result.
    ///
    /// # Errors
    ///
    /// Returns an error if a line cannot be parsed.
    pub(super) fn read_non_empty_lines_until_end(&mut self) -> Result<Vec<Str<'de>>, Error> {
        let mut values = Vec::new();

        loop {
            if self.input.is_empty() {
                break;
            }

            let line = self.parse_next(one_line).map_err(|err| self.to_readable_err(err))?;

            if line.is_empty() {
                break;
            }

            values.push(line);
        }

        Ok(values)
    }
}

/// Parses one line.
///
/// The trailing line ending is optional because patch files may omit the
/// final line ending.
///
/// # Errors
///
/// Returns a parser error if the input does not contain a line.
pub(super) fn one_line<'a>(input: &mut &'a str) -> ModalResult<Str<'a>> {
    let line = till_line_ending.parse_next(input)?;
    opt(line_ending).parse_next(input)?;
    Ok(line.into())
}

/// Parses one line and verifies that it can be parsed as `T`.
///
/// The original line is returned instead of the parsed value.
///
/// # Errors
///
/// Returns a parser error if the line cannot be parsed as `T`.
pub(super) fn verify_line_parses_to<'a, T>(input: &mut &'a str) -> ModalResult<Str<'a>>
where
    T: FromStr,
{
    let line = till_line_ending.verify(|s: &str| s.parse::<T>().is_ok()).parse_next(input)?;

    opt(line_ending).parse_next(input)?;

    Ok(line.into())
}

/// Parses one line directly into `T`.
///
/// # Errors
///
/// Returns a parser error if the line cannot be parsed as `T`.
pub(super) fn parse_one_line<T: FromStr>(input: &mut &str) -> ModalResult<T> {
    let value = till_line_ending.parse_to().parse_next(input)?;
    opt(line_ending).parse_next(input)?;
    Ok(value)
}
