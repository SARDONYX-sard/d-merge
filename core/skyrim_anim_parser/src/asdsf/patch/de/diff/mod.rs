mod anim_infos;
mod conditions;
mod error;

pub use self::anim_infos::*;
pub use self::conditions::*;
pub use self::error::*;

use crate::asdsf::normal::AnimSetData;
use crate::asdsf::patch::de::error::Error;
use crate::common_parser::lines::Str;
use json_patch::{apply_seq_array_directly, ValueWithPriority};
use rayon::prelude::*;
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
            bound(deserialize = "ConditionsDiff<'a>: serde::Deserialize<'de>")
        )
    )]
    pub(crate) conditions_patches: ConditionsDiff<'a>,

    pub(crate) attacks_patches: (),

    #[cfg_attr(
        feature = "serde",
        serde(
            borrow,
            bound(deserialize = "AnimInfosDiff<'a>: serde::Deserialize<'de>")
        )
    )]
    pub(crate) anim_infos_patches: AnimInfosDiff<'a>,
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

        if !self.conditions_patches.seq.is_empty() {
            let patches = core::mem::take(&mut self.conditions_patches.seq);

            let conditions = core::mem::take(&mut anim_set_data.conditions);
            let mut template: Vec<Value> = conditions.into_iter().map(Into::into).collect();
            apply_seq_array_directly(&mut template, patches)?;
            anim_set_data.conditions = from_borrowed_value(template.into())?;
        }

        if !self.anim_infos_patches.seq.is_empty() {
            let patches = core::mem::take(&mut self.anim_infos_patches.seq);

            let anim_infos = core::mem::take(&mut anim_set_data.anim_infos);
            let mut template: Vec<Value> = anim_infos.into_par_iter().map(Into::into).collect();
            apply_seq_array_directly(&mut template, patches)?;
            anim_set_data.anim_infos = from_borrowed_value(template.into())?;
        }

        Ok(())
    }
}
