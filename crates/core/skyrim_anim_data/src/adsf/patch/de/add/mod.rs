//! Parsers for animation data patch files.

mod clip_anim;
mod clip_motion;
mod common;

pub use clip_anim::parse_clip_anim_block_patch;
pub use clip_motion::parse_clip_motion_block_patch;
