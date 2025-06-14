pub mod clip_id_manager;
pub mod de;
pub mod patch;
pub mod ser;

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

/// Represents the alternative animation data structure.
///
/// Unlike [`Adsf`], which stores project names as strings with `.txt` extensions,
/// this structure uses a map keyed by project names without the `.txt` suffix.
/// Duplicate project names are disambiguated by appending `[n]` where `n` starts from 1.
///
/// This table shows how keys map from `Adsf` to `AltAdsf`:
///
/// | Adsf key           | AltAdsf key       | Description                          |
/// |--------------------|-------------------|--------------------------------------|
/// | `DefaultMale.txt`  | `DefaultMale~1`   | First occurrence (0th), no extension |
/// | `DefaultMale.txt`  | `DefaultMale~2`   | Second occurrence (1st duplicate)    |
/// | `DefaultMale.txt`  | `DefaultMale~3`   | Third occurrence (2nd duplicate)     |
/// | `Walk.txt`         | `Walk~1`          | Unique, no extension                 |
/// | `Walk.txt`         | `Walk~2`          | Duplicate occurrence                 |
///
/// This approach removes `.txt` from keys for efficiency and appends
/// numeric indices in brackets to avoid key collisions.
///
/// # Reasoning
///
/// This key design comes from how patch paths are parsed and referenced in memory.
/// According to Nemesis patch specs, the patch path format is:
///
/// ```text
/// <any>/<id>/animationdatasinglefile/<project_name>~<n th of project_name>/<array index>.txt
/// ```
///
/// When multiple entries share the same `project_name`, the `~n` suffix,
/// where `n` is 1-based and indicates the nth occurrence (with `1` meaning the first).
///
/// This allows most keys to be accessed as slices (partial string references)
/// pointing directly into the patch path without needing to allocate new strings via `to_string()`.
/// Consequently, `get` operations can be done efficiently with minimal allocations.
///
/// # Note
/// This allows fast O(1) access without extra heap allocations during patching.
#[cfg(feature = "alt_map")]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone, PartialEq)]
pub struct AltAdsf<'a>(pub indexmap::IndexMap<Str<'a>, AnimData<'a>>);

#[cfg(feature = "alt_map")]
impl<'a> From<Adsf<'a>> for AltAdsf<'a> {
    /// Converts [`Adsf`] into [`AltAdsf`] by transforming the parallel `Vec` fields
    /// into a map.
    ///
    /// Duplicate project names are disambiguated by appending `[n]`
    /// to their names, where `n` is the number of times the name has occurred so far.
    ///
    /// For example:
    /// ```text
    /// ["walk.txt", "run.txt", "walk.txt"] -> {"walk~1": ..., "run~1": ..., "walk~2": ...}
    /// ```
    /// This avoids key collisions in the map while preserving the original order.
    ///
    /// # NOTE
    /// The dir spec for the Nemesis adsf patch is 1based_index, but here it is 0based_index.
    #[inline]
    fn from(adsf: Adsf<'a>) -> Self {
        let Adsf {
            project_names,
            anim_list,
        } = adsf;

        debug_assert_eq!(
            project_names.len(),
            anim_list.len(),
            "Need to be the same length. but got project_names.len() != anim_list.len()"
        );

        use std::collections::HashMap;

        let mut map = indexmap::IndexMap::with_capacity(project_names.len());
        let mut counter: HashMap<String, usize> = HashMap::new();

        for (name, anim) in project_names.into_iter().zip(anim_list) {
            let name_str = name.as_ref();
            let base = name_str.strip_suffix(".txt").unwrap_or(name_str);

            let count = counter.entry(base.to_string()).or_insert(1);

            let key = if *count == 1 {
                Str::Owned(format!("{base}~1"))
            } else {
                Str::Owned(format!("{base}~{count}"))
            };

            *count += 1;
            map.insert(key, anim);
        }

        Self(map)
    }
}

#[cfg(feature = "alt_map")]
impl<'a> From<AltAdsf<'a>> for Adsf<'a> {
    /// Converts [`AltAdsf`] back into [`Adsf`] by recovering the original project names.
    ///
    /// Any suffix in the form `[n]` (where `n` is digits) after `.txt` is removed
    /// to restore the original name as accurately as possible.
    ///
    /// For example:
    /// ```text
    /// "walk" -> "walk.txt"
    /// "walk[1]" -> "walk.txt"
    /// ```
    ///
    /// If the name does not follow the `[n]` pattern, it is left unchanged.
    ///
    /// # NOTE
    /// The dir spec for the Nemesis adsf patch is 1based_index, but here it is 0based_index.
    #[inline]
    fn from(alt_adsf: AltAdsf<'a>) -> Self {
        let mut project_names = Vec::with_capacity(alt_adsf.0.len());
        let mut anim_list = Vec::with_capacity(alt_adsf.0.len());

        for (key, anim) in alt_adsf.0 {
            project_names.push(to_adsf_key(key));
            anim_list.push(anim);
        }

        Adsf {
            project_names,
            anim_list,
        }
    }
}

#[cfg(feature = "alt_map")]
/// Removes a trailing numeric index from a filename if it matches the pattern `.txt[<digits>]`.
///
/// This function is used when converting from [`AltAdsf`] back to [`Adsf`] to recover
/// the original project name before duplicate-disambiguation.
///
/// For example:
/// - `"walk~1"` becomes `"walk.txt"`
/// - `"jump.txt"` remains unchanged
///
/// If the input does not match this pattern, it is returned as-is.
///
/// # Returns
///
/// A `Cow<str>` containing the cleaned project name with the numeric index removed,
/// or the original `key` as a borrowed string if no match is found.
fn to_adsf_key<'a>(key: std::borrow::Cow<'a, str>) -> std::borrow::Cow<'a, str> {
    (|| {
        let start_index = key.rfind('~')?;
        let base = &key[..start_index];
        Some(std::borrow::Cow::Owned(format!("{base}.txt")))
    })()
    .unwrap_or(key)
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
