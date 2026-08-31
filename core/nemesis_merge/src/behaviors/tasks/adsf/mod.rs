//! `animationdatasinglefile.txt` patch handling.
pub(crate) mod path_parser;
pub(crate) mod sort;
pub(crate) mod types;

use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

use rayon::{iter::Either, prelude::*};
use skyrim_anim_parser::adsf::normal::{ClipAnimDataBlock, ClipMotionBlock};
use snafu::ResultExt as _;

pub(crate) use skyrim_anim_parser::{
    adsf::{
        alt::{AltAdsf, ser::serialize_alt_adsf},
        patch::de::{
            add::{parse_clip_anim_block_patch, parse_clip_motion_block_patch},
            anim_header::{AnimHeaderDiffPatch, deserializer::parse_anim_header_diff_patch},
            others::{
                clip_anim::{ClipAnimDiffPatch, deserializer::parse_clip_anim_diff_patch},
                clip_motion::{ClipMotionDiffPatch, deserializer::parse_clip_motion_diff_patch},
            },
        },
    },
    diff_line::{DiffLines, deserializer::parse_lines_diff_patch},
};

use crate::{
    Config, PatchMaps,
    behaviors::tasks::hkx::generate::write_patched_json,
    errors::{
        AnimPatchErrKind, AnimPatchErrSubKind, Error, FailedDiffLinesPatchSnafu, FailedIoSnafu,
        FailedParseAdsfAnimDataHeaderPatchSnafu, FailedParseAdsfPatchSnafu,
        FailedParseEditAdsfClipAnimPatchSnafu, FailedParseEditAdsfClipMotionPatchSnafu,
        FailedSerializeAdsfSnafu, MissingAdsfAnimClipSnafu, MissingAdsfMotionSnafu,
    },
};

use self::{
    path_parser::{ParserType, parse_adsf_path},
    sort::dedup_patches_by_priority_parallel,
    types::OwnedAdsfPatchMap,
};

const ADSF_INNER_PATH: &str = "meshes/animationdatasinglefile.bin";

#[derive(serde::Serialize, Debug, Default, Clone, PartialEq)]
pub(crate) struct AdsfPatch<'a> {
    /// The animation data target.
    ///
    /// Example: `FirstPerson~1`.
    pub target: &'a str,

    /// The Nemesis mod ID.
    pub id: &'a str,

    pub(crate) patch: PatchKind<'a>,
}

#[derive(serde::Serialize, Debug, Clone, PartialEq)]
pub(crate) enum PatchKind<'a> {
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
pub(crate) struct EditAnim<'a> {
    pub patch: ClipAnimDiffPatch<'a>,
    pub priority: usize,

    /// `<name>~<clip_id>`.
    ///
    /// Example: `Jump~42`.
    pub name_clip: &'a str,
}

#[derive(serde::Serialize, Debug, Default, Clone, PartialEq)]
pub(crate) struct EditMotion<'a> {
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

/// Applies `animationdatasinglefile.txt` patches.
///
/// # Patch categories
///
/// - `Add*`: appended to the corresponding `add_*_blocks` collection.
/// - `Edit*`: modifies an existing block.
/// - `Specified*`: inserts a block using the explicitly specified ID/index.
///   If a block with the same ID/index already exists, it is replaced.
///
/// `Specified*` patches are applied here rather than in the serializer because
/// an explicitly specified ID must replace an existing block before
/// serialization.
///
/// # Errors
///
/// Returns all errors encountered while parsing, applying, or serializing
/// patches.
pub(crate) fn apply_adsf_patches(
    owned_anim_data_patches: OwnedAdsfPatchMap,
    entries: &PatchMaps,
    config: &Config,
    fnis_adsf_patches: Vec<AdsfPatch<'_>>,
) -> Vec<Error> {
    // 1/5 Parse ADSF patches.
    let (mut borrowed_patches, mut errors): (Vec<_>, Vec<Error>) = owned_anim_data_patches
        .0
        .par_iter()
        .partition_map(|entry| match parse_anim_data_patch(entry, config) {
            Ok(value) => Either::Left(value),
            Err(error) => Either::Right(error),
        });

    borrowed_patches.par_extend(fnis_adsf_patches);

    // 2/5 Sort and resolve patch conflicts by priority.
    sort_patches_by_priority(&mut borrowed_patches, entries);
    let borrowed_patches = dedup_patches_by_priority_parallel(borrowed_patches);

    if config.debug.output_patch_json && !borrowed_patches.is_empty() {
        output_debug_patch_json(&borrowed_patches, config);
    }

    macro_rules! bail {
        ($expr:expr) => {
            match $expr {
                Ok(value) => value,
                Err(error) => {
                    errors.push(error);
                    return errors;
                }
            }
        };
    }

    // 3/5 Read the template ADSF.
    let alt_adsf_bytes = bail!(read_adsf_file(config));

    let mut alt_adsf: AltAdsf = bail!(rmp_serde::from_slice(&alt_adsf_bytes).with_context(|_| {
        crate::errors::FailedParseAdsfTemplateSnafu {
            path: config.resource_dir.join(ADSF_INNER_PATH),
        }
    }));

    let mut project_names_header_patches = DiffLines::DEFAULT;

    // 4/5 Apply patches.
    for adsf_patch in borrowed_patches {
        if let PatchKind::ProjectNamesHeader(mut diff) = adsf_patch.patch {
            project_names_header_patches.0.par_extend(core::mem::take(&mut diff.0));
            continue;
        }

        use indexmap::map::Entry;

        let anim_data = match alt_adsf.0.entry(Cow::Borrowed(adsf_patch.target)) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                entry.insert(skyrim_anim_parser::adsf::alt::AltAnimData::default())
            }
        };

        match adsf_patch.patch {
            PatchKind::ProjectNamesHeader(_) => unreachable!(),

            PatchKind::AnimDataHeader(diff) => {
                diff.into_apply(&mut anim_data.header);
            }

            PatchKind::AddAnim(block) => {
                anim_data.add_clip_anim_blocks.push(block);
            }

            PatchKind::EditAnim(edit) => {
                let Some(block) = anim_data.clip_anim_blocks.get_mut(edit.name_clip) else {
                    errors.push(
                        MissingAdsfAnimClipSnafu {
                            target: adsf_patch.target.to_owned(),
                            name_clip: edit.name_clip.to_owned(),
                        }
                        .build(),
                    );
                    continue;
                };

                if let Err(error) = edit.patch.into_apply(block).with_context(|_| {
                    FailedParseEditAdsfClipAnimPatchSnafu { path: edit.name_clip }
                }) {
                    errors.push(error);
                }
            }

            PatchKind::SpecifiedAnim { name_clip, patch, .. } => {
                anim_data.clip_anim_blocks.insert(Cow::Borrowed(name_clip), patch);
            }

            PatchKind::AddMotion(block) => {
                anim_data.add_clip_motion_blocks.push(block);
            }

            PatchKind::EditMotion(edit) => {
                let Some(block) = anim_data.clip_motion_blocks.get_mut(edit.clip_id) else {
                    errors.push(
                        MissingAdsfMotionSnafu {
                            target: adsf_patch.target.to_owned(),
                            index: edit.clip_id.to_owned(),
                        }
                        .build(),
                    );
                    continue;
                };

                if let Err(error) = edit.patch.into_apply(block).with_context(|_| {
                    FailedParseEditAdsfClipMotionPatchSnafu { path: edit.clip_id }
                }) {
                    errors.push(error);
                }
            }

            PatchKind::SpecifiedMotion { index, patch, .. } => {
                anim_data.clip_motion_blocks.insert(Cow::Borrowed(index), patch);
            }
        }
    }

    if config.debug.output_merged_json
        && let Err(_error) = output_merged_alt_adsf(&alt_adsf, config)
    {
        #[cfg(feature = "tracing")]
        tracing::error!("{_error}");
    }

    // 5/5 Write ADSF.
    let mut output_path = config.output_dir.join(ADSF_INNER_PATH);
    output_path.set_extension("txt");

    bail!(write_alt_adsf_file(output_path, alt_adsf, project_names_header_patches,));

    errors
}

fn parse_anim_data_patch<'a>(
    (path, (adsf_patch, priority)): (&'a PathBuf, &'a (String, usize)),
    config: &Config,
) -> Result<AdsfPatch<'a>, Error> {
    let priority = *priority;

    let parsed = parse_adsf_path(path)?;

    let patch = match parsed.parser_type {
        ParserType::TxtProjectHeader => PatchKind::ProjectNamesHeader(
            parse_lines_diff_patch(adsf_patch, priority).with_context(|_| {
                FailedDiffLinesPatchSnafu {
                    kind: AnimPatchErrKind::Adsf,
                    sub_kind: AnimPatchErrSubKind::ProjectNamesHeader,
                    path,
                }
            })?,
        ),

        ParserType::AnimHeader => PatchKind::AnimDataHeader(
            parse_anim_header_diff_patch(adsf_patch)
                .with_context(|_| FailedParseAdsfAnimDataHeaderPatchSnafu { path: path.clone() })?,
        ),

        ParserType::AddAnim => {
            let patch = match config.parser_mode {
                crate::ParserMode::Strict => parse_clip_anim_block_patch::<true>(adsf_patch),
                crate::ParserMode::Lenient => parse_clip_anim_block_patch::<false>(adsf_patch),
            }
            .with_context(|_| FailedParseAdsfPatchSnafu { path: path.clone() })?;

            PatchKind::AddAnim(patch)
        }

        ParserType::IndexedAnim { name_clip } => {
            // NOTE:
            // Some Nemesis mods, such as TK Dodge, use an indexed path like
            // `TKDodgeRight~348.txt` for an animation addition.
            //
            // Such a file has no `MOD_CODE` block because it is not an edit
            // patch. The explicit clip ID in the path is therefore part of
            // the add operation and must be preserved.
            if adsf_patch.contains("<!-- MOD_CODE") {
                let patch =
                    parse_clip_anim_diff_patch(adsf_patch, priority).with_context(|_| {
                        FailedParseEditAdsfClipAnimPatchSnafu { path: path.clone() }
                    })?;

                PatchKind::EditAnim(EditAnim { patch, priority, name_clip })
            } else {
                let patch = match config.parser_mode {
                    crate::ParserMode::Strict => parse_clip_anim_block_patch::<true>(adsf_patch),
                    crate::ParserMode::Lenient => parse_clip_anim_block_patch::<false>(adsf_patch),
                }
                .with_context(|_| FailedParseAdsfPatchSnafu { path: path.clone() })?;

                PatchKind::SpecifiedAnim { name_clip, patch, priority }
            }
        }

        ParserType::AddMotion => {
            let patch = match config.parser_mode {
                crate::ParserMode::Strict => parse_clip_motion_block_patch::<true>(adsf_patch),
                crate::ParserMode::Lenient => parse_clip_motion_block_patch::<false>(adsf_patch),
            }
            .with_context(|_| FailedParseAdsfPatchSnafu { path: path.clone() })?;

            PatchKind::AddMotion(patch)
        }

        ParserType::IndexedMotion { index } => {
            if adsf_patch.contains("<!-- MOD_CODE") {
                let patch =
                    parse_clip_motion_diff_patch(adsf_patch, priority).with_context(|_| {
                        FailedParseEditAdsfClipMotionPatchSnafu { path: path.clone() }
                    })?;

                PatchKind::EditMotion(EditMotion { patch, priority, clip_id: index })
            } else {
                let patch = match config.parser_mode {
                    crate::ParserMode::Strict => parse_clip_motion_block_patch::<true>(adsf_patch),
                    crate::ParserMode::Lenient => {
                        parse_clip_motion_block_patch::<false>(adsf_patch)
                    }
                }
                .with_context(|_| FailedParseAdsfPatchSnafu { path: path.clone() })?;

                PatchKind::SpecifiedMotion { index, patch, priority }
            }
        }
    };

    Ok(AdsfPatch { target: parsed.target, id: parsed.id, patch })
}

/// Sorts ADSF patches by mod priority.
///
/// Higher-priority patches are processed after lower-priority patches.
fn sort_patches_by_priority(patches: &mut [AdsfPatch], id_orders: &PatchMaps) {
    patches.par_sort_by_key(|patch| {
        id_orders
            .nemesis_entries
            .get(patch.id)
            .copied()
            .or_else(|| id_orders.fnis_entries.get(patch.id).copied())
            .unwrap_or(usize::MAX)
    });
}

/// Reads the ADSF template.
///
/// # Errors
///
/// Returns an error when the template cannot be read.
fn read_adsf_file(config: &Config) -> Result<Vec<u8>, Error> {
    let path = config.resource_dir.join(ADSF_INNER_PATH);

    std::fs::read(&path).with_context(|_| FailedIoSnafu { path })
}

/// Writes the merged ADSF.
///
/// # Errors
///
/// Returns an error when serialization or file writing fails.
fn write_alt_adsf_file(
    path: impl AsRef<Path>,
    alt_adsf: AltAdsf,
    patches: DiffLines,
) -> Result<(), Error> {
    let path = path.as_ref();

    let serialized = serialize_alt_adsf(alt_adsf, (!patches.is_empty()).then_some(patches))
        .with_context(|_| FailedSerializeAdsfSnafu {
            kind: AnimPatchErrKind::Adsf,
            sub_kind: AnimPatchErrSubKind::ProjectNamesHeader,
            path,
        })?;

    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    std::fs::write(path, serialized)
        .with_context(|_| FailedIoSnafu { path: path.to_path_buf() })?;
    #[cfg(feature = "tracing")]
    tracing::info!("Generated: {}", path.display());
    Ok(())
}

fn output_debug_patch_json(patches: &[AdsfPatch], config: &Config) {
    let mut path =
        config.output_dir.join(".d_merge").join(".debug").join("patches").join(ADSF_INNER_PATH);

    path.set_extension("patch.json");

    if let Err(_error) = write_patched_json(&path, patches) {
        #[cfg(feature = "tracing")]
        tracing::error!("{_error}");
    }
}

fn output_merged_alt_adsf(alt_adsf: &AltAdsf, config: &Config) -> Result<(), Error> {
    let mut path = config.output_dir.join(".d_merge").join(".debug").join(ADSF_INNER_PATH);
    path.set_extension("json");

    write_patched_json(&path, alt_adsf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sort_patches_by_priority() {
        let ids = ["dmco", "flinch", "a", "slide"];

        let mut patches = vec![
            AdsfPatch { id: ids[1], ..Default::default() }, // flinch
            AdsfPatch { id: ids[2], ..Default::default() }, // a
            AdsfPatch { id: ids[0], ..Default::default() }, // dmco
            AdsfPatch { id: ids[3], ..Default::default() },
        ];

        sort_patches_by_priority(
            &mut patches,
            &PatchMaps {
                nemesis_entries: ids
                    .iter()
                    .enumerate()
                    .map(|(priority, &id)| (id.to_string(), priority))
                    .collect(),
                ..Default::default()
            },
        );

        let sorted_ids: Vec<&str> = patches.iter().map(|p| p.id).collect();
        assert_eq!(sorted_ids, ids);
    }
}
