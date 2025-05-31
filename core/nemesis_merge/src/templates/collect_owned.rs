//! The Nemesis path consists of the following
//!
//! - Format: `Nemesis_Engine/mod/<id>/[1st_person/]<template name>/<patch index>.txt`
//! - e.g.: `/some/path/to/Nemesis_Engine/mod/flinch/1st_person/0_master/#0106.txt`.
//!
//! From here, we need to determine and load the xml to which the patch will be applied, so we use a table to load this.
//! Note that the dir paths above `mesh` can be optionally specified and concatenated as resource paths later.
use std::path::{Path, PathBuf};

/// Return HashMap<template key, `meshes` inner path>
pub fn collect_owned_templates<P>(path: P) -> dashmap::DashMap<String, PathBuf>
where
    P: AsRef<Path>,
{
    use rayon::prelude::*;

    jwalk::WalkDir::new(path)
        .into_iter()
        .par_bridge()
        .filter_map(|path| template_name_and_inner_path(&path.ok()?.path()))
        .collect()
}

/// Return (hashmap key, meshes inner path)
fn template_name_and_inner_path(path: &Path) -> Option<(String, PathBuf)> {
    let is_xml = path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("xml"));
    if !is_xml {
        return None;
    }

    let key = {
        let file_stem = path.file_stem()?.to_string_lossy();

        let mut components = path.components();
        let is_1stperson = components.any(|c| c.as_os_str().eq_ignore_ascii_case("_1stPerson"));
        if is_1stperson {
            if path.to_str().is_some_and(|s| s.contains("\\")) {
                format!("_1stperson\\{file_stem}")
            } else {
                format!("_1stperson/{file_stem}")
            }
        } else {
            file_stem.to_string()
        }
    };

    let inner_path = {
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

    Some((key, inner_path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_1st_person() {
        let path =
            Path::new("resource/assets/templates/meshes/actors/character/behaviors/weapequip.xml");
        match template_name_and_inner_path(path) {
            Some((key, value)) => {
                assert_eq!(key, "weapequip");
                assert_eq!(
                    value,
                    Path::new("meshes/actors/character/behaviors/weapequip.xml")
                );
            }
            None => panic!("Invalid path"),
        }
    }

    #[test]
    fn test_1st_person_character() {
        let path = Path::new(
            "resource/assets/templates/meshes/actors/character/_1stperson/characters/firstperson.xml",
        );
        match template_name_and_inner_path(path) {
            Some((key, value)) => {
                assert_eq!(key, "_1stperson/firstperson");
                assert_eq!(
                    value,
                    Path::new("meshes/actors/character/_1stperson/characters/firstperson.xml")
                );
            }
            None => panic!("Invalid path"),
        }
    }
}
