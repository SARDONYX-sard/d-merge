//! Parses an adsf path and returns target and id as &str references.
//!
//! rule:
//! txt projects:
//! - format: <any>/Nemesis_Engine/mod/<id>/animationsetdatasinglefile/$header$/$header$.txt
//!   (e.g. D:/mod/Test/animationsetdatasinglefile/$header$/$header$.txt)
//!
//! replace anim_set_data path:
//! - format: <any>/Nemesis_Engine/mod/<id>/animationsetdatasinglefile/<project data>/<anim set file stem>.txt
//!   (e.g. D:/GAME/Test mod name/Nemesis_Engine/mod/id/animationsetdatasinglefile/DefaultMaleData~DefaultMale/_MTSolo.txt)
//!
//! Parses an adsf path and returns target, id, and parser type.
use std::{
    num::ParseIntError,
    path::{Path, PathBuf},
};

use snafu::ResultExt;

use super::path_parser::parse_error::IndexMustBeNumberSnafu;
use crate::behaviors::priority_ids::get_nemesis_id;

// TODO: Support replace operation

/// Represents the type of parser required for a given animation patch path.
#[derive(Debug, PartialEq)]
pub enum ParserType<'a> {
    /// Indicates the special `$header$/$header$.txt`override
    ///
    /// e.g. `DefaultMaleData~DefaultMale`
    TxtProjectHeader,

    /// AnimSetData file name (e.g., `_MTSolo.txt`)
    EditAnimSet(&'a str),
}

/// Represents the parsed result of an animation patch path.
///
/// This contains the mod ID, the animation target (e.g., `Default` or `$header$`),
/// the type of data being patched, and whether it's an add or replace operation.
#[derive(Debug, PartialEq)]
pub struct ParsedAsdsfPatchPath<'a> {
    /// Unique ID corresponding to the mod(e.g. `slide`)
    pub id: &'a str,
    /// txt project name data (e.g. `DefaultMaleData~DefaultMale`)
    ///
    /// The key to `AltAsdsf`.
    pub target: &'a str,
    /// Type of parser logic required
    pub parser_type: ParserType<'a>,
}

/// Parses an ADSF(`animationdatasinglefile`) patch path and extracts the relevant metadata.
///
/// # Returns
/// Returns a [`ParsedAdsfPatchPath`] with extracted metadata or a [`ParseError`] if the format is invalid.
pub fn parse_asdsf_path<'a>(path: &'a Path) -> Result<ParsedAsdsfPatchPath<'a>, ParseError> {
    let components: Vec<&'a str> = path
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();

    let anim_data_index = components
        .iter()
        .position(|comp| comp.eq_ignore_ascii_case("animationsetdatasinglefile"))
        .ok_or_else(|| ParseError::MissingAnimationSetData {
            path: path.to_path_buf(),
        })?;

    if anim_data_index < 1 || components.len() <= anim_data_index + 2 {
        return Err(ParseError::TooShortPathFormat {
            path: path.to_path_buf(),
        });
    }

    let path_str = path.to_str().ok_or_else(|| ParseError::NonUtf8Path {
        path: path.to_path_buf(),
    })?;
    let id = get_nemesis_id(path_str)?;

    // e.g. `DefaultMaleData~DefaultMale`
    let target_component = components[anim_data_index + 1];
    let target = if target_component.eq_ignore_ascii_case("$header$") {
        "$header$"
    } else {
        target_component
    };

    let file_stem = path.file_stem().and_then(|s| s.to_str()).ok_or_else(|| {
        ParseError::TooShortPathFormat {
            path: path.to_path_buf(),
        }
    })?;
    let is_header_file = file_stem.eq_ignore_ascii_case("$header$");

    let parser_type = if target == "$header$" && is_header_file {
        ParserType::TxtProjectHeader
    } else {
        let _index: usize = file_stem.parse().with_context(|_| IndexMustBeNumberSnafu {
            index_str: (*file_stem).to_string(),
            path,
        })?;
        ParserType::EditAnimSet(file_stem)
    };

    Ok(ParsedAsdsfPatchPath {
        target,
        id,
        parser_type,
    })
}

/// Represents parsing errors from `parse_adsf_path`.
#[derive(Debug, snafu::Snafu)]
#[snafu(module)]
#[allow(clippy::enum_variant_names)]
pub enum ParseError {
    #[snafu(transparent)]
    MissingID {
        source: serde_hkx::errors::readable::ReadableError,
    },

    #[snafu(display("Non-UTF-8 path: {}", path.display()))]
    NonUtf8Path { path: PathBuf },

    /// "animationdatasinglefile" not found in path
    #[snafu(display(
        "The path '{}' does not contain the required 'animationsetdatasinglefile' directory.\n\
Expected a structure like: D:/mod/<id>/animationsetdatasinglefile/...",
        path.display()
    ))]
    MissingAnimationSetData { path: PathBuf },

    /// Path does not have enough segments to extract data
    #[snafu(display(
        "The path '{}' is too short to extract the mod ID and target.\n\
Expected format: D:/mod/<id>/animationsetdatasinglefile/<target>~1/...",
        path.display()
    ))]
    TooShortPathFormat { path: PathBuf },

    /// Target component doesn't follow the expected `Target~1` format
    #[snafu(display(
        "The target segment in path '{}' does not follow the expected '<target>~1' format.\
        Example: 'DefaultFemale~1'\n",
        path.display()
    ))]
    SplitTilde { path: PathBuf },

    /// Target component doesn't follow the expected `Target~1` format
    #[snafu(display( "Replace/Remove Edit patches expect index, i.e., numeric filenames. However, this {index_str} of path is different. {}", path.display()))]
    IndexMustBeNumber {
        source: ParseIntError,
        index_str: String,
        path: PathBuf,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(p: &str) -> ParsedAsdsfPatchPath<'_> {
        parse_asdsf_path(Path::new(p)).expect("should parse correctly")
    }

    #[test]
    fn test_txt_project_header_add() {
        let parsed = parse(
            "/some/mods/Nemesis_Engine/mod/slide/animationdatasinglefile/$header$/$header$.txt",
        );
        assert_eq!(
            parsed,
            ParsedAsdsfPatchPath {
                id: "/some/mods/Nemesis_Engine/mod/slide",
                target: "$header$",
                parser_type: ParserType::TxtProjectHeader,
            }
        );
    }

    #[test]
    fn test_anim_replace() {
        let parsed = parse(
            "/some/mods/Nemesis_Engine/mod/slide/animationsetdatasinglefile/DefaultMaleData~DefaultMale/_MTSolo.txt",
        );
        assert_eq!(
            parsed,
            ParsedAsdsfPatchPath {
                id: "/some/mods/Nemesis_Engine/mod/slide",
                target: "DefaultMaleData~DefaultMale",
                parser_type: ParserType::EditAnimSet("_MTSolo.txt"),
            }
        );
    }
}
