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
        .filter_map(|path| get_key_value(&path))
        .collect()
}

/// - key: (`_1stperson`) + file stem
/// - value: Path after `mesh`
fn get_key_value(path: &Path) -> Option<(String, PathBuf)> {
    let key = {
        let file_stem = path.file_stem()?.to_string_lossy();
        let mut components = path.components();
        let is_1stperson = components.any(|c| c.as_os_str().eq_ignore_ascii_case("_1stPerson"));
        if is_1stperson {
            format!("_1stperson/{file_stem}")
        } else {
            file_stem.to_string()
        }
    };

    let value = {
        let mut value = None;
        let mut components = path.components();

        while let Some(comp) = components.next() {
            if comp.as_os_str().eq_ignore_ascii_case("mesh")
                || comp.as_os_str().eq_ignore_ascii_case("meshes")
            {
                let mut rel_path = PathBuf::from(comp.as_os_str()); // Include `mesh` too.
                rel_path.extend(components);
                value = Some(rel_path);
                break;
            }
        }
        value?
    };

    Some((key, value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_1st_person() {
        let path = Path::new(
            "resource/assets/templates/meshes/actors/character/_1stperson/behaviors/weapequip.xml",
        );
        match get_key_value(path) {
            Some((key, value)) => {
                assert_eq!(key, "_1stperson/weapequip");
                assert_eq!(
                    value,
                    Path::new("meshes/actors/character/_1stperson/behaviors/weapequip.xml")
                );
            }
            None => panic!("Invalid path"),
        }
    }
}
