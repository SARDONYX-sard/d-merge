pub mod path_parser;
mod sort;
pub mod types;

use self::path_parser::{parse_asdsf_path, ParsedAsdsfPatchPath, ParserType};
use self::sort::dedup_patches_by_priority_parallel;
use self::types::OwnedAsdsfPatchMap;
use crate::behaviors::priority_ids::types::PriorityMap;
use crate::behaviors::tasks::hkx::generate::write_patched_json;
use crate::errors::{
    AnimPatchErrKind, AnimPatchErrSubKind, Error, FailedDiffLinesPatchSnafu, FailedIoSnafu,
    FailedParseAdsfTemplateSnafu, FailedParseAsdsfPatchSnafu, FailedParseEditAsdsfPatchSnafu,
    FailedSerializeAsdsfSnafu,
};
use crate::results::partition_results;
use crate::Config;

use rayon::prelude::*;
use skyrim_anim_parser::asdsf::alt::{
    ser::{serialize_alt_asdsf, SubHeaderDiffMap},
    AltAsdsf,
};
use skyrim_anim_parser::asdsf::patch::de::{
    deserializer::parse_anim_set_diff_patch, DiffPatchAnimSetData,
};
use skyrim_anim_parser::diff_line::{deserializer::parse_lines_diff_patch, DiffLines};
use snafu::ResultExt as _;
use std::path::{Path, PathBuf};
use winnow::Parser;

#[derive(serde::Serialize, Debug, Default, Clone, PartialEq)]
pub struct AsdsfPatch<'a> {
    /// e.g. `DefaultMaleData~DefaultMale`
    pub target: &'a str,
    /// e.g. `/some/Nemesis_Engine/mod/slide`
    pub id: &'a str,
    patch: PatchKind<'a>,
}

#[derive(serde::Serialize, Debug, Clone, PartialEq)]
enum PatchKind<'a> {
    /// Indicates the special `$header$/$header$.txt`override
    TxtProjectHeader(DiffLines<'a>),

    /// Indicates the special `<target>/$header$.txt`override
    SubTxtHeader(DiffLines<'a>),

    /// diff patch, priority
    EditAnimSet(Box<EditAnimSet<'a>>),

    /// add patch
    AddAnimSet {
        patch: skyrim_anim_parser::asdsf::normal::AnimSetData<'a>,
        priority: usize,
        file_name: &'a str,
    },
}

#[derive(serde::Serialize, Debug, Default, Clone, PartialEq)]
struct EditAnimSet<'a> {
    patch: DiffPatchAnimSetData<'a>,
    /// apply ordering
    priority: usize,
    /// file name of each txt project data
    /// - e.g. `_MTSolo.txt`
    file_name: &'a str,
}

impl<'a> Default for PatchKind<'a> {
    #[inline]
    fn default() -> Self {
        Self::EditAnimSet(Box::new(EditAnimSet::default()))
    }
}

const ASDSF_INNER_PATH: &str = "meshes/animationsetdatasinglefile.bin";

/// Patch to `animationsetdatasinglefile.txt`
pub(crate) fn apply_asdsf_patches(
    owned_anim_data_patches: OwnedAsdsfPatchMap,
    id_order: &PriorityMap,
    config: &Config,
) -> Vec<Error> {
    // 1/5 Parse adsf patch (1 loop with par_iter)
    let results: Vec<Result<AsdsfPatch, Error>> = owned_anim_data_patches
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

    // 3/5 read template asdsf.
    let alt_asdsf_bytes = bail!(read_asdsf_file(config));
    let mut alt_adsf: AltAsdsf =
        bail!(rmp_serde::from_slice(&alt_asdsf_bytes).with_context(|_| {
            FailedParseAdsfTemplateSnafu {
                path: config.resource_dir.join(ASDSF_INNER_PATH),
            }
        }));

    let mut txt_project_header_patches = DiffLines::DEFAULT;

    let mut sub_txt_header_patch_map = SubHeaderDiffMap::new();

    // 4/5. Apply adsf patch to base adsf(anim_data & motion data).
    for mut asdsf_patch in borrowed_patches {
        match asdsf_patch.patch {
            PatchKind::TxtProjectHeader(ref mut diff) => {
                txt_project_header_patches
                    .0
                    .par_extend(core::mem::take(&mut diff.0));
                continue;
            }
            PatchKind::SubTxtHeader(ref mut diff) => {
                use std::collections::hash_map::Entry;

                let lines = core::mem::take(&mut diff.0);
                match sub_txt_header_patch_map.entry(asdsf_patch.target) {
                    Entry::Occupied(mut e) => {
                        e.get_mut().0.par_extend(lines);
                    }
                    Entry::Vacant(e) => {
                        e.insert(DiffLines(lines));
                    }
                }
                continue;
            }
            _ => (),
        }

        if let Some(anim_data) = alt_adsf.txt_projects.0.get_mut(asdsf_patch.target) {
            match asdsf_patch.patch {
                PatchKind::AddAnimSet {
                    patch, file_name, ..
                } => {
                    anim_data
                        .0
                        .insert(std::borrow::Cow::Borrowed(file_name), patch);
                }
                PatchKind::EditAnimSet(edit_anim) => {
                    let file_name = edit_anim.file_name;
                    if let Some(anim) = anim_data.0.get_mut(file_name) {
                        bail!(edit_anim.patch.into_apply(anim).with_context(|_| {
                            FailedParseEditAsdsfPatchSnafu {
                                path: edit_anim.file_name,
                            }
                        }));
                    }
                }
                PatchKind::TxtProjectHeader(_) | PatchKind::SubTxtHeader(_) => {}
            };
        }
    }

    if config.debug.output_merged_json {
        if let Err(_err) = output_merged_alt_adsf(&alt_adsf, config) {
            tracing::error!("{_err}");
        }
    }

    // 5/5 Write adsf.
    let mut output_path = config.output_dir.join(ASDSF_INNER_PATH);
    output_path.set_extension("txt");
    bail!(write_alt_asdsf_file(
        output_path,
        alt_adsf,
        txt_project_header_patches,
        sub_txt_header_patch_map,
    ));

    errors
}

fn parse_anim_data_patch<'a>(
    (path, (asdsf_patch, priority)): (&'a PathBuf, &'a (String, usize)),
) -> Result<AsdsfPatch<'a>, Error> {
    let priority = *priority;

    let ParsedAsdsfPatchPath {
        target, // e.g. DefaultFemale
        id,     // e.g. slide
        parser_type,
    } = parse_asdsf_path(path)?;

    let patch = match parser_type {
        ParserType::TxtProjectHeader => PatchKind::TxtProjectHeader(
            parse_lines_diff_patch(asdsf_patch, priority).with_context(|_| {
                FailedDiffLinesPatchSnafu {
                    kind: AnimPatchErrKind::Asdsf,
                    sub_kind: AnimPatchErrSubKind::TxtProjectHeader,
                    path,
                }
            })?,
        ),
        ParserType::SubTxtHeader => {
            PatchKind::SubTxtHeader(parse_lines_diff_patch(asdsf_patch, priority).with_context(
                |_| FailedDiffLinesPatchSnafu {
                    kind: AnimPatchErrKind::Asdsf,
                    sub_kind: AnimPatchErrSubKind::SubTxtHeader,
                    path,
                },
            )?)
        }
        ParserType::AddAnimSet(file_name) => {
            let patch = skyrim_anim_parser::asdsf::normal::de::anim_set_data
                .parse(asdsf_patch)
                .map_err(|err| serde_hkx::errors::readable::ReadableError::from_parse(err))
                .with_context(|_| FailedParseAsdsfPatchSnafu { path: path.clone() })?;
            PatchKind::AddAnimSet {
                patch,
                priority,
                file_name,
            }
        }
        ParserType::EditAnimSet(file_name) => {
            let patch = parse_anim_set_diff_patch(asdsf_patch, priority)
                .with_context(|_| FailedParseEditAsdsfPatchSnafu { path: path.clone() })?;
            PatchKind::EditAnimSet(Box::new(EditAnimSet {
                patch,
                priority,
                file_name,
            }))
        }
    };
    Ok(AsdsfPatch { target, id, patch })
}

/// Sorts AdsfPatch list based on the given ID priority list.
fn sort_patches_by_priority(patches: &mut [AsdsfPatch], id_order: &PriorityMap) {
    patches.par_sort_by_key(|patch| id_order.get(patch.id).copied().unwrap_or(usize::MAX));
}

/// Read `animationsetdatasinglefile.txt` from the resource directory
fn read_asdsf_file(config: &Config) -> Result<Vec<u8>, Error> {
    let adsf_read_path = config.resource_dir.join(ASDSF_INNER_PATH);
    let adsf_string = std::fs::read(&adsf_read_path).with_context(|_| FailedIoSnafu {
        path: adsf_read_path,
    })?;
    Ok(adsf_string)
}

/// Write a `animationsetdatasinglefile.txt` file
fn write_alt_asdsf_file(
    path: impl AsRef<Path>,
    alt_asdsf: AltAsdsf,
    patches: DiffLines,
    sub_txt_header_patch_map: SubHeaderDiffMap,
) -> Result<(), Error> {
    let path = path.as_ref();

    let serialized = serialize_alt_asdsf(alt_asdsf, patches, sub_txt_header_patch_map)
        .with_context(|_| FailedSerializeAsdsfSnafu {
            kind: AnimPatchErrKind::Asdsf,
            sub_kind: AnimPatchErrSubKind::TxtProjectHeader,
            path,
        })?;

    if let Some(parent_dir) = path.parent() {
        let _ = std::fs::create_dir_all(parent_dir);
    }
    std::fs::write(path, serialized).with_context(|_| FailedIoSnafu {
        path: path.to_path_buf(),
    })?;
    Ok(())
}

/// Outputs debug JSON files for each patch in the provided slice.
fn output_debug_patch_json(patches: &[AsdsfPatch], config: &Config) {
    let mut dest_path = config
        .output_dir
        .join(".d_merge")
        .join(".debug")
        .join("patches")
        .join(ASDSF_INNER_PATH);
    dest_path.set_extension("patch.json");
    if let Err(_err) = write_patched_json(&dest_path, patches) {
        #[cfg(feature = "tracing")]
        tracing::error!("{_err}");
    };
}

/// Debug merged json.
fn output_merged_alt_adsf(alt_adsf: &AltAsdsf, config: &Config) -> Result<(), Error> {
    let mut dest_path = config
        .output_dir
        .join(".d_merge")
        .join(".debug")
        .join(ASDSF_INNER_PATH);
    dest_path.set_extension("json");
    write_patched_json(&dest_path, alt_adsf)
}
