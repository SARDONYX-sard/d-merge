pub mod deserializer;
mod raw_diff;

use std::borrow::Cow;

use json_patch::{JsonPatchError, apply_seq_array_directly};
use rayon::prelude::*;
use simd_json::{base::ValueTryAsArrayMut as _, borrowed::Value, serde::from_borrowed_value};

use crate::{
    adsf::{normal::ClipAnimDataBlock, patch::de::error::Error},
    asdsf::patch::de::NonNestedArrayDiff,
};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone, PartialEq)]
pub struct ClipAnimDiffPatch<'a> {
    /// str
    name: Option<Cow<'a, str>>,
    /// str
    clip_id: Option<Cow<'a, str>>,

    /// f32
    play_back_speed: Option<Cow<'a, str>>,

    /// f32
    crop_start_local_time: Option<Cow<'a, str>>,

    /// f32
    crop_end_local_time: Option<Cow<'a, str>>,

    #[cfg_attr(
        feature = "serde",
        serde(borrow, bound(deserialize = "NonNestedArrayDiff<'a>: serde::Deserialize<'de>"))
    )]
    trigger_names_patches: NonNestedArrayDiff<'a>,
}

impl ClipAnimDiffPatch<'_> {
    pub fn merge(&mut self, other: Self) {
        if other.name.is_some() {
            self.name = other.name;
        }
        if other.clip_id.is_some() {
            self.clip_id = other.clip_id;
        }
        if other.play_back_speed.is_some() {
            self.play_back_speed = other.play_back_speed;
        }
        if other.crop_start_local_time.is_some() {
            self.crop_start_local_time = other.crop_start_local_time;
        }
        if other.crop_end_local_time.is_some() {
            self.crop_end_local_time = other.crop_end_local_time;
        }

        self.trigger_names_patches.one.merge(other.trigger_names_patches.one);
        self.trigger_names_patches.seq.par_extend(other.trigger_names_patches.seq);
    }
}

impl<'a> ClipAnimDiffPatch<'a> {
    /// Apply the patches to the given `AnimData`.
    ///
    /// # Errors
    /// If the patches cannot be applied due to a mismatch in types or other issues.
    pub fn into_apply(mut self, clip_anim_block: &mut ClipAnimDataBlock<'a>) -> Result<(), Error> {
        if let Some(duration) = self.name {
            clip_anim_block.name = duration;
        }
        if let Some(clip_id) = self.clip_id {
            clip_anim_block.clip_id = clip_id;
        }
        if let Some(play_back_speed) = self.play_back_speed {
            clip_anim_block.play_back_speed = play_back_speed;
        }
        if let Some(crop_start_local_time) = self.crop_start_local_time {
            clip_anim_block.crop_start_local_time = crop_start_local_time;
        }
        if let Some(crop_end_local_time) = self.crop_end_local_time {
            clip_anim_block.crop_end_local_time = crop_end_local_time;
        }

        // trigger_names
        {
            let mut template: Value = core::mem::take(&mut clip_anim_block.trigger_names).into();

            for (path, patch) in self.trigger_names_patches.one.0 {
                json_patch::apply_one_field(&mut template, path, patch)?;
            }

            if !self.trigger_names_patches.seq.is_empty() {
                let patches = core::mem::take(&mut self.trigger_names_patches.seq);
                let template_array = template.try_as_array_mut().map_err(|_| {
                    JsonPatchError::unsupported_range_kind_from(
                        &json_patch::json_path!["trigger_names"],
                        &patches,
                    )
                })?;
                apply_seq_array_directly(template_array, patches)?;
            }
            clip_anim_block.trigger_names = from_borrowed_value(template)?;
            clip_anim_block.trigger_names_len = clip_anim_block.trigger_names.len();
        }

        Ok(())
    }
}
