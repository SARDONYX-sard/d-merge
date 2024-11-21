//! errors of `This crate`
use std::{io, path::PathBuf};

/// GUI Error
#[derive(Debug, snafu::Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    /// Unknown field names: \"{field_name}\". Expected one of [{}].", acceptable_fields.join(", ")
    #[snafu(display("Unknown field names: \"{field_name}\". Expected one of `[{}]`.", acceptable_fields.join(", ")))]
    UnknownField {
        field_name: String,
        acceptable_fields: Vec<&'static str>,
    },

    /// Missing field info
    MissingFieldInfo,

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
    /// Parser combinator Error: {err}
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
