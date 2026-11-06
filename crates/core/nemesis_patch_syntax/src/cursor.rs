//! Where a parser is in a patch, and its errors with that place.

use core::{fmt, marker::PhantomData};

use winnow::{
    Parser,
    error::{ContextError, ErrMode},
};
use winnow_ext::ReadableError;

/// The error type of a patch parser: what [`Cursor`] needs of it.
pub trait ParseFailure: fmt::Display + Sized {
    /// A parser combinator failed.
    fn from_context(err: ErrMode<ContextError>) -> Self;

    /// An error that shows where it is.
    fn from_readable(source: ReadableError) -> Self;

    /// `Err(self)`, unless this is one of the two above.
    ///
    /// # Errors
    /// This is another kind of error.
    fn into_context_or_readable(self) -> Result<ContextOrReadable, Self>;
}

/// See [`ParseFailure::into_context_or_readable`].
#[derive(Debug)]
pub enum ContextOrReadable {
    Context(ErrMode<ContextError>),
    Readable(ReadableError),
}

/// Implements [`ParseFailure`] for an error enum, given its two variants.
///
/// e.g. `impl_parse_failure!(Error, Context, Readable);` for
/// `Context { err: ErrMode<ContextError> }` and `Readable { source: ReadableError }`.
#[macro_export]
macro_rules! impl_parse_failure {
    ($error:ty, $context:ident, $readable:ident) => {
        impl $crate::cursor::ParseFailure for $error {
            fn from_context(err: ::winnow::error::ErrMode<::winnow::error::ContextError>) -> Self {
                Self::$context { err }
            }

            fn from_readable(source: ::winnow_ext::ReadableError) -> Self {
                Self::$readable { source }
            }

            fn into_context_or_readable(self) -> ::core::result::Result<$crate::cursor::ContextOrReadable, Self> {
                match self {
                    Self::$context { err } => Ok($crate::cursor::ContextOrReadable::Context(err)),
                    Self::$readable { source } => Ok($crate::cursor::ContextOrReadable::Readable(source)),
                    other => Err(other),
                }
            }
        }
    };
}

/// The part of a patch not yet parsed, and the whole of it for the errors. `E`: the parser's error.
pub struct Cursor<'a, E> {
    /// What is not parsed yet.
    pub input: &'a str,
    /// The whole patch, which does not move.
    pub original: &'a str,
    _error: PhantomData<fn() -> E>,
}

impl<E> fmt::Debug for Cursor<'_, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Cursor").field("input", &self.input).field("original", &self.original).finish()
    }
}

impl<E> Clone for Cursor<'_, E> {
    #[inline]
    fn clone(&self) -> Self {
        Self { input: self.input, original: self.original, _error: PhantomData }
    }
}

impl<E> Default for Cursor<'_, E> {
    #[inline]
    fn default() -> Self {
        Self::new("")
    }
}

impl<'a, E> Cursor<'a, E> {
    #[must_use]
    #[inline]
    pub const fn new(input: &'a str) -> Self {
        Self { input, original: input, _error: PhantomData }
    }

    /// How far the parsing is, in bytes.
    const fn offset(&self) -> usize {
        self.original.len() - self.input.len()
    }
}

impl<'a, E: ParseFailure> Cursor<'a, E> {
    /// Runs `parser` here and moves past what it took.
    ///
    /// # Errors
    /// `parser` failed.
    pub fn parse_next<O>(&mut self, mut parser: impl Parser<&'a str, O, ErrMode<ContextError>>) -> Result<O, E> {
        parser.parse_next(&mut self.input).map_err(E::from_context)
    }

    /// Runs `parser` here without moving.
    ///
    /// # Errors
    /// `parser` failed.
    pub fn parse_peek<O>(&self, mut parser: impl Parser<&'a str, O, ErrMode<ContextError>>) -> Result<O, E> {
        let (_, output) = parser.parse_peek(self.input).map_err(E::from_context)?;
        Ok(output)
    }

    /// `err` with the place the parsing stopped at.
    ///
    /// The checks a parser does on what it read have no place of their own, only this one has.
    #[cold]
    #[must_use]
    pub fn to_readable(&self, err: E) -> E {
        E::from_readable(match err.into_context_or_readable() {
            Ok(ContextOrReadable::Context(err)) => ReadableError::from_context(err, self.original, self.offset()),
            Ok(ContextOrReadable::Readable(source)) => source,
            Err(err) => ReadableError::from_display(err, self.original, self.offset()),
        })
    }

    /// `message` with the place the parsing stopped at.
    #[cold]
    pub fn message_to_readable(&self, message: impl fmt::Display) -> E {
        E::from_readable(ReadableError::from_display(message, self.original, self.offset()))
    }
}

#[cfg(test)]
mod tests {
    use winnow::ascii::digit1;

    use super::*;

    #[derive(Debug)]
    enum TestError {
        Context { err: ErrMode<ContextError> },
        Readable { source: ReadableError },
        Other,
    }

    impl fmt::Display for TestError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::Context { err } => write!(f, "{err}"),
                Self::Readable { source } => write!(f, "{source}"),
                Self::Other => f.write_str("other"),
            }
        }
    }

    impl_parse_failure!(TestError, Context, Readable);

    #[test]
    fn moves_only_when_asked() {
        let mut cursor = Cursor::<TestError>::new("12ab");
        assert_eq!(cursor.parse_peek(digit1).unwrap(), "12");
        assert_eq!(cursor.input, "12ab");
        assert_eq!(cursor.parse_next(digit1).unwrap(), "12");
        assert_eq!((cursor.input, cursor.original, cursor.offset()), ("ab", "12ab", 2));
    }

    #[test]
    fn every_error_becomes_readable_with_its_place() {
        let mut cursor = Cursor::<TestError>::new("12\nab");
        cursor.parse_next(digit1).unwrap();
        let err = cursor.parse_next(digit1).unwrap_err();
        assert!(matches!(cursor.to_readable(err), TestError::Readable { .. }));

        let readable = cursor.to_readable(TestError::Other);
        assert!(readable.to_string().contains("other"), "{readable}");
        assert_eq!(cursor.to_readable(readable).to_string(), cursor.to_readable(TestError::Other).to_string(), "stays");

        assert!(cursor.message_to_readable("why").to_string().contains("why"));
    }
}
