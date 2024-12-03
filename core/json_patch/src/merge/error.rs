use simd_json::TryTypeError;

/// Json patch error
#[derive(snafu::Snafu, Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Specified path was not found in the JSON:{path}
    NotFoundTarget { path: String },

    /// Pointer is empty, cannot add
    EmptyPointer,

    /// Invalid index: {index}
    InvalidIndex { index: String },

    /// The range syntax can only be used for Arrays.
    UnsupportedRangeKind,

    /// Cannot go deeper in a String
    InvalidString,

    /// Can't go deeper in a static node
    InvalidTarget,

    #[snafu(transparent)]
    TryType { source: TryTypeError },

    #[snafu(transparent)]
    OutOfRange {
        source: crate::merge::range::error::RangeError,
    },

    /// Invalid range format: {range}
    InvalidRange { range: String },

    /// Tried to put Alary for array index, but that is invalid. (Because 2-dimensional arrays do not exist in the C++ class.)
    WrongMatrix,

    /// Replace operation requires matching array size or a single value.
    InvalidReplaceSize,

    /// Type mismatch: expected {expected}, found {found}
    TypeMismatch {
        expected: &'static str,
        found: &'static str,
    },
}

/// Result type alias for JSON patch operations.
pub type Result<T, E = Error> = core::result::Result<T, E>;
