use simd_json::TryTypeError;

/// Json patch error
#[derive(snafu::Snafu, Debug, Clone, PartialEq, Eq)]
pub enum JsonPatchError {
    /// The specified path does not exist: {path}
    NotFoundTarget { path: String },

    /// Pointer is empty, cannot add
    EmptyPointer,

    /// Invalid index: {index}
    InvalidIndex { index: String },

    /// mismatch apply_patch type. Expected pure. but got {unexpected:?}
    MismatchApplyType { unexpected: crate::OpRangeKind },

    /// The range syntax can only be used for Arrays.
    UnsupportedRangeKind,

    /// Cannot go deeper in a String
    InvalidString,

    /// Can't go deeper in a static node
    InvalidTarget,

    #[snafu(transparent)]
    TryType { source: TryTypeError },

    /// Out Of Range {path}: {source}
    OutOfRange {
        path: String,
        source: crate::range::error::RangeError,
    },

    /// Tried to put Alary for array index, but that is invalid. (Because 2-dimensional arrays do not exist in the C++ class.)
    WrongMatrix,
}

/// Result type alias for JSON patch operations.
pub type Result<T, E = JsonPatchError> = core::result::Result<T, E>;
