//! The Nemesis path consists of the following
//!
//! - Format:
//!   - message_pack JsonValue: `meshes/../[1st_person/]<template name>.bin`
//!   -                    XML: `meshes/../[1st_person/]<template name>.xml`
//!
//! - e.g.: `../../resource/assets/templates/meshes/actors/character/defaultmale_Project.bin`.
//!
//! From here, we need to determine and load the xml to which the patch will be applied, so we use a table to load this.
//! Note that the dir paths above `mesh` can be optionally specified and concatenated as resource paths later.
use serde_hkx::errors::readable::ReadableError;
use snafu::prelude::*;
use std::path::Path;
use winnow::{
    ascii::Caseless,
    combinator::{alt, repeat, seq},
    error::{StrContext::*, StrContextValue::*},
    token::any,
    ModalResult, Parser,
};

use crate::behaviors::{priority_ids::take_until_ext, tasks::templates::key::TemplateKey};

/// Return (hashmap key, inner path starting from `meshes`)
pub(crate) fn template_name_and_inner_path(path: &Path) -> Result<TemplateKey<'_>, TemplateError> {
    Ok(unsafe {
        TemplateKey::new_unchecked(std::borrow::Cow::Borrowed(parse_template_path(path)?))
    })
}

pub fn parse_template_path(path: &Path) -> Result<&str, TemplateError> {
    let input = path.to_str().ok_or(TemplateError::InvalidUtf8)?;

    parse_components
        .parse(input)
        .map_err(|e| TemplateError::MissingMeshesDir {
            source: ReadableError::from_parse(e),
        })
}

fn parse_components<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
    seq! {
        _: take_until_ext(1.., (Caseless("meshes"), alt(('/', '\\')))).context(Expected(StringLiteral("meshes"))),
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

    fn assert_template(path: &str, expected_inner: &str) {
        let path = Path::new(path);
        let result = template_name_and_inner_path(path).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(result.as_str(), expected_inner);
    }

    #[test]
    fn test_regular_path() {
        assert_template(
            r"../../resource/assets/templates/meshes/actors/character/defaultmale_Project.bin",
            r"meshes/actors/character/defaultmale_Project.bin",
        );

        assert_template(
            "data/meshes/actors/character/behaviors/weapequip.xml",
            "meshes/actors/character/behaviors/weapequip.xml",
        );
    }

    #[test]
    fn test_with_backslashes_and_1stperson() {
        assert_template(
            r"data/meshes/actors/character/character assets/_1stperson/skeleton.nif",
            r"meshes/actors/character/character assets/_1stperson/skeleton.nif",
        );
    }

    #[test]
    fn test_with_slashes_and_1stperson() {
        assert_template(
            "Data/Meshes/_1stPerson/something.nif",
            "Meshes/_1stPerson/something.nif",
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_long_path() {
        assert_template(
            r"D:release-no-lto\assets\templates\meshes\actors\character\_1stperson\characters\firstperson.bin",
            r"meshes\actors\character\_1stperson\characters\firstperson.bin",
        );
        let path = r"\\?\D:\rust\d-merge\target\release-no-lto\assets\templates\meshes\actors\character\_1stperson\behaviors\blockbehavior.bin";
        assert_template(
            path,
            r"meshes\actors\character\_1stperson\behaviors\blockbehavior.bin",
        );
    }

    #[test]
    fn test_missing_meshes_dir() {
        let path = Path::new("data/not_meshes_dir/character/skeleton.nif");
        assert!(template_name_and_inner_path(path).is_err());
    }
}
