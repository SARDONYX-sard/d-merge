use json_patch::ValueWithPriority;

use crate::asdsf::patch::de::patch_map::OnePatchMap;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone, PartialEq)]
pub(crate) struct NonNestedArrayDiff<'a> {
    /// modifying fields of a single.
    ///
    /// - key(replace target index): e.g.,
    ///   - ["[0]", "value_a"]
    ///   - ["[0]", "hashed_path"]
    #[cfg_attr(feature = "serde", serde(borrow, bound(deserialize = "OnePatchMap<'a>: serde::Deserialize<'de>")))]
    pub one: OnePatchMap<'a>,

    /// A request to change all elements of an array.
    ///
    /// This is processed after partial patching is complete.
    #[cfg_attr(
        feature = "serde",
        serde(borrow, bound(deserialize = "Vec<ValueWithPriority<'a>>: serde::Deserialize<'de>"))
    )]
    pub seq: Vec<ValueWithPriority<'a>>,
}
