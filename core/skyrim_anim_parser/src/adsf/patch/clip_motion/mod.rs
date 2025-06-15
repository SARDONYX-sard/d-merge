pub mod current_state;
pub mod deserializer;

use std::borrow::Cow;

use json_patch::{Op, OpRange};

use crate::adsf::{ClipMotionBlock, Rotation, Translation};

#[derive(Debug, Default, Clone, PartialEq)]
pub struct ClipMotionDiffPatch<'a> {
    clip_id: Option<Cow<'a, str>>,
    duration: Option<Cow<'a, str>>,
    translations: Option<DiffTransitions<'a>>,
    rotations: Option<DiffRotations<'a>>,
}

impl ClipMotionDiffPatch<'_> {
    const DEFAULT: Self = Self {
        clip_id: None,
        duration: None,
        translations: None,
        rotations: None,
    };

    pub fn merge(&mut self, other: Self) {
        if other.clip_id.is_some() {
            self.clip_id = other.clip_id;
        }
        if other.duration.is_some() {
            self.duration = other.duration;
        }
        if other.translations.is_some() {
            self.translations = other.translations;
        }
        if other.rotations.is_some() {
            self.rotations = other.rotations;
        }
    }
}

impl<'a> ClipMotionDiffPatch<'a> {
    pub fn into_apply(self, motion_block: &mut ClipMotionBlock<'a>) {
        if let Some(clip_id) = self.clip_id {
            motion_block.clip_id = clip_id;
        }

        if let Some(duration) = self.duration {
            motion_block.duration = duration;
        }

        if let Some(translations) = self.translations {
            let OpRange { op, range } = translations.op.clone();
            match op {
                Op::Add => {
                    if range.start >= motion_block.translations.len() {
                        // Out-of-bounds → append at the end
                        motion_block.translations.extend(translations.values);
                    } else {
                        // In-bounds → insert at the middle
                        motion_block
                            .translations
                            .splice(range.start..range.start, translations.values);
                    }
                }
                Op::Replace => {
                    let vec_len = motion_block.translations.len();
                    let start = range.start.min(vec_len);
                    let end = range.end.min(vec_len);

                    let (replace_vals, append_vals) = {
                        let replace_count = end.saturating_sub(start);
                        let mut values = translations.values.into_iter();
                        let replace_vals: Vec<_> = values.by_ref().take(replace_count).collect();
                        let append_vals: Vec<_> = values.collect();
                        (replace_vals, append_vals)
                    };

                    // Replace within the valid range
                    motion_block.translations.splice(start..end, replace_vals);

                    // Append any remaining values (out-of-range)
                    if !append_vals.is_empty() {
                        motion_block.translations.extend(append_vals);
                    }
                }
                Op::Remove => {
                    let vec_len = motion_block.translations.len();
                    let start = range.start.min(vec_len);
                    let end = range.end.min(vec_len);
                    if start < end {
                        motion_block.translations.drain(start..end);
                    }
                }
            }
        }

        if let Some(rotations) = self.rotations {
            let OpRange { op, range } = rotations.op.clone();
            match op {
                Op::Add => {
                    if range.start >= motion_block.rotations.len() {
                        // Out-of-bounds → append at the end
                        motion_block.rotations.extend(rotations.values);
                    } else {
                        // In-bounds → insert at the middle
                        motion_block
                            .rotations
                            .splice(range.start..range.start, rotations.values);
                    }
                }
                Op::Replace => {
                    let vec_len = motion_block.rotations.len();
                    let start = range.start.min(vec_len);
                    let end = range.end.min(vec_len);

                    let (replace_vals, append_vals) = {
                        let replace_count = end.saturating_sub(start);
                        let mut values = rotations.values.into_iter();
                        let replace_vals: Vec<_> = values.by_ref().take(replace_count).collect();
                        let append_vals: Vec<_> = values.collect();
                        (replace_vals, append_vals)
                    };

                    // Replace within the valid range
                    motion_block.rotations.splice(start..end, replace_vals);

                    // Append any remaining values (out-of-range)
                    if !append_vals.is_empty() {
                        motion_block.rotations.extend(append_vals);
                    }
                }
                Op::Remove => {
                    let vec_len = motion_block.rotations.len();
                    let start = range.start.min(vec_len);
                    let end = range.end.min(vec_len);
                    if start < end {
                        motion_block.rotations.drain(start..end);
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiffTransitions<'a> {
    op: OpRange,
    values: Vec<Translation<'a>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiffRotations<'a> {
    op: OpRange,
    values: Vec<Rotation<'a>>,
}

#[derive(Debug, Clone, Copy, Default)]
enum LineKind {
    #[default]
    ClipId,
    Duration,

    TranslationLen,
    Translation,

    RotationLen,
    Rotation,
}
