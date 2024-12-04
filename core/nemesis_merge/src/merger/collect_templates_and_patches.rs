use super::{
    tables::{FIRST_PERSON_BEHAVIORS, THIRD_PERSON_BEHAVIORS},
    PatchMapOwned, PatchModMap, TemplateMap,
};
use crate::error::Result;
use crate::{
    collect_path::collect_nemesis_paths,
    error::{Error, FailedIoSnafu, JsonSnafu},
    merger::Options,
    output_path::{parse_input_nemesis_path, NemesisPath},
};
use dashmap::DashMap;
use rayon::prelude::*;
use serde_hkx_features::ClassMap;
use simd_json::{serde::to_borrowed_value, BorrowedValue};
use snafu::ResultExt as _;
use std::fs;
use std::path::{Path, PathBuf};

pub fn collect_templates_and_patches<'a>(
    nemesis_paths: Vec<PathBuf>,
    options: Options,
) -> Result<(TemplateMap<'a>, PatchModMap), Vec<Error>> {
    let templates = DashMap::new();
    let patch_mod_map = DashMap::new();
    let errors = DashMap::new();

    nemesis_paths.par_iter().for_each(|patch_path| {
        let mut patch_map_owned = PatchMapOwned::new();

        collect_nemesis_paths(patch_path)
            .into_iter()
            .for_each(
                |txt_path| match parse_and_process_path(&txt_path, &options, &templates) {
                    Ok((template_name_key, patch_txt)) => {
                        patch_map_owned.insert(template_name_key, patch_txt);
                    }
                    Err(err) => {
                        errors.insert(txt_path.clone(), err);
                    }
                },
            );

        if let Some(mod_code) = patch_path.file_name() {
            patch_mod_map.insert(mod_code.to_string_lossy().to_string(), patch_map_owned);
        }
    });

    // Combine results from DashMap into regular structures
    let templates: TemplateMap = templates.into_iter().collect();
    let patch_mod_map: PatchModMap = patch_mod_map.into_iter().collect();
    let errors: Vec<Error> = errors.into_iter().map(|(_, err)| err).collect();

    if errors.is_empty() {
        Ok((templates, patch_mod_map))
    } else {
        Err(errors)
    }
}

fn parse_and_process_path(
    txt_path: &Path,
    options: &Options,
    templates: &TemplateMap,
) -> Result<(String, String), Error> {
    let NemesisPath {
        relevant_path: _,
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
    let (output_path, template_path) = match is_1stperson {
        true => FIRST_PERSON_BEHAVIORS.get(&template_name),
        false => THIRD_PERSON_BEHAVIORS.get(&template_name),
    }
    .map(|path| {
        let Options {
            resource_dir,
            output_dir,
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

    // Process patch
    let patch_txt = fs::read_to_string(txt_path).context(FailedIoSnafu {
        path: txt_path.to_path_buf(),
    })?;

    Ok((template_name_key, patch_txt))
}

fn template_xml_to_value(path: PathBuf) -> Result<BorrowedValue<'static>> {
    let template_xml = fs::read_to_string(&path).context(FailedIoSnafu { path: path.clone() })?;
    let ast: ClassMap = serde_hkx::from_str(&template_xml)?;
    to_borrowed_value(ast).context(JsonSnafu { path }) // TODO: fix needless realloc
}
