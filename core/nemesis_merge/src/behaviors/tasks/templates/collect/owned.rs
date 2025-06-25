use rayon::prelude::*;
use std::{collections::HashSet, path::Path};

use crate::behaviors::tasks::templates::types::{OwnedTemplateMap, TemplateKey};

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

            let is_1st_person = path
                .components()
                .any(|c| c.as_os_str().eq_ignore_ascii_case("_1stperson"));
            template_names.get(&TemplateKey::new(file_stem.to_str()?, is_1st_person))?;

            let bytes = std::fs::read(&path).ok()?;
            Some((path, bytes))
        })
        .collect();

    map
}
