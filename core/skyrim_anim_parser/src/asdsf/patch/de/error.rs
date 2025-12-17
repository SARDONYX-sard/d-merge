//! errors of `This crate`

/// GUI Error
#[derive(Debug, snafu::Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
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

    //////////////////////////////////////////////////////////////////////
    /// {kind} entry in AnimSetData was expected to be modified, but no target for modification was found.
    NotFoundApplyTarget { kind: String },

    // merge
    #[snafu(transparent)]
    SimdJson {
        /// simd_json error
        source: simd_json::Error,
    },
    #[snafu(transparent)]
    JsonPatch { source: json_patch::JsonPatchError },
}

/// `Result` for this crate.
pub type Result<T, E = Error> = core::result::Result<T, E>;
