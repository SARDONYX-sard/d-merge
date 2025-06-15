#[cfg(feature = "alt_map")]
mod alt;
pub mod clip_id_manager;
pub mod de;
pub mod patch;
pub mod ser;

#[cfg(feature = "alt_map")]
pub use alt::{to_adsf_key, AltAdsf, AltAnimData};

use crate::lines::Str;
use rayon::prelude::*;

// NOTE: Since f32 is very slow if it is made into str, only check that it is f32 and allocate it as `&str`.

/// Represents the entire animation data structure.
///
/// This structure contains the names of the projects and a list of associated
/// animation data.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Adsf<'a> {
    /// A list of project names parsed from the input.
    pub project_names: Vec<Str<'a>>,

    /// A list of animation data corresponding to each project.
    pub anim_list: Vec<AnimData<'a>>,
}

/// Represents individual animation data.
///
/// This structure holds the header information for the animation and the
/// associated clip animation and motion blocks.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone, PartialEq)]
pub struct AnimData<'a> {
    /// The header containing metadata about the animation data.
    pub header: AnimDataHeader<'a>,

    /// A list of animation blocks corresponding to the clips.
    pub clip_anim_blocks: Vec<ClipAnimDataBlock<'a>>,

    /// It must be added at the beginning, but `Vec::insert` is slow.
    /// Therefore, another additional field is created and it is added first.
    ///
    /// # Note
    /// This is used during the patch merge phase.
    pub add_clip_anim_blocks: Vec<ClipAnimDataBlock<'a>>,

    /// A list of motion blocks corresponding to the clips.
    pub clip_motion_blocks: Vec<ClipMotionBlock<'a>>,

    /// It must be added at the beginning, but `Vec::insert` is slow.
    /// Therefore, another additional field is created and it is added first.
    ///
    /// # Note
    /// This is used during the patch merge phase.
    pub add_clip_motion_blocks: Vec<ClipMotionBlock<'a>>,
}

impl AnimData<'_> {
    /// Returns the number of lines when serialized.
    ///
    /// ```txt
    /// 1(header) + n(clip_anim_blocks) + n(clip_motion_blocks)
    /// = 1 + n_1 + n_2
    /// ```
    fn to_line_range(&self) -> usize {
        (self.header.to_line_len() - 1) + self.clip_anim_blocks_line_len()
    }

    pub(crate) fn clip_anim_blocks_line_len(&self) -> usize {
        // NOTE: `.zip()` is not used here because it must be the same length.
        let len: usize = self
            .clip_anim_blocks
            .par_iter()
            .map(|block| block.to_line_len())
            .sum();
        let add_len: usize = self
            .add_clip_anim_blocks
            .par_iter()
            .map(|block| block.to_line_len())
            .sum();
        len + add_len
    }

    pub(crate) fn clip_motion_blocks_line_len(&self) -> usize {
        // NOTE: `.zip()` is not used here because it must be the same length.
        let len: usize = self
            .clip_motion_blocks
            .par_iter()
            .map(|block| block.to_line_len())
            .sum();
        let add_len: usize = self
            .add_clip_motion_blocks
            .par_iter()
            .map(|block| block.to_line_len())
            .sum();
        len + add_len
    }
}

/// Represents the header of animation data.
///
/// This structure contains metadata related to the animation data, such as
/// the number of lines remaining, asset count, and project assets.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone, PartialEq)]
pub struct AnimDataHeader<'a> {
    /// An integer value related to the animation (meaning may vary based on context).
    pub lead_int: i32,

    /// A list of project asset names.
    pub project_assets: Vec<Str<'a>>,

    /// Indicates whether motion data is available.
    pub has_motion_data: bool,
}

impl AnimDataHeader<'_> {
    /// Returns the number of lines when serialized.
    ///
    /// ```txt
    /// 1(line_range) + 1(lead_int) + 1(project_assets_len size)
    ///         + n(project_assets_len) + 1(empty_line)
    /// = 4 + n
    /// ```
    const fn to_line_len(&self) -> usize {
        3 + self.project_assets.len() + 1
    }
}

/// Represents a clip animation data block.
///
/// This structure contains information about a single animation clip, such
/// as playback speed and the trigger names associated with the clip.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone, PartialEq)]
pub struct ClipAnimDataBlock<'a> {
    /// The name of the animation clip.
    pub name: Str<'a>,

    /// An identifier for the animation clip.
    pub clip_id: Str<'a>,

    /// The playback speed of the animation.
    /// - type: [`f32`]
    pub play_back_speed: Str<'a>,

    /// The start time for cropping the animation.
    /// - type: [`f32`]
    pub crop_start_local_time: Str<'a>,

    /// The end time for cropping the animation.
    /// - type: [`f32`]
    pub crop_end_local_time: Str<'a>,

    /// The length of the trigger names.
    pub trigger_names_len: usize,

    /// A list of names that trigger the animation.
    pub trigger_names: Vec<Str<'a>>,
}

impl ClipAnimDataBlock<'_> {
    /// Returns the number of lines when serialized.
    ///
    /// ```txt
    /// 1(name) + 1(clip_id) + 1(play_back_speed) + 1(crop_start_local_time) + 1(crop_end_local_time) +
    /// 1(trigger_names_len) + n(trigger_names)   + 1(empty_line)
    /// = 7 + n
    /// ```
    const fn to_line_len(&self) -> usize {
        7 + self.trigger_names.len()
    }
}

/// Represents a motion block for a clip.
///
/// This structure contains information about the duration and translation
/// and rotation data for a specific motion clip.
///
/// # Example
/// ```txt
/// 999
/// 3.66667
/// 1
/// 3.66667 0 0 0
/// 1
/// 3.66667 0 0 0 1
/// ```
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone, PartialEq)]
pub struct ClipMotionBlock<'a> {
    /// An identifier for the clip associated with this motion block.
    pub clip_id: Str<'a>,

    /// The duration of the motion in seconds.
    /// - type: [`f32`]
    pub duration: Str<'a>,

    /// The length of the translation data.
    ///
    /// # Note
    /// used only for deserialization
    pub translation_len: usize,

    /// A list of translation data points.
    pub translations: Vec<Translation<'a>>,

    /// The length of the rotation data.
    ///
    /// # Note
    /// used only for deserialization
    pub rotation_len: usize,

    /// A list of rotation data points.
    pub rotations: Vec<Rotation<'a>>,
}

impl ClipMotionBlock<'_> {
    /// Returns the number of lines when serialized.
    ///
    /// ```txt
    /// 1(clip_id)       +   1(duration)  + 1(translation_len) + n_1(translations) +
    /// 1(rotations_len) + n_2(rotations) + 1(empty_line)
    /// = 5 + n_1 + n_2
    /// ```
    const fn to_line_len(&self) -> usize {
        3 + self.translations.len() + 1 + self.rotations.len() + 1 // +1 for the empty line
    }
}

/// Represents the rotation data using a quaternion,
/// where time indicates the moment of the rotation,
/// and x, y, z, w represent the quaternion components.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Rotation<'a> {
    /// The timestamp in seconds at which this rotation occurs.
    /// - type: [`f32`]
    pub time: Str<'a>,

    /// The x component of the quaternion, representing the rotation axis.
    /// - type: [`f32`]
    pub x: Str<'a>,

    /// The y component of the quaternion, representing the rotation axis.
    /// - type: [`f32`]
    pub y: Str<'a>,

    /// The z component of the quaternion, representing the rotation axis.
    /// - type: [`f32`]
    pub z: Str<'a>,

    /// The w component of the quaternion, representing the cosine of half the rotation angle.
    /// A value of `1.0` means no rotation (identity quaternion).
    /// - type: [`f32`]
    pub w: Str<'a>,
}

/// Represents the translation data (movement in space),
/// where time indicates the moment of translation,
/// and x, y, z represent the movement along the respective axes.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Translation<'a> {
    /// The timestamp in seconds at which this translation occurs.
    /// - type: [`f32`]
    pub time: Str<'a>,

    /// The amount of movement along the x-axis.
    /// - type: [`f32`]
    pub x: Str<'a>,

    /// The amount of movement along the y-axis.
    /// - type: [`f32`]
    pub y: Str<'a>,

    /// The amount of movement along the z-axis.
    /// - type: [`f32`]
    pub z: Str<'a>,
}
