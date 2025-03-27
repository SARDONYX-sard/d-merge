use crate::{
    aliases::{OwnedPatchMap, TemplatePatchMap},
    errors::{Error, FailedIoSnafu, NemesisXmlErrSnafu, Result},
    paths::{
        collect::collect_all_patch_paths,
        parse::{parse_nemesis_path, NemesisPath},
    },
    results::{filter_results, partition_results},
};
use dashmap::DashSet;
use nemesis_xml::patch::parse_nemesis_patch;
use rayon::prelude::*;
use snafu::ResultExt as _;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
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

pub fn collect_borrowed_patches(owned_patches: &OwnedPatchMap) -> (PatchResult, Vec<Error>) {
    let template_patch_map = TemplatePatchMap::new();
    let template_names = DashSet::new();

    // key: template_name hkx, value: index of class
    // DashMap<String, usize>
    let id_index = AtomicUsize::new(0); // Assumed non use nemesis ID

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
            let (xml, id_idx) =
                parse_nemesis_patch(xml).with_context(|_| NemesisXmlErrSnafu { path })?;

            // <hkobject name="#0100"> // hkbGraph
            //
            // <hkobject name="$name$3">
            // <hkobject name="$name$4">
            if let Some(id_idx) = id_idx {
                if let Some(idx) = id_idx.parse().ok() {
                    id_index.store(idx, Ordering::Release)
                };
            }

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
    (
        PatchResult {
            template_names,
            template_patch_map,
            id_index: id_index.load(Ordering::Acquire),
        },
        errors,
    )
}

pub struct PatchResult<'a> {
    pub template_names: DashSet<String>,
    pub template_patch_map: TemplatePatchMap<'a>,
    pub id_index: usize,
}
