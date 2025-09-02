//! errors of `This crate`
use std::io;

/// `tracing_rotation` Error
#[derive(Debug, snafu::Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    /// Standard io error
    #[snafu(transparent)]
    FailedIo { source: io::Error },

    /// Failed to initialize logger.
    FailedInitLog,

    /// Uninitialized logger.
    UninitLog,

    /// Tracing log error
    #[snafu(transparent)]
    FailedSetTracing {
        source: tracing::subscriber::SetGlobalDefaultError,
    },

    /// Tracing subscriber reload error
    #[snafu(transparent)]
    FailedReloadTracingSub {
        source: tracing_subscriber::reload::Error,
    },
    ////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
}

/// `Result` for this crate.
pub type Result<T, E = Error> = core::result::Result<T, E>;
