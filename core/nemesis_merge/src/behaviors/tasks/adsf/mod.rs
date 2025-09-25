pub mod path_parser;
mod sort;
pub mod types;

use self::path_parser::{parse_adsf_path, ParsedAdsfPatchPath, ParserType};
use self::sort::dedup_patches_by_priority_parallel;
use self::types::OwnedAdsfPatchMap;
use crate::behaviors::priority_ids::types::PriorityMap;
use crate::behaviors::tasks::hkx::generate::write_patched_json;
use crate::errors::{
    AnimPatchErrKind, AnimPatchErrSubKind, Error, FailedDiffLinesPatchSnafu, FailedIoSnafu,
    FailedParseAdsfPatchSnafu, FailedParseAdsfTemplateSnafu, FailedParseEditAdsfPatchSnafu,
};
use crate::results::partition_results;
use crate::Config;
use rayon::prelude::*;
use skyrim_anim_parser::adsf::alt::{
    ser::{serialize_alt_adsf, serialize_alt_adsf_with_patches},
    AltAdsf,
};
use skyrim_anim_parser::adsf::normal::{ClipAnimDataBlock, ClipMotionBlock};
pub use skyrim_anim_parser::adsf::patch::de::add::{
    parse_clip_anim_block_patch, parse_clip_motion_block_patch,
};
pub use skyrim_anim_parser::adsf::patch::de::others::{
    clip_anim::{deserializer::parse_clip_anim_diff_patch, ClipAnimDiffPatch},
    clip_motion::{deserializer::parse_clip_motion_diff_patch, ClipMotionDiffPatch},
};
use skyrim_anim_parser::diff_line::{deserializer::parse_lines_diff_patch, DiffLines};
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
    /// Indicates the special `$header$/$header$.txt`override
    ProjectNamesHeader(DiffLines<'a>),
    #[allow(unused)]
    /// Indicates the special `<target>~<index>/$header$.txt`override
    AnimDataHeader(DiffLines<'a>),

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
    /// `<Name>~<clip_id>`
    /// - e.g. `Jump~42`
    ///
    /// NOTE: Unlike Motion, Anim sometimes references the same clip_id, so it cannot be used as an id.
    /// Therefore, Name is used instead
    name_clip: &'a str,
}

#[derive(serde::Serialize, Debug, Default, Clone, PartialEq)]
struct EditMotion<'a> {
    patch: ClipMotionDiffPatch<'a>,
    priority: usize,
    clip_id: &'a str,
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
        output_debug_patch_json(&borrowed_patches, config);
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

    let mut project_names_header_patches = DiffLines::DEFAULT;

    // 4/5. Apply adsf patch to base adsf(anim_data & motion data).
    for mut adsf_patch in borrowed_patches {
        if let PatchKind::ProjectNamesHeader(ref mut diff) = adsf_patch.patch {
            project_names_header_patches
                .0
                .par_extend(core::mem::take(&mut diff.0));
            continue;
        }

        if let Some(anim_data) = alt_adsf.0.get_mut(adsf_patch.target) {
            match adsf_patch.patch {
                PatchKind::ProjectNamesHeader(_) => {}
                PatchKind::AnimDataHeader(diff) => {
                    if let Err(err) = diff.into_apply(&mut anim_data.header.project_assets) {
                        tracing::error!("{err}");
                    };
                }
                PatchKind::AddAnim(clip_anim_data_block) => {
                    anim_data.add_clip_anim_blocks.push(clip_anim_data_block);
                }
                PatchKind::EditAnim(edit_anim) => {
                    if let Some(anim) = anim_data.clip_anim_blocks.get_mut(edit_anim.name_clip) {
                        edit_anim.patch.into_apply(anim);
                    }
                }
                PatchKind::AddMotion(clip_motion_block) => {
                    anim_data.add_clip_motion_blocks.push(clip_motion_block);
                }
                PatchKind::EditMotion(edit_motion) => {
                    if let Some(motion) = anim_data.clip_motion_blocks.get_mut(edit_motion.clip_id)
                    {
                        edit_motion.patch.into_apply(motion);
                    }
                }
            };
        }
    }

    if config.debug.output_merged_json {
        if let Err(_err) = output_merged_alt_adsf(&alt_adsf, config) {
            tracing::error!("{_err}");
        }
    }

    // 5/5 Write adsf.
    let mut output_path = config.output_dir.join(ADSF_INNER_PATH);
    output_path.set_extension("txt");
    bail!(write_alt_adsf_file(
        output_path,
        alt_adsf,
        project_names_header_patches
    ));

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
        ParserType::TxtProjectHeader => PatchKind::ProjectNamesHeader(
            parse_lines_diff_patch(adsf_patch, priority).with_context(|_| {
                FailedDiffLinesPatchSnafu {
                    kind: AnimPatchErrKind::Adsf,
                    sub_kind: AnimPatchErrSubKind::ProjectNamesHeader,
                    path,
                }
            })?,
        ),
        ParserType::AnimHeader => {
            return Err(Error::Custom {
                msg: "Unsupported anim header $header$ yet.".to_owned(),
            });
            // PatchKind::AnimDataHeader(parse_lines_diff_patch(adsf_patch, priority).with_context(
            //     |_| FailedDiffLinesPatchSnafu {
            //         kind: AnimPatchErrKind::Adsf,
            //         sub_kind: AnimPatchErrSubKind::AnimDataHeader,
            //         path,
            //     },
            // )?)
        }

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
                name_clip: index,
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
                clip_id: index,
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
fn write_alt_adsf_file(
    path: impl AsRef<Path>,
    alt_adsf: AltAdsf,
    patches: DiffLines,
) -> Result<(), Error> {
    let path = path.as_ref();

    let serialized = if patches.is_empty() {
        serialize_alt_adsf(&alt_adsf)
    } else {
        serialize_alt_adsf_with_patches(alt_adsf, patches).with_context(|_| {
            FailedDiffLinesPatchSnafu {
                kind: AnimPatchErrKind::Adsf,
                sub_kind: AnimPatchErrSubKind::ProjectNamesHeader,
                path,
            }
        })?
    };
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
/// | `edit`/`add`  | Indicates whether the patch modifies existing data or adds new content      | `edit`, `add`                 |
/// | `_idxNNN`     | Only included for `Edit` patches; shows the index of the edited entry       | `_idx044`                     |
/// | `patchXX`     | Shared patch group index formatted as a two-digit number                    | `patch07`                     |
///
/// ### Full Filename Examples
///
/// - `clip_anim_add_patch07.json`
/// - `clip_anim_edit_idx044_patch07.json`
/// - `clip_motion_edit_idx005_patch12.json`
///
/// This naming convention helps ensure:
///
/// - Easy grouping and lookup by `patchXX`
/// - Clear distinction between `add` and `edit` actions
/// - Fine-grained identification of edits via index
fn output_debug_patch_json(borrowed_patches: &[AdsfPatch], config: &Config) {
    for (patch_id, patch) in borrowed_patches.iter().enumerate() {
        let mut debug_path = config.output_dir.join(".d_merge").join(".debug");
        let (kind, index_str): (_, Cow<'_, str>) = match &patch.patch {
            PatchKind::ProjectNamesHeader(_) => ("txt_project_header", "".into()),
            PatchKind::AnimDataHeader(_) => ("anim_header", "".into()),
            PatchKind::AddAnim(_) => ("clip_anim", "".into()),
            PatchKind::AddMotion(_) => ("clip_motion", "".into()),
            PatchKind::EditAnim(edit) => ("clip_anim", format!("_id{}", edit.name_clip).into()),
            PatchKind::EditMotion(edit) => ("clip_motion", format!("_id{}", edit.clip_id).into()),
        };

        let action = match &patch.patch {
            PatchKind::ProjectNamesHeader(_) | PatchKind::AnimDataHeader(_) => "patch",
            PatchKind::AddAnim(_) | PatchKind::AddMotion(_) => "add",
            PatchKind::EditAnim(_) | PatchKind::EditMotion(_) => "edit",
        };

        let target = patch.target;
        let inner_path = format!(
            "patches/animationdatasinglefile/{target}/{kind}_{action}{index_str}_{patch_id:04}.json",
        );
        debug_path.push(inner_path);
        if let Err(_err) = write_patched_json(&debug_path, patch) {
            #[cfg(feature = "tracing")]
            tracing::error!("{_err}");
        };
    }
}

fn output_merged_alt_adsf(alt_adsf: &AltAdsf, config: &Config) -> Result<(), Error> {
    let adsf_path = config
        .output_dir
        .join(".d_merge")
        .join(".debug")
        .join(ADSF_INNER_PATH);
    write_patched_json(&adsf_path, alt_adsf)
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
            &ids.iter()
                .enumerate()
                .map(|(priority, &id)| (id.to_string(), priority))
                .collect(),
        );

        let sorted_ids: Vec<&str> = patches.iter().map(|p| p.id).collect();
        assert_eq!(sorted_ids, ids);
    }
}
