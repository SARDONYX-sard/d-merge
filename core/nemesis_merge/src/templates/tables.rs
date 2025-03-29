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

pub fn collect_table_paths<P>(path: P) -> DashMap<String, PathBuf>
where
    P: AsRef<Path>,
{
    let paths: Vec<PathBuf> = jwalk::WalkDir::new(path)
        .into_iter()
        .filter_map(|res| validate_template_path(res.ok()?.path()))
        .collect();

    paths
        .into_par_iter()
        .filter_map(|path| get_key_value(&path))
        .collect()
}

// - key: (`_1stperson`) + file stem
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

/// Ignore
/// - `meshes/actors/character/_1stperson/file_stem.xml` or
/// - `meshes/actors/character/file_stem.xml`
///
/// Expected
/// - `meshes/actors/character/_1stperson/<middle dir>/file_stem.xml` -> rev 1 skip, `_1stperson` -> valid 1st person
/// - `meshes/actors/character/<middle dir>/file_stem.xml` -> rev 2 skip, then `actors` -> valid 3rd person
pub fn validate_template_path(path: PathBuf) -> Option<PathBuf> {
    let is_xml = path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("xml"));
    if !is_xml {
        return None;
    }

    // ignore until actors.next()
    let mut components = path.components();
    for comp in components.by_ref() {
        if comp.as_os_str().eq_ignore_ascii_case("actors") {
            break;
        }
    }

    // Skip `components.next()` => e.g. `character`, `troll`
    components.next();

    // _1stperson or `<middle dir>`(3rd person)
    let is_1stperson = components.next().is_some_and(|maybe_1stperson| {
        maybe_1stperson
            .as_os_str()
            .eq_ignore_ascii_case("_1stperson")
    });

    if is_1stperson {
        // skip middle dir for _1stperson (e.g. `characters`)
        components.next();
    }

    let file_stem = components.next()?.as_os_str().to_str()?;
    let ext_idx = file_stem.find('.')?;

    file_stem[ext_idx + 1..]
        .eq_ignore_ascii_case("xml")
        .then_some(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    // behaviors/1hm_behavior => 1hm_behavior
    #[test]
    fn test_1st_person() {
        let path =
            Path::new("resource/assets/templates/meshes/actors/character/behaviors/weapequip.xml");
        match get_key_value(path) {
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

    // _1stperson/characters/firstperson => _1stperson/firstperson
    #[test]
    fn test_1st_person_character() {
        let path = Path::new(
            "resource/assets/templates/meshes/actors/character/_1stperson/characters/firstperson.xml",
        );
        match get_key_value(path) {
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

    #[test]
    fn test_parse() {
        let path =
            Path::new("templates/meshes/actors/character/_1stperson/characters/firstperson.xml")
                .to_path_buf();
        assert_eq!(validate_template_path(path.clone()), Some(path));
    }
}
