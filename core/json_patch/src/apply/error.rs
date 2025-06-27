use std::borrow::Cow;

use simd_json::TryTypeError;

/// Result type alias for JSON patch operations.
pub type Result<T, E = JsonPatchError> = core::result::Result<T, E>;

/// Represents errors that may occur during JSON patch operations.
#[derive(snafu::Snafu, Debug, Clone, PartialEq, Eq)]
pub enum JsonPatchError {
    /// The specified path was not found in the JSON structure.
    #[snafu(display("No such path(`{path}`) in template.\n{value}"))]
    NotFoundTarget {
        /// The path in the JSON structure.
        path: String,
        /// The JSON patch value debug representation.
        value: String,
    },

    /// The JSON Pointer is empty, which is invalid for this operation.
    #[snafu(display("Pointer is empty, cannot add\n{path}\n{value}"))]
    EmptyPointer { path: String, value: String },

    /// The index specified for an array operation is invalid.
    #[snafu(display("Invalid index: {index}\n{path}\n{value}"))]
    InvalidIndex {
        /// The index that caused the error.
        index: String,
        /// The path in the JSON structure.
        path: String,
        /// The JSON patch value debug representation.
        value: String,
    },

    /// The type of the patch operation is mismatched; expected a pure type.
    #[snafu(display(
        "Mismatch apply_patch type. Expected pure. but got {unexpected:?}\n{path}\n{value}"
    ))]
    MismatchApplyType {
        /// The unexpected type encountered.
        unexpected: crate::OpRangeKind,
        /// The path in the JSON structure.
        path: String,
        /// The JSON patch value debug representation.
        value: String,
    },

    /// The range syntax is only supported for arrays, not other JSON types.
    #[snafu(display("The range syntax can only be used for Arrays.\n{path}\n{value}"))]
    UnsupportedRangeKind { path: String, value: String },

    /// Attempted to traverse into a string as if it were a container.
    #[snafu(display("Cannot go deeper in a String\n{path}\n{value}"))]
    InvalidString { path: String, value: String },

    /// Tried to traverse a static or terminal JSON node further.
    #[snafu(display("Can't go deeper in a static node\n{path}\n{value}"))]
    InvalidTarget { path: String, value: String },

    /// Type conversion or inference failed during patching.
    #[snafu(display("TryType failed: {source}\n{path}\n{value}"))]
    TryType {
        source: TryTypeError,
        path: String,
        value: String,
    },

    /// Attempted to access a range that is out of bounds in an array.
    #[snafu(display("Out Of Range {path}: {source}\n{value}"))]
    OutOfRange {
        source: crate::range::error::RangeError,
        path: String,
        value: String,
    },

    /// Attempted to access a range that is out of bounds in an array.
    #[snafu(display("Valid range is [0, {actual_len}), but got {patch_range:?}"))]
    UnexpectedRange {
        patch_range: std::ops::Range<usize>,
        actual_len: usize,
    },

    /// Invalid matrix operation: attempted to simulate 2D array in a flat structure.
    #[snafu(display("Tried to put Alary for array index, but that is invalid. 2D arrays do not exist in the C++ class.\n{path}\n{value}"))]
    WrongMatrix { path: String, value: String },

    #[snafu(display("Expected Seq. but got {unexpected:#?}"))]
    ExpectedSeq { unexpected: crate::OpRangeKind },
}

impl JsonPatchError {
    /// Converts a JSON path and value to a string representation for error reporting.
    fn format_path_value<'a>(
        path: &[Cow<'a, str>],
        value: impl core::fmt::Debug,
    ) -> (String, String) {
        (path.join("/"), format!("{value:#?}"))
    }

    /// Creates a `NotFoundTarget` error from the given path and value.
    pub fn not_found_target_from<'a>(path: &[Cow<'a, str>], value: impl core::fmt::Debug) -> Self {
        let (path, value) = Self::format_path_value(path, value);
        Self::NotFoundTarget { path, value }
    }

    /// Creates an `EmptyPointer` error from the given path and value.
    pub fn empty_pointer_from<'a>(path: &[Cow<'a, str>], value: impl core::fmt::Debug) -> Self {
        let (path, value) = Self::format_path_value(path, value);
        Self::EmptyPointer { path, value }
    }

    /// Creates an `InvalidIndex` error with the specified index, path, and value.
    pub fn invalid_index_from<'a>(
        index: usize,
        path: &[Cow<'a, str>],
        value: impl core::fmt::Debug,
    ) -> Self {
        let (path, value) = Self::format_path_value(path, value);
        Self::InvalidIndex {
            index: index.to_string(),
            path,
            value,
        }
    }

    /// Creates a `MismatchApplyType` error with the unexpected type, path, and value.
    pub fn mismatch_apply_type_from<'a>(
        unexpected: crate::OpRangeKind,
        path: &[Cow<'a, str>],
        value: impl core::fmt::Debug,
    ) -> Self {
        let (path, value) = Self::format_path_value(path, value);
        Self::MismatchApplyType {
            unexpected,
            path,
            value,
        }
    }

    /// Creates an `UnsupportedRangeKind` error from the given path and value.
    pub fn unsupported_range_kind_from<'a>(
        path: &[Cow<'a, str>],
        value: impl core::fmt::Debug,
    ) -> Self {
        let (path, value) = Self::format_path_value(path, value);
        Self::UnsupportedRangeKind { path, value }
    }

    /// Creates an `InvalidString` error from the given path and value.
    pub fn invalid_string_from<'a>(path: &[Cow<'a, str>], value: impl core::fmt::Debug) -> Self {
        let (path, value) = Self::format_path_value(path, value);
        Self::InvalidString { path, value }
    }

    /// Creates an `InvalidTarget` error from the given path and value.
    pub fn invalid_target_from<'a>(path: &[Cow<'a, str>], value: impl core::fmt::Debug) -> Self {
        let (path, value) = Self::format_path_value(path, value);
        Self::InvalidTarget { path, value }
    }

    /// Creates a `TryType` error from the given source error, path, and value.
    pub fn try_type_from<'a>(
        source: TryTypeError,
        path: &[Cow<'a, str>],
        value: impl core::fmt::Debug,
    ) -> Self {
        let (path, value) = Self::format_path_value(path, value);
        Self::TryType {
            source,
            path,
            value,
        }
    }
}
