//! errors of `patch de`

/// Patch deserializer error
#[derive(Debug, snafu::Snafu)]
pub enum Error {
    /// It should be a patch that requires range, but for some reason the information was missing.
    NeedRange,

    /// Diff patch need `<! -- MODE_CODE ~<id>~ OPEN`. but not found.
    NeedInModDiff,

    /// `<! -- MODE_CODE ~<id>~ OPEN` and `<! -- CLOSE -->` should be a pair,
    /// but before the `CLOSE` comment comes `<! -- MODE_CODE ~<id>~ OPEN` has come twice.
    AlreadyPatchMode,

    /// Incomplete animationdatasinglefile.txt patch
    IncompleteParse,

    // NOTE: Cannot `#snafu(transparent)`
    /// Parser combinator Error: {err}
    Context {
        err: winnow::error::ErrMode<winnow::error::ContextError>,
    },

    /// Human readable XML parsing error
    #[snafu(transparent)]
    Readable {
        source: serde_hkx::errors::readable::ReadableError,
    },

    //////////////////////////////////////////////////////////////////////
    // merge
    #[snafu(transparent)]
    SimdJson {
        /// simd_json error
        source: simd_json::Error,
    },
    #[snafu(transparent)]
    JsonPatch { source: json_patch::JsonPatchError },
}

/// Result type alias for AnimSetData patch error.
pub type Result<T, E = Error> = core::result::Result<T, E>;
