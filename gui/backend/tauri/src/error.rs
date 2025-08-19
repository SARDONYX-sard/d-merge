//! errors of `This crate`
use std::io;

/// GUI Error
#[derive(Debug, snafu::Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    /// Failed to get skyrim data dir: {source}
    NotFoundSkyrimDataDir { source: io::Error },

    /// Standard io error
    #[snafu(transparent)]
    FailedIo { source: io::Error },

    /// Not found resource dir. {source}
    NotFoundResourceDir { source: tauri::Error },

    ////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
    // Logger
    /// Not found log dir. {source}
    NotFoundLogDir { source: tauri::Error },

    /// Failed to initialize logger.
    #[snafu(transparent)]
    FailedInitRotationLog {
        source: tracing_rotation::error::Error,
    },
}

/// `Result` for this crate.
pub type Result<T, E = Error> = core::result::Result<T, E>;
