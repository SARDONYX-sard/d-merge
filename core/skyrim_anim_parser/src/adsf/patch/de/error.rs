//! errors of `patch de`

/// Patch deserializer error
#[derive(Debug, snafu::Snafu)]
pub enum Error {
    /// Mismatched type. Expected one, but got array patch
    ExpectedOne,

    /// Mismatched type. Expected array, but got one replacer type.
    ExpectedArray,

    /// Mismatched type. Expected transitions, but got other type
    ExpectedTransition,

    /// Mismatched type. Expected rotations, but got other type
    ExpectedRotation,

    /// Iterator end. The following parsing target is required but could not be found.
    EndOfLineKind,

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
}

/// `Result` for this crate.
pub type Result<T, E = Error> = core::result::Result<T, E>;
