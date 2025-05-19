pub mod de;
pub mod ser;

use crate::lines::Str;

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

    /// A list of motion blocks corresponding to the clips.
    pub clip_motion_blocks: Vec<ClipMotionBlock<'a>>,
}

/// Represents the header of animation data.
///
/// This structure contains metadata related to the animation data, such as
/// the number of lines remaining, asset count, and project assets.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone, PartialEq)]
pub struct AnimDataHeader<'a> {
    /// Number of lines remaining representing `anim_data` after this line is read.
    pub line_range: usize,

    /// An integer value related to the animation (meaning may vary based on context).
    pub lead_int: i32,

    /// The length of the project assets.
    pub project_assets_len: usize,

    /// A list of project asset names.
    pub project_assets: Vec<Str<'a>>,

    /// Indicates whether motion data is available.
    pub has_motion_data: bool,
}

impl AnimDataHeader<'_> {
    /// Returns the number of lines consumed to read this struct.
    const fn parsed_line_len(&self) -> usize {
        3 + self.project_assets_len + 1
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
    /// Returns the number of lines consumed to read this struct.
    const fn parsed_line_len(&self) -> usize {
        6 + self.trigger_names_len + 1 // +1 for the empty line
    }
}

/// Represents a motion block for a clip.
///
/// This structure contains information about the duration and translation
/// and rotation data for a specific motion clip.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone, PartialEq)]
pub struct ClipMotionBlock<'a> {
    /// An identifier for the clip associated with this motion block.
    pub clip_id: Str<'a>,

    /// The duration of the motion in seconds.
    /// - type: [`f32`]
    pub duration: Str<'a>,

    /// The length of the translation data.
    pub translation_len: usize,

    /// A list of translation data points.
    pub translations: Vec<Translation<'a>>,

    /// The length of the rotation data.
    pub rotation_len: usize,

    /// A list of rotation data points.
    pub rotations: Vec<Rotation<'a>>,
}

impl ClipMotionBlock<'_> {
    /// Returns the number of lines consumed to read this struct.
    const fn parsed_line_len(&self) -> usize {
        3 + self.translation_len + 1 + self.rotation_len + 1 // +1 for the empty line
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
