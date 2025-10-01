use rayon::prelude::*;
use snafu::ResultExt as _;
use std::{collections::HashSet, path::Path};

use crate::{
    behaviors::tasks::templates::{
        collect::path::template_name_and_inner_path, key::TemplateKey, types::OwnedTemplateMap,
    },
    errors::FailedToGetInnerPathFromTemplateSnafu,
};

/// Return HashMap<template key, `meshes` inner path>
pub fn collect_templates(
    path: &Path,
    template_names: HashSet<TemplateKey<'_>>,
) -> OwnedTemplateMap {
    let map: OwnedTemplateMap = jwalk::WalkDir::new(path)
        .into_iter()
        .par_bridge()
        .into_par_iter()
        .flat_map(|entry| {
            let path = entry.ok()?.path();
            if !path.is_file() {
                return None;
            }

            let file_stem = path.file_stem()?;
            if file_stem.eq_ignore_ascii_case("animationdatasinglefile") {
                return None;
            }

            let inner_path = match template_name_and_inner_path(&path)
                .with_context(|_| FailedToGetInnerPathFromTemplateSnafu { path: path.clone() })
            {
                Ok(inner_path) => inner_path,
                Err(e) => {
                    tracing::error!(%e);
                    return None;
                }
            };

            template_names.get(&inner_path)?;

            let bytes = std::fs::read(&path).ok()?;
            Some((path, bytes))
        })
        .collect();

    map
}
