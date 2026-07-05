pub mod deserializer;
mod raw_diff;

use std::borrow::Cow;

use json_patch::{JsonPatchError, apply_seq_array_directly};
use rayon::prelude::*;
use simd_json::{base::ValueTryAsArrayMut as _, borrowed::Value, serde::from_borrowed_value};

use crate::{
    adsf::{normal::ClipMotionBlock, patch::de::error::Error},
    asdsf::patch::de::NonNestedArrayDiff,
};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone, PartialEq)]
pub struct ClipMotionDiffPatch<'a> {
    clip_id: Option<Cow<'a, str>>,
    duration: Option<Cow<'a, str>>,

    #[cfg_attr(
        feature = "serde",
        serde(borrow, bound(deserialize = "NonNestedArrayDiff<'a>: serde::Deserialize<'de>"))
    )]
    translations_patches: NonNestedArrayDiff<'a>,
    #[cfg_attr(
        feature = "serde",
        serde(borrow, bound(deserialize = "NonNestedArrayDiff<'a>: serde::Deserialize<'de>"))
    )]
    rotations_patches: NonNestedArrayDiff<'a>,
}

impl ClipMotionDiffPatch<'_> {
    pub fn merge(&mut self, other: Self) {
        if other.clip_id.is_some() {
            self.clip_id = other.clip_id;
        }
        if other.duration.is_some() {
            self.duration = other.duration;
        }

        self.translations_patches.one.merge(other.translations_patches.one);
        self.translations_patches.seq.par_extend(other.translations_patches.seq);

        self.rotations_patches.one.merge(other.rotations_patches.one);
        self.rotations_patches.seq.par_extend(other.rotations_patches.seq);
    }
}

impl<'a> ClipMotionDiffPatch<'a> {
    /// Apply the patches to the given `AnimData`.
    ///
    /// # Errors
    /// If the patches cannot be applied due to a mismatch in types or other issues.
    pub fn into_apply(mut self, motion_block: &mut ClipMotionBlock<'a>) -> Result<(), Error> {
        if let Some(clip_id) = self.clip_id {
            motion_block.clip_id = clip_id;
        }

        if let Some(duration) = self.duration {
            motion_block.duration = duration;
        }

        // translations
        {
            let mut template: Value = core::mem::take(&mut motion_block.translations).into();

            for (path, patch) in self.translations_patches.one.0 {
                json_patch::apply_one_field(&mut template, path, patch)?;
            }

            if !self.translations_patches.seq.is_empty() {
                let patches = core::mem::take(&mut self.translations_patches.seq);
                let template_array = template.try_as_array_mut().map_err(|_| {
                    JsonPatchError::unsupported_range_kind_from(
                        &json_patch::json_path!["translations"],
                        &patches,
                    )
                })?;
                apply_seq_array_directly(template_array, patches)?;
            }
            motion_block.translations = from_borrowed_value(template)?;
            motion_block.translation_len = motion_block.translations.len();
        }

        // rotations
        {
            let mut template: Value = core::mem::take(&mut motion_block.rotations).into();

            for (path, patch) in self.rotations_patches.one.0 {
                json_patch::apply_one_field(&mut template, path, patch)?;
            }

            if !self.rotations_patches.seq.is_empty() {
                let patches = core::mem::take(&mut self.rotations_patches.seq);
                let template_array = template.try_as_array_mut().map_err(|_| {
                    JsonPatchError::unsupported_range_kind_from(
                        &json_patch::json_path!["rotations"],
                        &patches,
                    )
                })?;
                apply_seq_array_directly(template_array, patches)?;
            }
            motion_block.rotations = from_borrowed_value(template)?;
            motion_block.rotation_len = motion_block.rotations.len();
        }

        Ok(())
    }
}
