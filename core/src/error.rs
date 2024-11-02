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

    /// Not found {field_name}
    UnknownFieldName { field_name: String },

    /// Not found {class_name}
    UnknownClassName { class_name: String },

    /// Unknown field type name: {field_type}
    UnknownFieldType { field_type: String },

    /// `<! -- CLOSE -->`If no comments have come in, but `<! -- MODE_CODE` is coming.
    AlreadyPatchMode,

    /// Not found push target json patch.
    NotFoundPushTargetJson,

    /// Human readable XML parsing error
    #[snafu(transparent)]
    ReadableError {
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
