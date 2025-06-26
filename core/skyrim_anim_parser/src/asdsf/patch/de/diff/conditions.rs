use std::collections::HashMap;

use crate::common_parser::lines::Str;
use json_patch::ValueWithPriority;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone, PartialEq)]
pub struct ConditionsDiff<'a> {
    /// - key: replace target index
    /// - value: Partial change request
    pub one: HashMap<usize, ConditionDiff<'a>>,

    /// A request to change all elements of an array.
    ///
    /// This is processed after partial patching is complete.
    #[cfg_attr(
        feature = "serde",
        serde(
            borrow,
            bound(deserialize = "Vec<ValueWithPriority<'a>>: serde::Deserialize<'de>")
        )
    )]
    pub seq: Vec<ValueWithPriority<'a>>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ConditionDiff<'a> {
    /// The name of the variable used in the condition.
    pub variable_name: Option<Str<'a>>,

    /// The **start** of the allowed range (inclusive) for the condition value.
    /// - type: [`i32`]
    pub value_a: Option<i32>,
    /// - type: [`i32`]
    pub value_b: Option<i32>,
}
