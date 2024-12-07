use super::{
    aliases::{BorrowedTemplateMap, ModPatchMap, ModPatchPair, OwnedPatchMap},
    config::Config,
    tables::{FIRST_PERSON_BEHAVIORS, THIRD_PERSON_BEHAVIORS},
};
use crate::error::Result;
use crate::{
    collect_path::collect_nemesis_paths,
    error::{Error, FailedIoSnafu, JsonSnafu},
    output_path::{parse_input_nemesis_path, NemesisPath},
};
use dashmap::DashMap;
use rayon::{iter::Either, prelude::*};
use serde_hkx_features::ClassMap;
use simd_json::{serde::to_borrowed_value, BorrowedValue};
use snafu::ResultExt as _;
use std::fs;
use std::path::{Path, PathBuf};

pub fn collect_templates_and_patches<'a>(
    nemesis_paths: Vec<PathBuf>,
    options: &Config,
) -> Result<(BorrowedTemplateMap<'a>, ModPatchMap), Vec<Error>> {
    let templates = DashMap::new();

    let results: Vec<Result<ModPatchPair, Vec<Error>>> = nemesis_paths
        .par_iter()
        .map(|patch_path| {
            let result: Vec<_> = collect_nemesis_paths(patch_path)
                .par_iter()
                // # NOTE: Internally mut templates
                .map(|txt_path| parse_and_process_path(txt_path, options, &templates))
                .collect();

            let (patch_map_owned, errors): (Vec<_>, Vec<_>) =
                result.into_par_iter().partition_map(|res| match res {
                    Ok(val) => Either::Left(val),
                    Err(err) => Either::Right(err),
                });
            let patch_map_owned: OwnedPatchMap = patch_map_owned.into_par_iter().collect();

            patch_path.file_name().map_or_else(
                || Err(errors),
                |mod_code| Ok((mod_code.to_string_lossy().to_string(), patch_map_owned)),
            )
        })
        .collect();

    let (successes, errors): (Vec<ModPatchPair>, Vec<Vec<Error>>) =
        results.into_par_iter().partition_map(|res| match res {
            Ok((key, patch)) => Either::Left((key, patch)),
            Err(errs) => Either::Right(errs),
        });

    let errors: Vec<Error> = errors.into_par_iter().flatten().collect();
    if errors.is_empty() {
        let patch_mod_map = successes.into_par_iter().collect();
        Ok((templates, patch_mod_map))
    } else {
        Err(errors)
    }
}

fn parse_and_process_path(
    txt_path: &Path,
    options: &Config,
    templates: &BorrowedTemplateMap,
) -> Result<(String, String), Error> {
    let NemesisPath {
        relevant_path: _relevant_path,
        template_name,
        is_1stperson,
    } = parse_input_nemesis_path(txt_path).ok_or(Error::FailedParseNemesisPath {
        path: txt_path.to_path_buf(),
    })?;

    let template_name_key = match is_1stperson {
        true => format!("_1stperson/{template_name}"),
        false => template_name.clone(),
    };

    // Process template
    if !templates.contains_key(&template_name_key) {
        let (output_path, template_path) = match is_1stperson {
            true => FIRST_PERSON_BEHAVIORS.get(&template_name),
            false => THIRD_PERSON_BEHAVIORS.get(&template_name),
        }
        .map(|path| {
            let Config {
                resource_dir,
                output_dir,
                ..
            } = options;
            let mut output_path = output_dir.join(path);
            output_path.set_extension("hkx");
            (output_path, resource_dir.join(path))
        })
        .ok_or_else(|| Error::FailedParseNemesisPath {
            path: txt_path.to_path_buf(),
        })?;

        let json_value = template_xml_to_value(template_path)?;
        templates.insert(template_name_key.clone(), (output_path, json_value));
    }

    // Process patch
    let patch_txt = fs::read_to_string(txt_path).context(FailedIoSnafu {
        path: txt_path.to_path_buf(),
    })?;

    #[cfg(feature = "debug")] // Output patch.json for debugging
    {
        let mut json_patch_path = options.output_dir.join(_relevant_path);
        json_patch_path.set_extension("json");
        let patches_json = nemesis_xml::patch::parse_nemesis_patch(&patch_txt).context(
            crate::error::NemesisXmlErrSnafu {
                path: json_patch_path.clone(),
            },
        )?;
        fs::write(
            &json_patch_path,
            simd_json::to_string_pretty(&patches_json).context(JsonSnafu {
                path: json_patch_path.clone(),
            })?,
        )
        .context(FailedIoSnafu {
            path: json_patch_path.clone(),
        })?;
    }

    Ok((template_name_key, patch_txt))
}

fn template_xml_to_value(path: PathBuf) -> Result<BorrowedValue<'static>> {
    let template_xml = fs::read_to_string(&path).context(FailedIoSnafu { path: path.clone() })?;
    let ast: ClassMap = serde_hkx::from_str(&template_xml)?;
    to_borrowed_value(ast).context(JsonSnafu { path }) // TODO: fix needless realloc
}
