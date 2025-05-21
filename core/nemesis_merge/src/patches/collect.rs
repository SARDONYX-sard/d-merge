use crate::{
    aliases::{OwnedPatchMap, PtrMap, TemplatePatchMap},
    errors::{Error, FailedIoSnafu, NemesisXmlErrSnafu, Result},
    paths::{
        collect::collect_nemesis_paths,
        parse::{parse_nemesis_path, NemesisPath},
    },
    results::filter_results,
};
use dashmap::DashSet;
use nemesis_xml::patch::parse_nemesis_patch;
use rayon::prelude::*;
use snafu::ResultExt as _;
use std::path::PathBuf;
use tokio::fs;

/// Collects all patches from the given nemesis paths and returns a map of owned patches.
///
/// # Errors
/// Returns an error if any of the paths cannot be read or parsed.
pub async fn collect_owned_patches(nemesis_paths: &[PathBuf]) -> Result<OwnedPatchMap, Vec<Error>> {
    let mut handles = vec![];

    let paths = nemesis_paths.iter().flat_map(collect_nemesis_paths);
    for path in paths {
        handles.push(tokio::spawn(async move {
            let xml = fs::read_to_string(&path)
                .await
                .with_context(|_| FailedIoSnafu { path: path.clone() })?;
            Ok((path, xml))
        }));
    }

    let mut owned_patches = OwnedPatchMap::new();
    let mut errors = vec![];
    for handle in handles {
        let result = match handle.await {
            Ok(result) => result,
            Err(err) => {
                errors.push(Error::JoinError { source: err });
                continue;
            }
        };

        match result {
            Ok((path, xml)) => {
                owned_patches.insert(path, xml);
            }
            Err(err) => {
                errors.push(err);
            }
        }
    }

    if errors.is_empty() {
        Ok(owned_patches)
    } else {
        Err(errors)
    }
}

pub fn collect_borrowed_patches(owned_patches: &OwnedPatchMap) -> (BorrowedPatches, Vec<Error>) {
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
        BorrowedPatches {
            template_names,
            template_patch_map,
            ptr_map,
        },
        errors,
    )
}

pub struct BorrowedPatches<'a> {
    pub template_names: DashSet<String>,
    pub template_patch_map: TemplatePatchMap<'a>,
    pub ptr_map: PtrMap<'a>,
}
