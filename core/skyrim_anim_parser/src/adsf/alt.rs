use crate::adsf::Adsf;
use crate::{
    adsf::{AnimData, AnimDataHeader, ClipAnimDataBlock, ClipMotionBlock},
    lines::Str,
};
use indexmap::IndexMap;
use rayon::prelude::*;

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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone, PartialEq)]
pub struct AltAdsf<'a>(pub indexmap::IndexMap<Str<'a>, AltAnimData<'a>>);

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
            map.insert(key, anim.into());
        }

        Self(map)
    }
}

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
            anim_list.push(anim.into());
        }

        Adsf {
            project_names,
            anim_list,
        }
    }
}

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
pub fn to_adsf_key<'a>(key: std::borrow::Cow<'a, str>) -> std::borrow::Cow<'a, str> {
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
pub struct AltAnimData<'a> {
    /// The header containing metadata about the animation data.
    pub header: AnimDataHeader<'a>,

    /// A list of animation blocks corresponding to the clips.
    /// - Key: `<name>~<clip_id>` (e.g. `Jump~42`)
    pub clip_anim_blocks: IndexMap<Str<'a>, ClipAnimDataBlock<'a>>,

    /// It must be added at the beginning, but `Vec::insert` is slow.
    /// Therefore, another additional field is created and it is added first.
    ///
    /// # Note
    /// This is used during the patch merge phase.
    pub add_clip_anim_blocks: Vec<ClipAnimDataBlock<'a>>,

    /// A list of motion blocks corresponding to the clips.
    /// - key: `<clip_id>`
    pub clip_motion_blocks: IndexMap<Str<'a>, ClipMotionBlock<'a>>,

    /// It must be added at the beginning, but `Vec::insert` is slow.
    /// Therefore, another additional field is created and it is added first.
    ///
    /// # Note
    /// This is used during the patch merge phase.
    pub add_clip_motion_blocks: Vec<ClipMotionBlock<'a>>,
}

impl AltAnimData<'_> {
    /// Returns the number of lines when serialized.
    ///
    /// ```txt
    /// 1(header) + n(clip_anim_blocks) + n(clip_motion_blocks)
    /// = 1 + n_1 + n_2
    /// ```
    pub(super) fn to_line_range(&self) -> usize {
        (self.header.to_line_len() - 1) + self.clip_anim_blocks_line_len()
    }

    pub(crate) fn clip_anim_blocks_line_len(&self) -> usize {
        // NOTE: `.zip()` is not used here because it must be the same length.
        let len: usize = self
            .clip_anim_blocks
            .par_iter()
            .map(|(_, block)| block.to_line_len())
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
            .map(|(_, block)| block.to_line_len())
            .sum();
        let add_len: usize = self
            .add_clip_motion_blocks
            .par_iter()
            .map(|block| block.to_line_len())
            .sum();
        len + add_len
    }
}

impl<'a> From<AltAnimData<'a>> for AnimData<'a> {
    fn from(alt: AltAnimData<'a>) -> Self {
        AnimData {
            header: alt.header,
            clip_anim_blocks: alt
                .clip_anim_blocks
                .into_par_iter()
                .map(|(_, v)| v)
                .collect(),
            add_clip_anim_blocks: alt.add_clip_anim_blocks,
            clip_motion_blocks: alt
                .clip_motion_blocks
                .into_par_iter()
                .map(|(_, v)| v)
                .collect(),
            add_clip_motion_blocks: alt.add_clip_motion_blocks,
        }
    }
}

impl<'a> From<AnimData<'a>> for AltAnimData<'a> {
    fn from(anim: AnimData<'a>) -> Self {
        let clip_anim_blocks: IndexMap<_, _> = anim
            .clip_anim_blocks
            .into_par_iter()
            .map(|block| (format!("{}~{}", block.name, block.clip_id).into(), block))
            .collect();

        let clip_motion_blocks: IndexMap<_, _> = anim
            .clip_motion_blocks
            .into_par_iter()
            .map(|block| (block.clip_id.clone(), block))
            .collect();

        AltAnimData {
            header: anim.header,
            clip_anim_blocks,
            add_clip_anim_blocks: anim.add_clip_anim_blocks,
            clip_motion_blocks,
            add_clip_motion_blocks: anim.add_clip_motion_blocks,
        }
    }
}
