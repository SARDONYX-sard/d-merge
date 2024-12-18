use super::{
    aliases::{BorrowedTemplateMap, OwnedPatchMap, TemplatePatchMap},
    results::filter_results,
    tables::TEMPLATE_BEHAVIORS,
};
use crate::{
    collect_path::collect_all_patch_paths,
    error::{Error, FailedIoSnafu, JsonSnafu, NemesisXmlErrSnafu, NotFoundTemplateSnafu, Result},
    merger::results::partition_results,
    output_path::{parse_nemesis_path, NemesisPath},
};
use dashmap::DashSet;
use nemesis_xml::patch::parse_nemesis_patch;
use rayon::{iter::Either, prelude::*};
use serde_hkx_features::ClassMap;
use simd_json::{serde::to_borrowed_value, BorrowedValue};
use snafu::{OptionExt, ResultExt as _};
use std::{
    fs,
    path::{Path, PathBuf},
};

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

pub fn collect_templates<'a>(
    template_names: DashSet<String>,
    resource_dir: &Path,
) -> (BorrowedTemplateMap<'a>, Vec<Error>) {
    let results: Vec<Result<(String, (&'static str, BorrowedValue<'static>))>> = template_names
        .into_par_iter()
        .map(|name| {
            let value = template_xml_to_value(name.as_str(), resource_dir)?;
            Ok((name, value))
        })
        .collect();

    results.into_par_iter().partition_map(|res| match res {
        Ok(value) => Either::Left(value),
        Err(err) => Either::Right(err),
    })
}

fn template_xml_to_value(
    template_name: &str,
    resource_dir: &Path,
) -> Result<(&'static str, BorrowedValue<'static>)> {
    let inner_path = TEMPLATE_BEHAVIORS
        .get(template_name)
        .with_context(|| NotFoundTemplateSnafu { template_name })?;

    let path = resource_dir.join(inner_path);
    let template_xml = fs::read_to_string(path).context(FailedIoSnafu { path: inner_path })?;
    let ast: ClassMap = serde_hkx::from_str(&template_xml)?;
    let value = to_borrowed_value(ast).with_context(|_| JsonSnafu { path: inner_path })?;
    Ok((inner_path, value))
}
