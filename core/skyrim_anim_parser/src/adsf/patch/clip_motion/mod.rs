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

        if let Some(trans_patch) = self.translations {
            let OpRange { op, range } = trans_patch.op.clone();
            match op {
                Op::Add => {
                    motion_block
                        .translations
                        .splice(range.start..range.start, trans_patch.values);
                }
                Op::Replace => {
                    motion_block.translations.splice(range, trans_patch.values);
                }
                Op::Remove => {
                    motion_block.translations.drain(range);
                }
            }
        }

        if let Some(rot_patch) = self.rotations {
            let OpRange { op, range } = rot_patch.op.clone();
            match op {
                Op::Add => {
                    motion_block
                        .rotations
                        .splice(range.start..range.start, rot_patch.values);
                }
                Op::Replace => {
                    motion_block.rotations.splice(range, rot_patch.values);
                }
                Op::Remove => {
                    motion_block.rotations.drain(range);
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
