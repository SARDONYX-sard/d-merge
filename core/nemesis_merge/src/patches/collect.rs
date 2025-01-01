use crate::{
    aliases::{OwnedPatchMap, TemplatePatchMap},
    errors::{Error, FailedIoSnafu, NemesisXmlErrSnafu, Result},
    output_path::{parse_nemesis_path, NemesisPath},
    paths::collect::collect_all_patch_paths,
    results::{filter_results, partition_results},
};
use dashmap::DashSet;
use nemesis_xml::patch::parse_nemesis_patch;
use rayon::prelude::*;
use snafu::ResultExt as _;
use std::{fs, path::PathBuf};

pub fn collect_owned_patches(nemesis_paths: &[PathBuf]) -> Result<OwnedPatchMap, Vec<Error>> {
    let results: Vec<Result<(PathBuf, String)>> = collect_all_patch_paths(nemesis_paths)
        .into_par_iter()
        .map(|path| {
            let xml =
                fs::read_to_string(&path).with_context(|_| FailedIoSnafu { path: path.clone() })?;
            Ok((path, xml))
        })
        .collect();

    partition_results(results)
}

pub fn collect_borrowed_patches(
    owned_patches: &OwnedPatchMap,
) -> ((DashSet<String>, TemplatePatchMap<'_>), Vec<Error>) {
    let template_patch_map = TemplatePatchMap::new();
    let template_names = DashSet::new();

    let results: Vec<Result<()>> = owned_patches
        .par_iter()
        .map(|(path, xml)| {
            let NemesisPath {
                mod_code,
                template_name,
                index,
            } = parse_nemesis_path(path)?;
            template_names.insert(template_name.clone());

            let patch_idx_map = template_patch_map.entry(template_name).or_default();
            let xml = parse_nemesis_patch(xml).with_context(|_| NemesisXmlErrSnafu { path })?;

            patch_idx_map
                .entry(index)
                .or_default()
                .insert(mod_code, xml);
            Ok(())
        })
        .collect();

    let errors = match filter_results(results) {
        Ok(_) => vec![],
        Err(errors) => errors,
    };
    ((template_names, template_patch_map), errors)
}
