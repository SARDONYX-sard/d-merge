mod anim_infos;
mod attacks;
pub(crate) mod patch_map;

pub use self::anim_infos::*;
pub use self::attacks::*;

use crate::asdsf::normal::AnimSetData;
use crate::asdsf::patch::de::error::Error;
use crate::common_parser::lines::Str;
use json_patch::JsonPatchError;
use json_patch::{apply_seq_array_directly, ValueWithPriority};
use rayon::prelude::*;
use simd_json::base::ValueTryAsArrayMut;
use simd_json::borrowed::Value;
use simd_json::serde::from_borrowed_value;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone, PartialEq)]
pub struct DiffPatchAnimSetData<'a> {
    pub(crate) version: Option<Str<'a>>,

    /// # Why are triggers vec?
    /// This is so that multiple seq patches can be resolved simultaneously later.
    #[cfg_attr(
        feature = "serde",
        serde(
            borrow,
            bound(deserialize = "Vec<ValueWithPriority<'a>>: serde::Deserialize<'de>")
        )
    )]
    pub(crate) triggers_patches: Vec<ValueWithPriority<'a>>,

    #[cfg_attr(
        feature = "serde",
        serde(
            borrow,
            bound(deserialize = "NonNestedArrayDiff<'a>: serde::Deserialize<'de>")
        )
    )]
    pub(crate) conditions_patches: NonNestedArrayDiff<'a>,

    #[cfg_attr(
        feature = "serde",
        serde(
            borrow,
            bound(deserialize = "AttacksDiff<'a>: serde::Deserialize<'de>")
        )
    )]
    pub(crate) attacks_patches: AttacksDiff<'a>,

    #[cfg_attr(
        feature = "serde",
        serde(
            borrow,
            bound(deserialize = "NonNestedArrayDiff<'a>: serde::Deserialize<'de>")
        )
    )]
    pub(crate) anim_infos_patches: NonNestedArrayDiff<'a>,
}

impl<'a> DiffPatchAnimSetData<'a> {
    #[inline]
    pub fn merge(&mut self, other: Self) {
        if other.version.is_some() {
            self.version = other.version;
        }
        if !other.triggers_patches.is_empty() {
            self.triggers_patches.par_extend(other.triggers_patches);
        }

        for (path, value) in other.conditions_patches.one.0 {
            self.conditions_patches.one.insert(path, value);
        }

        if !other.conditions_patches.seq.is_empty() {
            self.conditions_patches
                .seq
                .par_extend(other.conditions_patches.seq);
        }

        self.attacks_patches.one.merge(other.attacks_patches.one);
        self.attacks_patches.seq.merge(other.attacks_patches.seq);

        for (path, value) in other.anim_infos_patches.one.0 {
            self.anim_infos_patches.one.insert(path, value);
        }

        if !other.anim_infos_patches.seq.is_empty() {
            self.anim_infos_patches
                .seq
                .par_extend(other.anim_infos_patches.seq);
        }
    }

    /// Apply the patches to the given `AnimSetData`.
    ///
    /// # Errors
    /// If the patches cannot be applied due to a mismatch in types or other issues.
    pub fn into_apply(mut self, anim_set_data: &mut AnimSetData<'a>) -> Result<(), Error> {
        if let Some(version) = self.version {
            anim_set_data.version = version;
        }

        // take & change condition to json -> marge
        if !self.triggers_patches.is_empty() {
            let patches = core::mem::take(&mut self.triggers_patches);

            let triggers = core::mem::take(&mut anim_set_data.triggers);
            let mut template: Vec<Value> = triggers.into_iter().map(Into::into).collect();
            apply_seq_array_directly(&mut template, patches)?;
            anim_set_data.triggers = from_borrowed_value(template.into())?;
        }

        // Conditions
        {
            let mut template_value: Value = core::mem::take(&mut anim_set_data.conditions).into();

            for (path, patch) in self.conditions_patches.one.0 {
                json_patch::apply_one_field(&mut template_value, path, patch)?;
            }

            if !self.conditions_patches.seq.is_empty() {
                let patches = core::mem::take(&mut self.conditions_patches.seq);
                let template_array = template_value.try_as_array_mut().map_err(|_| {
                    JsonPatchError::unsupported_range_kind_from(
                        &json_patch::json_path!["conditions"],
                        &patches,
                    )
                })?;
                apply_seq_array_directly(template_array, patches)?;
            }
            anim_set_data.conditions = from_borrowed_value(template_value)?;
        }

        // Attacks
        if !self.attacks_patches.one.0.is_empty() {
            let attacks = core::mem::take(&mut anim_set_data.attacks);
            let mut template_value = attacks_to_borrowed_value(attacks);

            let AttacksDiff {
                one: one_patch_map,
                seq: seq_patch_map,
            } = self.attacks_patches;
            for (path, patch) in one_patch_map.0 {
                json_patch::apply_one_field(&mut template_value, path, patch)?;
            }
            for (path, patches) in seq_patch_map.0 {
                if path.is_empty() {
                    let template_array = template_value.try_as_array_mut().map_err(|_| {
                        JsonPatchError::unsupported_range_kind_from(&path, &patches)
                    })?;
                    json_patch::apply_seq_array_directly(template_array, patches)?;
                } else {
                    json_patch::apply_seq_by_priority(
                        "attacks",
                        &mut template_value,
                        path,
                        patches,
                    )?;
                }
            }

            anim_set_data.attacks = from_borrowed_value(template_value)?;
        }

        // Anim Infos
        {
            let mut template_value: Value = core::mem::take(&mut anim_set_data.anim_infos).into();

            for (path, patch) in self.anim_infos_patches.one.0 {
                json_patch::apply_one_field(&mut template_value, path, patch)?;
            }

            if !self.anim_infos_patches.seq.is_empty() {
                let patches = core::mem::take(&mut self.anim_infos_patches.seq);
                let template_array = template_value.try_as_array_mut().map_err(|_| {
                    JsonPatchError::unsupported_range_kind_from(
                        &json_patch::json_path!["anim_infos"],
                        &patches,
                    )
                })?;
                apply_seq_array_directly(template_array, patches)?;
            }
            anim_set_data.anim_infos = from_borrowed_value(template_value)?;
        }

        Ok(())
    }
}
