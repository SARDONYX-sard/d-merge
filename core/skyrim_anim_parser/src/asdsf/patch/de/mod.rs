mod current_state;
pub mod deserializer;
mod error;

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
            bound(deserialize = "Vec<ValueWithPriority<'a>>: serde::Deserialize<'de>")
        )
    )]
    conditions_patches: Vec<ValueWithPriority<'a>>,

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
            bound(deserialize = "Vec<ValueWithPriority<'a>>: serde::Deserialize<'de>")
        )
    )]
    anim_infos_patches: Vec<ValueWithPriority<'a>>,
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

impl DiffPatchAnimSetData<'_> {
    #[inline]
    pub fn merge(&mut self, other: Self) {
        if other.version.is_some() {
            self.version = other.version;
        }
        if !other.triggers_patches.is_empty() {
            self.triggers_patches.par_extend(other.triggers_patches);
        }
        if !other.conditions_patches.is_empty() {
            self.conditions_patches.par_extend(other.conditions_patches);
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

        if !other.anim_infos_patches.is_empty() {
            self.anim_infos_patches.par_extend(other.anim_infos_patches);
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

            apply_seq_by_priority(
                "triggers",
                &mut template,
                &vec!["triggers".into()],
                patches,
                vec![],
            )?;

            anim_set_data.triggers = from_borrowed_value(template)?;
        }

        if !self.conditions_patches.is_empty() {
            // take & change condition to json -> marge
            let mut template = to_borrowed_value(core::mem::take(&mut anim_set_data.conditions))?;
            let patches = core::mem::take(&mut self.conditions_patches);

            apply_seq_by_priority(
                "conditions",
                &mut template,
                &vec!["conditions".into()],
                patches,
                vec![],
            )?;

            anim_set_data.conditions = from_borrowed_value(template)?;
        }

        if !self.attacks_patches.base.is_empty() || !self.attacks_patches.children.is_empty() {
            let mut template = to_borrowed_value(core::mem::take(&mut anim_set_data.attacks))?;
            let NestedPatches { base, children } = core::mem::take(&mut self.attacks_patches);

            apply_seq_by_priority(
                "attacks",
                &mut template,
                &vec!["attacks".into()],
                base,
                children,
            )?;

            anim_set_data.conditions = from_borrowed_value(template)?;
        }

        if !self.anim_infos_patches.is_empty() {
            // take & change condition to json -> marge
            let patches = core::mem::take(&mut self.anim_infos_patches);

            let anim_infos = core::mem::take(&mut anim_set_data.anim_infos);
            let mut template = to_borrowed_value(anim_infos)?;
            apply_seq_by_priority(
                "anim_infos",
                &mut template,
                &vec!["anim_infos".into()],
                patches,
                vec![],
            )?;

            anim_set_data.anim_infos = from_borrowed_value(template)?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(super) enum LineKind {
    /// Str
    #[default]
    Version,

    /// usize
    TriggersLen,
    /// Vec<Str>
    Triggers,

    /// usize
    ConditionsLen,
    /// Vec<Condition>
    Conditions,

    /// usize
    AttacksLen,
    /// Vec<Attack>
    Attacks,

    /// usize
    AnimInfosLen,
    /// Vec<AnimInfo>
    AnimInfos,
}
