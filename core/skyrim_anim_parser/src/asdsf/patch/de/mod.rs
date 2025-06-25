mod current_state;
pub mod deserializer;
mod error;

use std::collections::HashMap;

use self::error::Error;
use json_patch::{apply_seq_by_priority, JsonPath, ValueWithPriority};
use rayon::prelude::*;
use simd_json::serde::{from_borrowed_value, to_borrowed_value};

use crate::{asdsf::normal::AnimSetData, common_parser::lines::Str};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone, PartialEq)]
pub struct DiffPatchAnimSetData<'a> {
    version: Option<Str<'a>>,

    /// # Why are triggers vec?
    /// This is so that multiple seq patches can be resolved simultaneously later.
    #[cfg_attr(
        feature = "serde",
        serde(
            borrow,
            bound(deserialize = "Vec<ValueWithPriority<'a>>: serde::Deserialize<'de>")
        )
    )]
    triggers_patches: Vec<ValueWithPriority<'a>>,
    #[cfg_attr(
        feature = "serde",
        serde(
            borrow,
            bound(deserialize = "ConditionsDiff<'a>: serde::Deserialize<'de>")
        )
    )]
    conditions_patches: ConditionsDiff<'a>,

    #[cfg_attr(
        feature = "serde",
        serde(
            borrow,
            bound(deserialize = "NestedPatches<'a>: serde::Deserialize<'de>")
        )
    )]
    attacks_patches: NestedPatches<'a>,

    #[cfg_attr(
        feature = "serde",
        serde(
            borrow,
            bound(deserialize = "AnimInfosDiff<'a>: serde::Deserialize<'de>")
        )
    )]
    anim_infos_patches: AnimInfosDiff<'a>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone, PartialEq)]
pub struct ConditionsDiff<'a> {
    /// - key: replace target index
    /// - value: Partial change request
    one: HashMap<usize, ConditionDiff<'a>>,

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
    seq: Vec<ValueWithPriority<'a>>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ConditionDiff<'a> {
    /// The name of the variable used in the condition.
    pub variable_name: Option<Str<'a>>,

    /// The **start** of the allowed range (inclusive) for the condition value.
    /// - type: [`i32`]
    pub value_a: Option<Str<'a>>,
    /// - type: [`i32`]
    pub value_b: Option<Str<'a>>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone, PartialEq)]
struct NestedPatches<'a> {
    /// struct Attack
    /// - sample json_path: `["attacks"]`
    #[cfg_attr(
        feature = "serde",
        serde(
            borrow,
            bound(deserialize = "Vec<ValueWithPriority<'a>>: serde::Deserialize<'de>")
        )
    )]
    base: Vec<ValueWithPriority<'a>>,

    /// - sample json_path: `["clip_names", ["0"]]`
    #[cfg_attr(
        feature = "serde",
        serde(
            borrow,
            bound(
                deserialize = "Vec<(JsonPath<'a>, ValueWithPriority<'a>)>: serde::Deserialize<'de>"
            )
        )
    )]
    children: Vec<(JsonPath<'a>, ValueWithPriority<'a>)>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone, PartialEq)]
pub struct AnimInfosDiff<'a> {
    /// - key: replace target index
    /// - value: Partial change request
    one: HashMap<usize, AnimInfoDiff<'a>>,

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
    seq: Vec<ValueWithPriority<'a>>,
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

impl DiffPatchAnimSetData<'_> {
    #[inline]
    pub fn merge(&mut self, other: Self) {
        if other.version.is_some() {
            self.version = other.version;
        }
        if !other.triggers_patches.is_empty() {
            self.triggers_patches.par_extend(other.triggers_patches);
        }

        if !other.conditions_patches.one.is_empty() {
            self.conditions_patches
                .one
                .par_extend(other.conditions_patches.one);
        }
        if !other.conditions_patches.seq.is_empty() {
            self.conditions_patches
                .seq
                .par_extend(other.conditions_patches.seq);
        }

        if !other.attacks_patches.base.is_empty() {
            self.attacks_patches
                .base
                .par_extend(other.attacks_patches.base);
        }
        if !other.attacks_patches.children.is_empty() {
            self.attacks_patches
                .children
                .par_extend(other.attacks_patches.children);
        }

        if !other.anim_infos_patches.one.is_empty() {
            self.anim_infos_patches
                .one
                .par_extend(other.anim_infos_patches.one);
        }
        if !other.anim_infos_patches.seq.is_empty() {
            self.anim_infos_patches
                .seq
                .par_extend(other.anim_infos_patches.seq);
        }
    }
}

impl<'a> DiffPatchAnimSetData<'a> {
    /// Apply the patches to the given `AnimSetData`.
    ///
    /// # Errors
    /// If the patches cannot be applied due to a mismatch in types or other issues.
    pub fn into_apply(mut self, anim_set_data: &mut AnimSetData<'a>) -> Result<(), Error> {
        if let Some(version) = self.version {
            anim_set_data.version = version;
        }

        if !self.triggers_patches.is_empty() {
            // take & change condition to json -> marge
            let mut template = to_borrowed_value(core::mem::take(&mut anim_set_data.triggers))?;
            let patches = core::mem::take(&mut self.triggers_patches);

            apply_seq_by_priority("triggers", &mut template, vec!["triggers".into()], patches)?;

            anim_set_data.triggers = from_borrowed_value(template)?;
        }

        if !self.conditions_patches.seq.is_empty() {
            // take & change condition to json -> marge
            let mut template = to_borrowed_value(core::mem::take(&mut anim_set_data.conditions))?;
            let patches = core::mem::take(&mut self.conditions_patches.seq);

            apply_seq_by_priority(
                "conditions",
                &mut template,
                vec!["conditions".into()],
                patches,
            )?;

            anim_set_data.conditions = from_borrowed_value(template)?;
        }

        if !self.attacks_patches.base.is_empty() || !self.attacks_patches.children.is_empty() {
            let mut template = to_borrowed_value(core::mem::take(&mut anim_set_data.attacks))?;
            let NestedPatches { base, children: _ } = core::mem::take(&mut self.attacks_patches);

            apply_seq_by_priority("attacks", &mut template, vec!["attacks".into()], base)?;

            anim_set_data.conditions = from_borrowed_value(template)?;
        }

        if !self.anim_infos_patches.seq.is_empty() {
            // take & change condition to json -> marge
            let patches = core::mem::take(&mut self.anim_infos_patches.seq);

            let anim_infos = core::mem::take(&mut anim_set_data.anim_infos);
            let mut template = to_borrowed_value(anim_infos)?;
            apply_seq_by_priority(
                "anim_infos",
                &mut template,
                vec!["anim_infos".into()],
                patches,
            )?;

            anim_set_data.anim_infos = from_borrowed_value(template)?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(super) enum ParserKind {
    /// - type: `Str`
    #[default]
    Version,

    /// - type: `usize`
    TriggersLen,
    /// - type: `Vec<Str>`
    Triggers,

    /// - type: [`usize`]
    ConditionsLen,
    /// - type: `Vec<Condition>`
    Conditions,

    /// - type: [`usize`]
    AttacksLen,
    /// - type: `Vec<Attack>`
    Attacks,

    /// - type: [`usize`]
    AnimInfosLen,
    /// - type: `Vec<AnimInfo>`
    AnimInfos,
}
