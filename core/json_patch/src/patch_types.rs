pub use crate::operation::Op;
use crate::JsonPatchError;

use simd_json::BorrowedValue;
use std::ops::Range;

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
    /// # Errors
    /// If not the kind is `Seq.
    #[inline]
    pub fn try_as_seq(&self) -> Result<&OpRange, JsonPatchError> {
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
