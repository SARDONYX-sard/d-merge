pub use crate::operation::Op;

use simd_json::BorrowedValue;
use std::borrow::Cow;
use std::ops::Range;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub enum Patch<'a> {
    /// - value: (json patch, priority)
    #[cfg_attr(
        feature = "serde",
        serde(bound(deserialize = "ValueWithPriority<'a>: serde::Deserialize<'de>"))
    )]
    OneField(ValueWithPriority<'a>),
    /// - value: [(json patch, priority), (json patch, priority), ...]
    Seq(Vec<ValueWithPriority<'a>>),
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ValueWithPriority<'a> {
    #[cfg_attr(
        feature = "serde",
        serde(bound(deserialize = "JsonPatch<'a>: serde::Deserialize<'de>"))
    )]
    pub patch: JsonPatch<'a>,
    pub priority: usize,
}

impl<'a> ValueWithPriority<'a> {
    #[inline]
    pub const fn new(patch: JsonPatch<'a>, priority: usize) -> Self {
        Self { patch, priority }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OpRange {
    pub op: Op,
    pub range: Range<usize>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OpRangeKind {
    // Except for array
    Pure(Op),
    // Sequence Array
    Seq(OpRange),
    // Discrete Array
    Discrete(Vec<OpRange>),
}

impl OpRangeKind {
    /// # Panics
    /// self is Pure
    #[inline]
    pub fn as_seq(&self) -> &OpRange {
        match self {
            Self::Pure(op) => panic!("Expected Seq. But got Pure: op: {op:?}"),
            Self::Seq(op_range) => op_range,
            Self::Discrete(_) => todo!(),
        }
    }
}

impl Default for OpRangeKind {
    fn default() -> Self {
        Self::Pure(Op::Add)
    }
}

/// Struct representing a JSON patch operation.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Default, PartialEq)]
pub struct JsonPatch<'a> {
    /// The type of operation to perform (Add, Remove, Replace).
    pub op: OpRangeKind,
    /// The value to be added or replaced in the JSON.
    #[cfg_attr(
        feature = "serde",
        serde(bound(deserialize = "BorrowedValue<'a>: serde::Deserialize<'de>"))
    )]
    pub value: BorrowedValue<'a>,
}

pub type JsonPath<'a> = Vec<Cow<'a, str>>;
