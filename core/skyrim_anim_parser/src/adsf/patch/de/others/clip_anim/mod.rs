mod current_state;
pub mod deserializer;

use json_patch::{Op, OpRange};
use std::borrow::Cow;

use crate::adsf::normal::ClipAnimDataBlock;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone, PartialEq)]
pub struct ClipAnimDiffPatch<'a> {
    name: Option<Cow<'a, str>>,
    clip_id: Option<Cow<'a, str>>,
    play_back_speed: Option<Cow<'a, str>>,
    crop_start_local_time: Option<Cow<'a, str>>,
    crop_end_local_time: Option<Cow<'a, str>>,
    trigger_names: Option<DiffTriggerNames<'a>>,
}

impl ClipAnimDiffPatch<'_> {
    const DEFAULT: Self = Self {
        name: None,
        clip_id: None,
        play_back_speed: None,
        crop_start_local_time: None,
        crop_end_local_time: None,
        trigger_names: None,
    };

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
        if other.trigger_names.is_some() {
            self.trigger_names = other.trigger_names;
        }
    }
}

impl<'a> ClipAnimDiffPatch<'a> {
    pub fn into_apply(self, anim_block: &mut ClipAnimDataBlock<'a>) {
        if let Some(name) = self.name {
            anim_block.name = name;
        }

        if let Some(clip_id) = self.clip_id {
            anim_block.clip_id = clip_id;
        }

        if let Some(play_back_speed) = self.play_back_speed {
            anim_block.play_back_speed = play_back_speed;
        }

        if let Some(crop_start_local_time) = self.crop_start_local_time {
            anim_block.crop_start_local_time = crop_start_local_time;
        }

        if let Some(crop_end_local_time) = self.crop_end_local_time {
            anim_block.crop_end_local_time = crop_end_local_time;
        }

        if let Some(trigger_patch) = self.trigger_names {
            let OpRange { op, range } = trigger_patch.op.clone();
            match op {
                Op::Add => {
                    if range.start >= anim_block.trigger_names.len() {
                        // Out-of-bounds → append at the end
                        anim_block.trigger_names.extend(trigger_patch.values);
                    } else {
                        // In-bounds → insert at the middle
                        anim_block
                            .trigger_names
                            .splice(range.start..range.start, trigger_patch.values);
                    }
                }
                Op::Replace => {
                    let vec_len = anim_block.trigger_names.len();
                    let start = range.start.min(vec_len);
                    let end = range.end.min(vec_len);

                    let (replace_vals, append_vals) = {
                        let replace_count = end.saturating_sub(start);
                        let mut values = trigger_patch.values.into_iter();
                        let replace_vals: Vec<_> = values.by_ref().take(replace_count).collect();
                        let append_vals: Vec<_> = values.collect();
                        (replace_vals, append_vals)
                    };

                    // Replace within the valid range
                    anim_block.trigger_names.splice(start..end, replace_vals);

                    // Append any remaining values (out-of-range)
                    if !append_vals.is_empty() {
                        anim_block.trigger_names.extend(append_vals);
                    }
                }
                Op::Remove => {
                    let vec_len = anim_block.trigger_names.len();
                    let start = range.start.min(vec_len);
                    let end = range.end.min(vec_len);
                    if start < end {
                        anim_block.trigger_names.drain(start..end);
                    }
                }
            }
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct DiffTriggerNames<'a> {
    op: OpRange,
    values: Vec<Cow<'a, str>>,
}

#[derive(Debug, Clone, Copy, Default)]
pub(super) enum LineKind {
    /// Str
    #[default]
    Name,
    /// Str
    ClipId,
    /// f32
    PlayBackSpeed,

    /// f32
    CropStartLocalTime,
    /// f32
    CropEndLocalTime,

    /// usize
    TriggerNamesLen,
    /// Vec<Str>
    TriggerNames,
}
