//! errors of `This crate`
use std::{io, path::PathBuf};

/// GUI Error
#[derive(Debug, snafu::Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    /// Not found {field_name}
    UnknownField { field_name: String },

    /// Not found {class_name}
    UnknownClass { class_name: String },

    /// Unknown field type name: {field_type}
    UnknownFieldType { field_type: String },

    /// `<! -- MODE_CODE ~<id>~` and `<! -- CLOSE -->` should be a pair,
    /// but before the `CLOSE` comment comes `<! -- MODE_CODE ~<id>~` has come twice.
    AlreadyPatchMode,

    /// Not found push target json patch.
    NotFoundPushTargetJson,

    /// Failed to read file from
    #[snafu(display("[I/O Error]{}: {source}", path.display()))]
    IoError { source: io::Error, path: PathBuf },

    // NOTE: Cannot `#snafu(transparent)`
    /// Parser combinator Error
    #[snafu(display("{err}"))]
    ContextError {
        err: winnow::error::ErrMode<winnow::error::ContextError>,
    },

    /// Human readable XML parsing error
    #[snafu(transparent)]
    ReadableError {
        source: serde_hkx::errors::readable::ReadableError,
    },
}

/// `Result` for this crate.
pub type Result<T, E = Error> = core::result::Result<T, E>;
