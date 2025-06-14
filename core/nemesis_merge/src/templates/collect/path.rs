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
use snafu::{prelude::*, OptionExt};
use std::path::Path;
use winnow::{
    ascii::Caseless,
    combinator::{alt, repeat, seq},
    error::{StrContext::*, StrContextValue::*},
    token::any,
    ModalResult, Parser,
};

use crate::types::Key;

/// Return (hashmap key, inner path starting from `meshes`)
pub(super) fn template_name_and_inner_path(path: &Path) -> Result<(Key<'_>, &str), TemplateError> {
    let file_stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .context(MissingFileStemSnafu)?;

    let is_1stperson = path
        .components()
        .any(|c| c.as_os_str().eq_ignore_ascii_case("_1stperson"));

    let key = Key::new(file_stem, is_1stperson);
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

/// take_until implementation using only winnow
fn take_until_ext<Input, Output, Error, ParseNext>(
    occurrences: impl Into<winnow::stream::Range>,
    parser: ParseNext,
) -> impl Parser<Input, Input::Slice, Error>
where
    Input: winnow::stream::StreamIsPartial + winnow::stream::Stream,
    Error: winnow::error::ParserError<Input>,
    ParseNext: Parser<Input, Output, Error>,
{
    use winnow::combinator::{not, peek, repeat, trace};
    use winnow::token::any;

    trace(
        "take_until_ext",
        repeat::<_, _, (), _, _>(occurrences, (peek(not(parser)), any)).take(),
    )
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

    fn assert_template(path: &str, expected_key: (&str, bool), expected_inner: &str) {
        let path = Path::new(path);
        let result = template_name_and_inner_path(path).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(result.0, Key::new(expected_key.0, expected_key.1));
        assert_eq!(result.1, expected_inner);
    }

    #[test]
    fn test_regular_path() {
        assert_template(
            r"../../resource/assets/templates/meshes/actors/character/defaultmale_Project.bin",
            ("defaultmale_Project", false),
            r"meshes/actors/character/defaultmale_Project.bin",
        );

        assert_template(
            "data/meshes/actors/character/behaviors/weapequip.xml",
            ("weapequip", false),
            "meshes/actors/character/behaviors/weapequip.xml",
        );
    }

    #[test]
    fn test_with_backslashes_and_1stperson() {
        assert_template(
            r"data/meshes/actors/character/character assets/_1stperson/skeleton.nif",
            ("skeleton", true),
            r"meshes/actors/character/character assets/_1stperson/skeleton.nif",
        );
    }

    #[test]
    fn test_with_slashes_and_1stperson() {
        assert_template(
            "Data/Meshes/_1stPerson/something.nif",
            ("something", true),
            "Meshes/_1stPerson/something.nif",
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_long_path() {
        assert_template(
            r"D:release-no-lto\assets\templates\meshes\actors\character\_1stperson\characters\firstperson.bin",
            ("firstperson", true),
            r"meshes\actors\character\_1stperson\characters\firstperson.bin",
        );
        let path = r"\\?\D:\rust\d-merge\target\release-no-lto\assets\templates\meshes\actors\character\_1stperson\behaviors\blockbehavior.bin";
        assert_template(
            path,
            ("blockbehavior", true),
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

            let Ok((key, inner_path)) = template_name_and_inner_path(&path) else {
                continue;
            };

            if let Some(prev) = map.insert(key.to_string(), inner_path.to_string()) {
                overwrite.push((inner_path.to_string(), prev));
            };
        }

        dbg!(&overwrite);
        dbg!(overwrite.len());
    }
}
