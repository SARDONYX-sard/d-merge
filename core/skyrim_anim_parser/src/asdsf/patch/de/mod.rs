#![allow(unused)] // TODO: Remove this line.
mod current_state;
// pub mod deserializer;
mod error;

use self::error::Error;
use json_patch::{apply_seq_by_priority, ValueWithPriority};
use rayon::prelude::*;

use crate::{asdsf::normal::AnimSetData, common_parser::lines::Str};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone, PartialEq)]
pub struct DiffPatchAnimSetData<'a> {
    version: Option<Str<'a>>,
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
            bound(deserialize = "Vec<ValueWithPriority<'a>>: serde::Deserialize<'de>")
        )
    )]
    attacks_patches: Vec<ValueWithPriority<'a>>,

    #[cfg_attr(
        feature = "serde",
        serde(
            borrow,
            bound(deserialize = "Vec<ValueWithPriority<'a>>: serde::Deserialize<'de>")
        )
    )]
    anim_infos_patches: Vec<ValueWithPriority<'a>>,
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
        if !other.attacks_patches.is_empty() {
            self.attacks_patches.par_extend(other.attacks_patches);
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
            let patches = core::mem::take(&mut self.triggers_patches);

            let triggers = core::mem::take(&mut anim_set_data.triggers);
            let mut template = simd_json::serde::to_borrowed_value(triggers)?;
            apply_seq_by_priority("triggers", &mut template, vec!["triggers".into()], patches)?;
            anim_set_data.triggers = simd_json::serde::from_borrowed_value(template)?;
        }

        if !self.triggers_patches.is_empty() {
            // take & change condition to json -> marge
            let patches = core::mem::take(&mut self.conditions_patches);

            let conditions = core::mem::take(&mut anim_set_data.conditions);
            let mut template = simd_json::serde::to_borrowed_value(conditions)?;
            apply_seq_by_priority(
                "conditions",
                &mut template,
                vec!["conditions".into()],
                patches,
            )?;

            anim_set_data.conditions = simd_json::serde::from_borrowed_value(template)?;
        }

        if !self.anim_infos_patches.is_empty() {
            // take & change condition to json -> marge
            let patches = core::mem::take(&mut self.anim_infos_patches);

            let anim_infos = core::mem::take(&mut anim_set_data.anim_infos);
            let mut template = simd_json::serde::to_borrowed_value(anim_infos)?;
            apply_seq_by_priority(
                "anim_infos",
                &mut template,
                vec!["anim_infos".into()],
                patches,
            )?;

            anim_set_data.anim_infos = simd_json::serde::from_borrowed_value(template)?;
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
