//! `animationdatasinglefile.txt` patch types (shared by Nemesis and FNIS patch generation).

use skyrim_anim_data::{
    adsf::{
        normal::{ClipAnimDataBlock, ClipMotionBlock},
        patch::de::{
            anim_header::AnimHeaderDiffPatch,
            others::{clip_anim::ClipAnimDiffPatch, clip_motion::ClipMotionDiffPatch},
        },
    },
    diff_line::DiffLines,
};
#[derive(serde::Serialize, Debug, Default, Clone, PartialEq)]
pub struct AdsfPatch<'a> {
    /// The animation data target.
    ///
    /// Example: `FirstPerson~1`.
    pub target: &'a str,

    /// The Nemesis mod ID.
    pub id: &'a str,

    pub patch: PatchKind<'a>,
}

#[derive(serde::Serialize, Debug, Clone, PartialEq)]
pub enum PatchKind<'a> {
    /// Patch the project names header.
    ProjectNamesHeader(DiffLines<'a>),

    /// Patch the animation data header.
    AnimDataHeader(AnimHeaderDiffPatch<'a>),

    /// Add an animation whose clip ID is assigned later.
    AddAnim(ClipAnimDataBlock<'a>),

    /// Edit an existing animation.
    EditAnim(EditAnim<'a>),

    /// Add an animation with an explicitly specified clip ID.
    ///
    /// Unlike [`AddAnim`], the clip ID must be preserved exactly and is
    /// inserted into `clip_anim_blocks` during patch application.
    SpecifiedAnim {
        /// `<name>~<clip_id>`.
        ///
        /// Example: `Jump~42`.
        name_clip: &'a str,

        /// Animation block.
        patch: ClipAnimDataBlock<'a>,
        priority: usize,
    },

    /// Add a motion whose clip ID is assigned later.
    AddMotion(ClipMotionBlock<'a>),

    /// Edit an existing motion.
    EditMotion(EditMotion<'a>),

    /// Add a motion with an explicitly specified index.
    ///
    /// Unlike [`AddMotion`], the index must be preserved exactly and is
    /// inserted into `clip_motion_blocks` during patch application.
    SpecifiedMotion {
        /// Explicitly specified motion index.
        index: &'a str,

        /// Motion block.
        patch: ClipMotionBlock<'a>,
        priority: usize,
    },
}

#[derive(serde::Serialize, Debug, Default, Clone, PartialEq)]
pub struct EditAnim<'a> {
    pub patch: ClipAnimDiffPatch<'a>,
    pub priority: usize,

    /// `<name>~<clip_id>`.
    ///
    /// Example: `Jump~42`.
    pub name_clip: &'a str,
}

#[derive(serde::Serialize, Debug, Default, Clone, PartialEq)]
pub struct EditMotion<'a> {
    pub patch: ClipMotionDiffPatch<'a>,
    pub priority: usize,

    /// Explicit motion index.
    pub clip_id: &'a str,
}

impl<'a> Default for PatchKind<'a> {
    #[inline]
    fn default() -> Self {
        Self::AddAnim(ClipAnimDataBlock::default())
    }
}
