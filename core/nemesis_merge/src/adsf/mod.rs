pub mod path_parser;
mod sort;

use self::path_parser::{parse_adsf_path, ParsedAdsfPatchPath, ParserType};
use crate::adsf::sort::dedup_patches_by_priority_parallel;
use crate::errors::{
    Error, FailedIoSnafu, FailedParseAdsfPatchSnafu, FailedParseAdsfTemplateSnafu,
    FailedParseEditAdsfPatchSnafu,
};
use crate::hkx::generate::write_patched_json;
use crate::results::partition_results;
use crate::types::{OwnedAdsfPatchMap, PriorityMap};
use crate::Config;
use rayon::prelude::*;
use skyrim_anim_parser::adsf::patch::{
    parse_clip_anim_block_patch, parse_clip_anim_diff_patch, parse_clip_motion_block_patch,
    parse_clip_motion_diff_patch, ClipAnimDiffPatch, ClipMotionDiffPatch,
};
use skyrim_anim_parser::adsf::ser::serialize_alt_adsf;
use skyrim_anim_parser::adsf::{AltAdsf, ClipAnimDataBlock, ClipMotionBlock};
use snafu::ResultExt as _;
use std::borrow::Cow;
use std::path::{Path, PathBuf};

#[derive(serde::Serialize, Debug, Default, Clone, PartialEq)]
pub struct AdsfPatch<'a> {
    /// e.g. `DefaultMale`, `DefaultFemale`
    pub target: &'a str,
    /// e.g. `/some/Nemesis_Engine/mod/slide`
    pub id: &'a str,
    patch: PatchKind<'a>,
}

#[derive(serde::Serialize, Debug, Clone, PartialEq)]
enum PatchKind<'a> {
    AddAnim(ClipAnimDataBlock<'a>),
    /// diff patch, priority
    EditAnim(EditAnim<'a>),
    AddMotion(ClipMotionBlock<'a>),
    /// diff patch, priority
    EditMotion(EditMotion<'a>),
}

#[derive(serde::Serialize, Debug, Default, Clone, PartialEq)]
struct EditAnim<'a> {
    patch: ClipAnimDiffPatch<'a>,
    priority: usize,
    index: usize,
}

#[derive(serde::Serialize, Debug, Default, Clone, PartialEq)]
struct EditMotion<'a> {
    patch: ClipMotionDiffPatch<'a>,
    priority: usize,
    /// NOTE: 1 based index
    index: usize,
}

impl<'a> Default for PatchKind<'a> {
    #[inline]
    fn default() -> Self {
        Self::AddAnim(ClipAnimDataBlock::default())
    }
}

const ADSF_INNER_PATH: &str = "meshes/animationdatasinglefile.bin";

// "dmco", "slide"
/// Patch to `animationdatasinglefile.txt`
pub(crate) fn apply_adsf_patches(
    owned_anim_data_patches: OwnedAdsfPatchMap,
    id_order: &PriorityMap,
    config: &Config,
) -> Vec<Error> {
    // 1/5 Parse adsf patch (1 loop with par_iter)
    let results: Vec<Result<AdsfPatch, Error>> = owned_anim_data_patches
        .0
        .par_iter() // par iter
        .map(parse_anim_data_patch)
        .collect(); // back iter

    let (mut borrowed_patches, mut errors) = partition_results(results);

    // 2/5 Sort by priority ids.(to vec 2 loop) => borrowed_map
    sort_patches_by_priority(&mut borrowed_patches, id_order);
    let borrowed_patches = dedup_patches_by_priority_parallel(borrowed_patches);

    if config.debug.output_patch_json {
        output_debug_json(&borrowed_patches, config);
    }

    macro_rules! bail {
        ($expr:expr) => {
            match $expr {
                Ok(adsf) => adsf,
                Err(err) => {
                    errors.push(err);
                    return errors;
                }
            }
        };
    }

    // 3/5 read template adsf.
    let alt_adsf_bytes = bail!(read_adsf_file(config));
    let mut alt_adsf: AltAdsf = bail!(rmp_serde::from_slice(&alt_adsf_bytes).with_context(|_| {
        FailedParseAdsfTemplateSnafu {
            path: config.resource_dir.join(ADSF_INNER_PATH),
        }
    }));

    // 4/5. Apply adsf patch to base adsf(anim_data & motion data).
    for adsf_patch in borrowed_patches {
        if let Some(anim_data) = alt_adsf.0.get_mut(adsf_patch.target) {
            match adsf_patch.patch {
                PatchKind::AddAnim(clip_anim_data_block) => {
                    anim_data.add_clip_anim_blocks.push(clip_anim_data_block);
                }
                PatchKind::EditAnim(edit_anim) => {
                    if let Some(anim) = anim_data.clip_anim_blocks.get_mut(edit_anim.index - 1) {
                        edit_anim.patch.into_apply(anim);
                    }
                }
                PatchKind::AddMotion(clip_motion_block) => {
                    anim_data.add_clip_motion_blocks.push(clip_motion_block);
                }
                PatchKind::EditMotion(edit_motion) => {
                    if let Some(motion) =
                        anim_data.clip_motion_blocks.get_mut(edit_motion.index - 1)
                    {
                        edit_motion.patch.into_apply(motion);
                    }
                }
            };
        }
    }

    // 5/5 Write adsf.
    let mut output_path = config.output_dir.join(ADSF_INNER_PATH);
    output_path.set_extension("txt");
    bail!(write_alt_adsf_file(output_path, &alt_adsf));

    errors
}

fn parse_anim_data_patch<'a>(
    (path, (adsf_patch, priority)): (&'a PathBuf, &'a (String, usize)),
) -> Result<AdsfPatch<'a>, Error> {
    let priority = *priority;

    let ParsedAdsfPatchPath {
        target, // e.g. DefaultFemale
        id,     // e.g. slide
        parser_type,
    } = parse_adsf_path(path)?;

    let patch = match parser_type {
        ParserType::AddAnim => PatchKind::AddAnim(
            parse_clip_anim_block_patch(adsf_patch)
                .with_context(|_| FailedParseAdsfPatchSnafu { path: path.clone() })?,
        ),
        ParserType::EditAnim(index) => {
            let patch = parse_clip_anim_diff_patch(adsf_patch)
                .with_context(|_| FailedParseEditAdsfPatchSnafu { path: path.clone() })?;
            PatchKind::EditAnim(EditAnim {
                patch,
                priority,
                index,
            })
        }

        ParserType::AddMotion => PatchKind::AddMotion(
            parse_clip_motion_block_patch(adsf_patch)
                .with_context(|_| FailedParseAdsfPatchSnafu { path: path.clone() })?,
        ),
        ParserType::EditMotion(index) => {
            let patch = parse_clip_motion_diff_patch(adsf_patch)
                .with_context(|_| FailedParseEditAdsfPatchSnafu { path: path.clone() })?;
            PatchKind::EditMotion(EditMotion {
                patch,
                priority,
                index,
            })
        }

        ParserType::AnimHeader => {
            return Err(Error::Custom {
                msg: "Unsupported $header$ yet.".to_owned(),
            })
        }
    };
    Ok(AdsfPatch { target, id, patch })
}

/// Sorts AdsfPatch list based on the given ID priority list.
fn sort_patches_by_priority(patches: &mut [AdsfPatch], id_order: &PriorityMap) {
    patches.par_sort_by_key(|patch| id_order.get(patch.id).copied().unwrap_or(usize::MAX));
}

/// Read the ADSF file from the resource directory
fn read_adsf_file(config: &Config) -> Result<Vec<u8>, Error> {
    let adsf_read_path = config.resource_dir.join(ADSF_INNER_PATH);
    let adsf_string = std::fs::read(&adsf_read_path).with_context(|_| FailedIoSnafu {
        path: adsf_read_path,
    })?;
    Ok(adsf_string)
}

/// Write a single adsf file
fn write_alt_adsf_file(path: impl AsRef<Path>, alt_adsf: &AltAdsf) -> Result<(), Error> {
    let serialized = serialize_alt_adsf(alt_adsf);
    let path = path.as_ref();
    if let Some(parent_dir) = path.parent() {
        let _ = std::fs::create_dir_all(parent_dir);
    }
    std::fs::write(path, serialized).with_context(|_| FailedIoSnafu {
        path: path.to_path_buf(),
    })?;
    Ok(())
}

/// Outputs debug JSON files for each patch in the provided slice.
///
/// Each file is written to a subdirectory under `.d_merge/.debug/` and is
/// categorized by patch type and target. The filenames are structured to support
/// easy sorting and searching, using a shared patch identifier (`patchXX`) across
/// both `Add` and `Edit` variants.
///
/// ### Filename Format
///
/// | Field         | Description                                                                 | Example                       |
/// |---------------|-----------------------------------------------------------------------------|-------------------------------|
/// | `clip_anim`   | Category of the patch (`clip_anim` or `clip_motion`)                        | `clip_anim`                   |
/// | `patchXX`     | Shared patch group index formatted as a two-digit number                    | `patch07`                     |
/// | `edit`/`add`  | Indicates whether the patch modifies existing data or adds new content      | `edit`, `add`                 |
/// | `_idxNNN`     | Only included for `Edit` patches; shows the index of the edited entry       | `_idx044`                     |
///
/// ### Full Filename Examples
///
/// - `clip_anim_patch07_add.json`
/// - `clip_anim_patch07_edit_idx044.json`
/// - `clip_motion_patch12_edit_idx005.json`
///
/// This naming convention helps ensure:
///
/// - Easy grouping and lookup by `patchXX`
/// - Clear distinction between `add` and `edit` actions
/// - Fine-grained identification of edits via index
fn output_debug_json(borrowed_patches: &[AdsfPatch], config: &Config) {
    for (patch_id, patch) in borrowed_patches.iter().enumerate() {
        let mut debug_path = config.output_dir.join(".d_merge").join(".debug");
        let (kind, index_str): (_, Cow<'_, str>) = match &patch.patch {
            PatchKind::AddAnim(_) => ("clip_anim", "".into()),
            PatchKind::AddMotion(_) => ("clip_motion", "".into()),
            PatchKind::EditAnim(edit) => ("clip_anim", format!("_idx{:04}", edit.index).into()),
            PatchKind::EditMotion(edit) => ("clip_motion", format!("_idx{:04}", edit.index).into()),
        };

        let action = match &patch.patch {
            PatchKind::AddAnim(_) | PatchKind::AddMotion(_) => "add",
            PatchKind::EditAnim(_) | PatchKind::EditMotion(_) => "edit",
        };

        let target = patch.target;
        let inner_path = format!(
            "patches/animationdatasinglefile/{target}/{kind}{patch_id:03}_{action}{index_str}.json",
        );
        debug_path.push(inner_path);
        if let Err(_err) = write_patched_json(&debug_path, patch) {
            #[cfg(feature = "tracing")]
            tracing::error!("{_err}");
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sort_patches_by_priority() {
        let ids = ["dmco", "flinch", "a", "slide"];

        let mut patches = vec![
            AdsfPatch {
                id: ids[1],
                ..Default::default()
            }, // flinch
            AdsfPatch {
                id: ids[2],
                ..Default::default()
            }, // a
            AdsfPatch {
                id: ids[0],
                ..Default::default()
            }, // dmco
            AdsfPatch {
                id: ids[3],
                ..Default::default()
            },
        ];

        sort_patches_by_priority(
            &mut patches,
            &ids.iter().enumerate().map(|(i, &p)| (p, i)).collect(),
        );

        let sorted_ids: Vec<&str> = patches.iter().map(|p| p.id).collect();
        assert_eq!(sorted_ids, ids);
    }
}
