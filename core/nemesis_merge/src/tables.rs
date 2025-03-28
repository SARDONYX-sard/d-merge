//! The Nemesis path consists of the following
//!
//! - Format: `Nemesis_Engine/mod/<id>/[1st_person/]<template name>/<patch index>.txt`
//! - e.g.: `/some/path/to/Nemesis_Engine/mod/flinch/1st_person/0_master/#0106.txt`.
//!
//! From here, we need to determine and load the xml to which the patch will be applied, so we use a table to load this.
//! Note that the dir paths above `mesh` can be optionally specified and concatenated as resource paths later.
use dashmap::DashMap;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::path::{Path, PathBuf};

pub fn collect_nemesis_paths<P>(path: P) -> DashMap<String, PathBuf>
where
    P: AsRef<Path>,
{
    let paths: Vec<PathBuf> = jwalk::WalkDir::new(path)
        .into_iter()
        .filter_map(|res| {
            if let Ok(path) = res.map(|entry| entry.path()) {
                let ext = path
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("xml"));
                if ext {
                    return Some(path);
                }
            }
            None
        })
        .collect();

    paths
        .into_par_iter()
        .filter_map(|path| {
            let key = key_from_path(&path)?;
            Some((key, path))
        })
        .collect()
}

fn key_from_path(path: &Path) -> Option<String> {
    let file_stem = path.file_stem()?.to_string_lossy();
    let mut components = path.components();

    let is_1stperson = components.any(|c| c.as_os_str().eq_ignore_ascii_case("_1stPerson"));

    Some(if is_1stperson {
        format!("_1stperson/{file_stem}")
    } else {
        file_stem.to_string()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_1st_person() {
        let path = Path::new(
            "resource/assets/templates/meshes/actors/character/_1stperson/behaviors/weapequip.xml",
        );

        assert_eq!(key_from_path(path).as_deref(), Some("_1stperson/weapequip"));
    }
}
