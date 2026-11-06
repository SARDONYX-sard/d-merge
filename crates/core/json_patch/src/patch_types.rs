use std::ops::Range;

use simd_json::BorrowedValue;

use crate::JsonPatchError;
pub(crate) use crate::operation::Op;

/// A JSON patch along with its associated priority.
///
/// The priority determines how conflicting patches should be resolved.
/// Higher numbers indicate higher precedence.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ValueWithPriority<'a> {
    #[cfg_attr(feature = "serde", serde(bound(deserialize = "JsonPatch<'a>: serde::Deserialize<'de>")))]
    pub patch: JsonPatch<'a>,
    /// The priority of the patch. Higher values have higher precedence.
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
    pub(crate) fn try_as_seq(&self) -> Result<(Op, Range<usize>), JsonPatchError> {
        match self {
            Self::Seq { op, range } => Ok((*op, range.clone())),
            _ => Err(JsonPatchError::ExpectedSeq { unexpected: self.clone() }),
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
    #[cfg_attr(feature = "serde", serde(bound(deserialize = "BorrowedValue<'a>: serde::Deserialize<'de>")))]
    pub value: BorrowedValue<'a>,
}

impl<'a> JsonPatch<'a> {
    /// This patch without what it does to the elements in `hidden`, as the patches that are left.
    ///
    /// A replace or a remove writes to each element of its range on its own (see `apply::seq`), so
    /// the rest of the range still does the same. What adds or pushes writes to no element.
    ///
    /// e.g. a replace of `0..3` with `[a, b, c]`, without `1..2`:
    /// a replace of `0..1` with `[a]`, and one of `2..3` with `[c]`.
    #[must_use]
    pub fn without_ranges(self, hidden: &[Range<usize>]) -> Vec<Self> {
        let (op, whole) = match &self.action {
            Action::Seq { op: op @ (Op::Replace | Op::Remove), range } => (*op, range.clone()),
            _ => return vec![self],
        };
        let values = match self.value {
            BorrowedValue::Array(values) if op == Op::Replace => *values,
            _ => Vec::new(),
        };
        pieces_without(&whole, hidden)
            .into_iter()
            .map(|range| {
                let value = if op == Op::Replace {
                    // A replace with fewer values than its range removes the rest: so does a piece.
                    let start = (range.start - whole.start).min(values.len());
                    let end = (range.end - whole.start).min(values.len());
                    BorrowedValue::Array(Box::new(values[start..end].to_vec()))
                } else {
                    BorrowedValue::default()
                };
                Self { action: Action::Seq { op, range }, value }
            })
            .collect()
    }
}

/// What is left of `whole` without `hidden`.
fn pieces_without(whole: &Range<usize>, hidden: &[Range<usize>]) -> Vec<Range<usize>> {
    let mut hidden = hidden.to_vec();
    hidden.sort_unstable_by_key(|range| range.start);
    let mut pieces = Vec::new();
    let mut at = whole.start;
    for range in hidden.iter().filter(|range| range.start < whole.end && range.end > whole.start) {
        if range.start > at {
            pieces.push(at..range.start);
        }
        at = at.max(range.end);
    }
    if at < whole.end {
        pieces.push(at..whole.end);
    }
    pieces
}

#[cfg(test)]
#[expect(clippy::single_range_in_vec_init, reason = "a list of holes, which here has one")]
mod tests {
    use super::*;

    fn replace(range: Range<usize>, values: &[&'static str]) -> JsonPatch<'static> {
        let values = values.iter().map(|value| BorrowedValue::from(*value)).collect();
        JsonPatch { action: Action::Seq { op: Op::Replace, range }, value: BorrowedValue::Array(Box::new(values)) }
    }

    #[test]
    fn a_hole_leaves_the_rest_of_a_range() {
        let patch = replace(0..3, &["a", "b", "c"]);
        assert_eq!(patch.without_ranges(&[1..2]), [replace(0..1, &["a"]), replace(2..3, &["c"])]);

        let patch = replace(0..3, &["a", "b", "c"]);
        assert_eq!(patch.without_ranges(&[0..3]), []);

        // Beyond its range: all of it.
        let patch = replace(1..3, &["b", "c"]);
        assert_eq!(patch.without_ranges(&[0..1, 3..9]), [replace(1..3, &["b", "c"])]);

        // Overlapping holes, in any order.
        let patch = replace(0..5, &["a", "b", "c", "d", "e"]);
        assert_eq!(patch.without_ranges(&[3..4, 1..2, 1..3]), [replace(0..1, &["a"]), replace(4..5, &["e"])]);

        // Too few values remove the rest, also in a piece.
        let patch = replace(0..4, &["a", "b"]);
        assert_eq!(patch.without_ranges(&[0..1]), [replace(1..4, &["b"])]);
    }

    #[test]
    fn a_remove_stays_a_remove() {
        let remove =
            |range| JsonPatch { action: Action::Seq { op: Op::Remove, range }, value: BorrowedValue::default() };
        assert_eq!(remove(0..3).without_ranges(&[1..2]), [remove(0..1), remove(2..3)]);
    }

    #[test]
    fn what_adds_writes_to_no_element() {
        let push = JsonPatch { action: Action::SeqPush, value: BorrowedValue::default() };
        assert_eq!(push.clone().without_ranges(&[0..9]), [push]);
        let add = JsonPatch { action: Action::Seq { op: Op::Add, range: 0..1 }, value: BorrowedValue::default() };
        assert_eq!(add.clone().without_ranges(&[0..9]), [add]);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn json_fmt_test() {
        let pure = Action::Pure { op: Op::Replace };
        let seq = Action::Seq { op: Op::Add, range: 2..5 };
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
