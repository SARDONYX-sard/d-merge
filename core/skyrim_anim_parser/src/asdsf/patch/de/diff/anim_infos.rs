use crate::{asdsf::patch::de::patch_map::OnePatchMap, common_parser::lines::Str};
use json_patch::ValueWithPriority;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone, PartialEq)]
pub(crate) struct NonNestedArrayDiff<'a> {
    /// modifying fields of a single.
    ///
    /// - key(replace target index): e.g.,
    ///   - ["[0]", "value_a"]
    ///   - ["[0]", "hashed_path"]
    #[cfg_attr(
        feature = "serde",
        serde(
            borrow,
            bound(deserialize = "OnePatchMap<'a>: serde::Deserialize<'de>")
        )
    )]
    pub one: OnePatchMap<'a>,

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
pub struct AnimInfoDiff<'a> {
    /// CRC32 representation path
    /// - type: [`u32`]
    pub hashed_path: Option<Str<'a>>,
    /// CRC32 representation file name
    /// - type: [`u32`]
    pub hashed_file_name: Option<Str<'a>>,
    /// Always `7891816`
    /// - type: [`u32`]
    pub ascii_extension: Option<Str<'a>>,
}
