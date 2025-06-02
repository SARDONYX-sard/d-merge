//! The Nemesis path consists of the following
//!
//! - Format: `Nemesis_Engine/mod/<id>/[1st_person/]<template name>/<patch index>.txt`
//! - e.g.: `/some/path/to/Nemesis_Engine/mod/flinch/1st_person/0_master/#0106.txt`.
//!
//! From here, we need to determine and load the xml to which the patch will be applied, so we use a table to load this.
//! Note that the dir paths above `mesh` can be optionally specified and concatenated as resource paths later.
use snafu::{prelude::*, OptionExt};
use std::path::Path;

/// Return (hashmap key, inner path starting from `meshes`)
pub(super) fn template_name_and_inner_path(path: &Path) -> Result<(String, &str), TemplateError> {
    let file_stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .context(MissingFileStemSnafu)?;

    let path_str = path.to_str().context(InvalidUtf8Snafu)?;

    let is_1stperson = path
        .components()
        .any(|c| c.as_os_str().eq_ignore_ascii_case("_1stperson"));

    let key = if is_1stperson {
        if path_str.contains('\\') {
            format!("_1stperson\\{file_stem}")
        } else {
            format!("_1stperson/{file_stem}")
        }
    } else {
        file_stem.to_string()
    };

    let mut search_offset = 0;
    for comp in path.components() {
        let comp_str = comp.as_os_str().to_str().context(InvalidUtf8Snafu)?;

        if comp_str.eq_ignore_ascii_case("meshes") || comp_str.eq_ignore_ascii_case("mesh") {
            if let Some(pos) = path_str[search_offset..].find(comp_str) {
                let idx = search_offset + pos;
                let inner = &path_str[idx..];
                return Ok((key, inner));
            }
        }

        // Safety: path_str is valid UTF-8, so this is safe
        if let Some(comp_len) = comp.as_os_str().to_str().map(|s| s.len()) {
            search_offset += comp_len;
            if search_offset < path_str.len() {
                search_offset += 1;
            }
        }
    }

    Err(TemplateError::MissingMeshesDir)
}

#[derive(Debug, Snafu)]
pub enum TemplateError {
    #[snafu(display("Path is not valid UTF-8"))]
    InvalidUtf8,
    #[snafu(display("Missing file stem"))]
    MissingFileStem,
    #[snafu(display("Could not find `meshes` directory"))]
    MissingMeshesDir,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_template(path: &str, expected_key: &str, expected_inner: &str) {
        let path = Path::new(path);
        let result = template_name_and_inner_path(path).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(result.0, expected_key);
        assert_eq!(result.1, expected_inner);
    }

    #[test]
    fn test_regular_path() {
        assert_template(
            r"../../resource/assets/templates\meshes\actors\character\defaultmale_Project.bin",
            "defaultmale_Project",
            r"meshes\actors\character\defaultmale_Project.bin",
        );

        assert_template(
            "data/meshes/actors/character/behaviors/weapequip.xml",
            "weapequip",
            "meshes/actors/character/behaviors/weapequip.xml",
        );
    }

    #[test]
    fn test_with_backslashes_and_1stperson() {
        assert_template(
            r"data/meshes/actors/character/character assets/_1stperson/skeleton.nif",
            "_1stperson/skeleton",
            r"meshes/actors/character/character assets/_1stperson/skeleton.nif",
        );
    }

    #[test]
    fn test_with_slashes_and_1stperson() {
        assert_template(
            "Data/Meshes/_1stPerson/something.nif",
            "_1stperson/something",
            "Meshes/_1stPerson/something.nif",
        );
    }

    #[test]
    fn test_missing_meshes_dir() {
        let path = Path::new("data/not_meshes_dir/character/skeleton.nif");
        let err = template_name_and_inner_path(path).unwrap_err();
        assert!(matches!(err, TemplateError::MissingMeshesDir));
    }

    #[ignore = "local only checker"]
    #[test]
    fn test_overwrite_path() {
        let path = "../../resource/assets";

        let mut map = indexmap::IndexMap::new();
        let mut overwrite = vec![];
        for result in jwalk::WalkDir::new(path) {
            let path = result.unwrap().path();

            if !path.is_file() {
                continue;
            }

            let Ok((template_name, inner_path)) = template_name_and_inner_path(&path) else {
                continue;
            };

            if let Some(prev) = map.insert(template_name, inner_path.to_string()) {
                overwrite.push((inner_path.to_string(), prev));
            };
        }

        dbg!(&overwrite);
        dbg!(overwrite.len());
    }
}
