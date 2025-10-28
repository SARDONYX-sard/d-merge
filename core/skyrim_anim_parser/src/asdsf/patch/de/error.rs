//! errors of `patch de`

/// Patch deserializer error
#[derive(Debug, snafu::Snafu)]
pub enum Error {
    /// Mismatched type. Expected one, but got array patch
    InvalidOpForOneField { op: json_patch::Op },

    /// Mismatched type. Expected transitions/rotation, but got one replacer type.
    ExpectedArray,

    /// Mismatched type. Expected Trigger(`Str`), but got other type
    ExpectedTrigger,

    /// The only thing that can be arranged in a double array is attacks. (This is a bug in the implementation.)
    InvalidSubRangeUsage,

    /// Mismatched type. Expected 1 field of `struct Condition`, but got other one field patch: {other}
    ExpectedOneFieldOfCondition { other: String },

    /// Mismatched type. Expected 1 field of `struct AnimInfo`, but got other one field patch: {other}
    ExpectedOneFieldOfAnimInfo { other: String },

    /// It should be a patch that requires range, but for some reason the information was missing.
    NeedMainRangeInformation,

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

    /// {kind} entry in AnimSetData was expected to be modified, but no target for modification was found.
    NotFoundApplyTarget { kind: String },

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
