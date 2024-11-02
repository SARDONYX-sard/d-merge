//! errors of `This crate`
use std::{io, path::PathBuf};

/// GUI Error
#[derive(Debug, snafu::Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    /// Failed to read file from
    #[snafu(display("{source}: {}", path.display()))]
    FailedReadFile { source: io::Error, path: PathBuf },

    /// Parser combinator Error
    #[snafu(display("{err}"))]
    ContextError {
        err: winnow::error::ErrMode<winnow::error::ContextError>,
    },

    /// Parser error
    #[snafu(transparent)]
    FailedParse {
        source: serde_hkx::errors::readable::ReadableError,
    },

    /// Standard io error
    #[snafu(transparent)]
    FailedIo { source: io::Error },

    /// Glob error
    #[snafu(transparent)]
    InvalidGlob { source: glob::PatternError },
}

/// `Result` for this crate.
pub type Result<T, E = Error> = core::result::Result<T, E>;
