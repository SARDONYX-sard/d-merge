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

/// Represents the kind of modification applied to a value or array field.
///
/// This enum distinguishes between scalar/object changes and array operations:
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "kind", rename_all = "snake_case"))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Action {
    /// Single field operation on a scalar or object.
    /// Only `replace` or `remove` are valid.
    Pure {
        /// The type of operation (Add, Remove, Replace).
        op: Op,
    },

    /// Operation on a contiguous range of an array.
    /// Supports `add`, `replace`, and `remove`.
    Seq {
        /// The type of operation (Add, Remove, Replace).
        op: Op,

        /// The target index range in the array (0-based, exclusive at the end).
        range: Range<usize>,
    },

    /// Append operation to the end of an array.
    /// The target index is implicit; no internal fields are needed.
    SeqPush,
}

impl Action {
    /// Returns the `OpRange` if the operation is of kind `Seq`.
    ///
    /// # Errors
    /// If the kind is `Pure`/ `SeqPush`
    #[inline]
    pub fn try_as_seq(&self) -> Result<(Op, Range<usize>), JsonPatchError> {
        match self {
            Self::Seq { op, range } => Ok((*op, range.clone())),
            _ => Err(JsonPatchError::ExpectedSeq {
                unexpected: self.clone(),
            }),
        }
    }
}

impl Default for Action {
    #[inline]
    fn default() -> Self {
        Self::Pure { op: Op::Add }
    }
}

/// Represents a single JSON patch operation.
///
/// This can be either a scalar update or an array modification.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Default, PartialEq)]
pub struct JsonPatch<'a> {
    /// The type and target of the operation (including range if applicable).
    pub action: Action,

    /// The value involved in the patch (e.g., value to add or replace).
    #[cfg_attr(
        feature = "serde",
        serde(bound(deserialize = "BorrowedValue<'a>: serde::Deserialize<'de>"))
    )]
    pub value: BorrowedValue<'a>,
}

#[cfg(test)]
mod tests {

    #[cfg(feature = "serde")]
    #[test]
    fn json_fmt_test() {
        use super::*;

        let pure = Action::Pure { op: Op::Replace };
        let seq = Action::Seq {
            op: Op::Add,
            range: 2..5,
        };
        let push = Action::SeqPush;

        let pure_json = simd_json::to_string(&pure).unwrap();
        let seq_json = simd_json::to_string(&seq).unwrap();
        let push_json = simd_json::to_string(&push).unwrap();

        let expected_pure = "{\
\"kind\":\"pure\",\
\"op\":\"replace\"\
}";

        let expected_seq = "{\
\"kind\":\"seq\",\
\"op\":\"add\",\
\"range\":{\"start\":2,\"end\":5}\
}";

        let expected_push = "{\
\"kind\":\"seq_push\"\
}";

        assert_eq!(pure_json, expected_pure);
        assert_eq!(seq_json, expected_seq);
        assert_eq!(push_json, expected_push);
    }
}
