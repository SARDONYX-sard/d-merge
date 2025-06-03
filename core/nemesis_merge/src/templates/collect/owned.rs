use dashmap::DashSet;
use rayon::prelude::*;
use std::path::Path;

use crate::types::OwnedTemplateMap;

/// Return HashMap<template key, `meshes` inner path>
pub fn collect_templates<P>(path: P, template_names: DashSet<&str>) -> OwnedTemplateMap
where
    P: AsRef<Path>,
{
    let map: OwnedTemplateMap = jwalk::WalkDir::new(path)
        .into_iter()
        .par_bridge()
        .into_par_iter()
        .flat_map(|path| {
            let path = path.ok()?.path();
            if !path.is_file() {
                return None;
            }
            let file_stem = path.file_stem()?;
            if file_stem.eq_ignore_ascii_case("animationdatasinglefile") {
                return None;
            }
            if !template_names.contains(file_stem.to_str()?) {
                return None;
            }
            let needed_template_path = path;

            let bytes = std::fs::read(&needed_template_path).ok()?;
            Some((needed_template_path, bytes))
        })
        .collect();

    map
}
