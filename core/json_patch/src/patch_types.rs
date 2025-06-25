pub use crate::operation::Op;
use crate::JsonPatchError;

use simd_json::BorrowedValue;
use std::ops::Range;

/// A prioritized JSON patch.
///
/// This patch can either represent:
///
/// - `One`: a single field or C++ class (serialized as a JSON value),
/// - `Seq`: a sequence of patch operations, intended to be applied to JSON arrays.
///
/// This enum allows merging multiple patches, each with an associated priority,
/// and supports both scalar and sequence-style edits.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub enum Patch<'a> {
    /// A single patch targeting one field or one object.
    ///
    /// Typically used to modify a scalar value or an entire C++ object
    /// serialized as a single JSON value.
    /// - Priority is considered just before merging.
    #[cfg_attr(
        feature = "serde",
        serde(bound(deserialize = "ValueWithPriority<'a>: serde::Deserialize<'de>"))
    )]
    One(ValueWithPriority<'a>),

    /// A sequence of prioritized patches.
    ///
    /// Intended for patches that apply to JSON arrays, representing multiple
    /// insertions, deletions, or replacements at specific ranges.
    /// - Patch on the assumption that it has already been overwritten by the highest priority.
    Seq(Vec<ValueWithPriority<'a>>),
}

/// A JSON patch along with its associated priority.
///
/// The priority determines how conflicting patches should be resolved.
/// Lower numbers indicate higher precedence.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ValueWithPriority<'a> {
    #[cfg_attr(
        feature = "serde",
        serde(bound(deserialize = "JsonPatch<'a>: serde::Deserialize<'de>"))
    )]
    pub patch: JsonPatch<'a>,
    /// The priority of the patch. Lower values have higher precedence.
    pub priority: usize,
}

impl<'a> ValueWithPriority<'a> {
    #[inline]
    pub const fn new(patch: JsonPatch<'a>, priority: usize) -> Self {
        Self { patch, priority }
    }
}

/// A JSON patch operation targeting a specific range in an array.
///
/// This is used only for array-based (sequence) operations.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OpRange {
    /// The type of operation (Add, Remove, Replace).
    pub op: Op,
    /// The target index range in the array (0-based, exclusive at the end).
    pub range: Range<usize>,
}

/// Represents the kind of patch operation, depending on the JSON data structure.
///
/// This enum allows distinguishing between patches on scalars vs. sequences.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OpRangeKind {
    /// A non-array operation (e.g., replacing an object or scalar).
    Pure(Op),

    /// An operation on a continuous range of array indices.
    Seq(OpRange),

    /// An operation on discrete array ranges.
    ///
    /// Operations that remain due to compatibility issues.
    /// Currently disassembled and merged in Seq.
    ///
    /// TODO: Remove this.
    Discrete(Vec<OpRange>),
}

impl OpRangeKind {
    /// Returns the `OpRange` if the operation is of kind `Seq`.
    ///
    /// # Panics
    /// Panics if the kind is `Pure`, as no range information is available.
    #[inline]
    pub fn as_seq(&self) -> Result<&OpRange, JsonPatchError> {
        match self {
            Self::Seq(op_range) => Ok(op_range),
            _ => Err(JsonPatchError::ExpectedSeq {
                unexpected: self.clone(),
            }),
        }
    }
}

impl Default for OpRangeKind {
    fn default() -> Self {
        Self::Pure(Op::Add)
    }
}

/// Represents a single JSON patch operation.
///
/// This can be either a scalar update or an array modification.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Default, PartialEq)]
pub struct JsonPatch<'a> {
    /// The type and target of the operation (including range if applicable).
    pub op: OpRangeKind,

    /// The value involved in the patch (e.g., value to add or replace).
    #[cfg_attr(
        feature = "serde",
        serde(bound(deserialize = "BorrowedValue<'a>: serde::Deserialize<'de>"))
    )]
    pub value: BorrowedValue<'a>,
}
