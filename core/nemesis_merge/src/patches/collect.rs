use crate::{
    aliases::{OwnedPatchMap, PtrMap, TemplatePatchMap},
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

pub fn collect_borrowed_patches(owned_patches: &OwnedPatchMap) -> (PatchResult, Vec<Error>) {
    let template_patch_map = TemplatePatchMap::new();
    let template_names = DashSet::new();
    let ptr_map = PtrMap::new();

    let results: Vec<Result<()>> = owned_patches
        .par_iter()
        .map(|(path, xml)| {
            let NemesisPath {
                mod_code,
                template_name,
                index,
            } = parse_nemesis_path(path)?;
            template_names.insert(template_name.clone());

            let patch_idx_map = template_patch_map.entry(template_name.clone()).or_default();
            let (xml, ptr) =
                parse_nemesis_patch(xml).with_context(|_| NemesisXmlErrSnafu { path })?;

            // ptr == `#0100`
            //
            // ```xml
            // <hkobject name="#0100" class="hkbBehaviorGraphStringData"></hkobject>
            // <hkobject name="$name$3" class="hkbBehaviorGraphStringData"></hkobject>
            // <hkobject name="$name$4" class="hkbBehaviorGraphStringData"></hkobject>
            // ```
            if let Some(ptr) = ptr {
                ptr_map.0.entry(template_name).or_insert(ptr);
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
            ptr_map,
        },
        errors,
    )
}

pub struct PatchResult<'a> {
    pub template_names: DashSet<String>,
    pub template_patch_map: TemplatePatchMap<'a>,
    pub ptr_map: PtrMap<'a>,
}
