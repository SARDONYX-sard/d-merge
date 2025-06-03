//! The Nemesis path consists of the following
//!
//! - Format: `Nemesis_Engine/mod/<id>/[1st_person/]<template name>/<patch index>.txt`
//! - e.g.: `/some/path/to/Nemesis_Engine/mod/flinch/1st_person/0_master/#0106.txt`.
//!
//! From here, we need to determine and load the xml to which the patch will be applied, so we use a table to load this.
//! Note that the dir paths above `mesh` can be optionally specified and concatenated as resource paths later.
use serde_hkx::errors::readable::ReadableError;
use snafu::{prelude::*, OptionExt};
use std::path::Path;
use winnow::{
    ascii::Caseless,
    combinator::{alt, not, peek, repeat, seq},
    error::{ContextError, ErrMode, StrContext::*, StrContextValue::*},
    token::any,
    ModalResult, Parser,
};

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

    let inner_path = parse_template_path(path)?;

    Ok((key, inner_path))
}

pub fn parse_template_path(path: &Path) -> Result<&str, TemplateError> {
    let input = path.to_str().ok_or(TemplateError::InvalidUtf8)?;

    parse_components
        .parse(input)
        .map_err(|e| TemplateError::MissingMeshesDir {
            source: ReadableError::from_parse(e),
        })
}

/// Case-insensitive take_until implementation using only winnow
fn take_until_caseless<'s>(
    tag: &'static str,
) -> impl Parser<&'s str, &'s str, ErrMode<ContextError>> {
    move |input: &mut &'s str| {
        let end = not((Caseless(tag), alt(('/', '\\'))));
        repeat::<_, _, (), _, _>(1.., (peek(end), any))
            .take()
            .parse_next(input)
    }
}

fn parse_components<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
    seq! {
        _: take_until_caseless("meshes").context(Expected(StringLiteral("meshes"))),
    }
    .parse_next(input)?;

    let inner_path = *input;
    repeat::<_, _, (), _, _>(1.., any).parse_next(input)?;
    Ok(inner_path)
}

#[derive(Debug, Snafu)]
pub enum TemplateError {
    #[snafu(display("Path is not valid UTF-8"))]
    InvalidUtf8,
    #[snafu(display("Missing file stem"))]
    MissingFileStem,
    #[snafu(transparent)]
    MissingMeshesDir { source: ReadableError },
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
            r"../../resource/assets/templates/meshes/actors/character/defaultmale_Project.bin",
            "defaultmale_Project",
            r"meshes/actors/character/defaultmale_Project.bin",
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

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_long_path() {
        assert_template(
            r"D:release-no-lto\assets\templates\meshes\actors\character\_1stperson\characters\firstperson.bin",
            "_1stperson\\firstperson",
            r"meshes\actors\character\_1stperson\characters\firstperson.bin",
        );
        let path = r"\\?\D:\rust\d-merge\target\release-no-lto\assets\templates\meshes\actors\character\_1stperson\behaviors\blockbehavior.bin";
        assert_template(
            path,
            r"_1stperson\blockbehavior",
            r"meshes\actors\character\_1stperson\behaviors\blockbehavior.bin",
        );
    }

    #[test]
    fn test_missing_meshes_dir() {
        let path = Path::new("data/not_meshes_dir/character/skeleton.nif");
        assert!(template_name_and_inner_path(path).is_err());
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
