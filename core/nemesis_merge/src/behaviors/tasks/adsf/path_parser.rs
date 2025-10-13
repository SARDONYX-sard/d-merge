//! Parses an adsf path and returns target and id as &str references.
//!
//! rule:
//! txt projects:
//! - format: <any>/<id>/animationdatasinglefile/$header$/$header$.txt
//!   (e.g. D:/mod/slide/animationdatasinglefile/$header$/$header$.txt)
//!
//! anim block header:
//! - format: <any>/<id>/animationdatasinglefile/<target>~<index>/$header$.txt
//!   (e.g. D:/mod/slide/animationdatasinglefile/<target>~<index>/$header$.txt)
//!
//! add anim block path:
//! - format: <any>/<id>/animationdatasinglefile/<target>~<index>/<name>~<anim_data_clip_id>.txt
//!   (e.g. D:/mod/slide/animationdatasinglefile/DefaultFemale~1/SprintSlide~slide$0.txt)
//!
//! replace anim block path:
//! - format: <any>/<id>/animationdatasinglefile/<target>~1/<name>~<array index>.txt
//!   (e.g. D:/mod/slide/animationdatasinglefile/DefaultFemale~1/MT_Jump~50.txt)
//!
//! add motion block path:
//! - format: <any>/<id>/animationdatasinglefile/<target>~1/<anim_data_clip_id>.txt
//!   (e.g. D:/mod/slide/animationdatasinglefile/DefaultFemale~1/slide$0.txt)
//!
//! replace motion block path:
//! - format: <any>/<id>/animationdatasinglefile/<target>~1/<array index>.txt
//!   (e.g. D:/mod/slide/animationdatasinglefile/DefaultFemale~1/50.txt)
//!
//! Parses an adsf path and returns target, id, and parser type.
use std::{
    num::ParseIntError,
    path::{Path, PathBuf},
};

use snafu::ResultExt;

use super::path_parser::parse_error::IndexMustBeNumberSnafu;
use crate::behaviors::priority_ids::get_nemesis_id;

/// Represents the type of parser required for a given animation patch path.
#[derive(Debug, PartialEq)]
pub enum ParserType<'a> {
    /// Indicates the special `$header$/$header$.txt`override
    TxtProjectHeader,

    /// Indicates the special `<target>~<index>/$header$.txt`override
    AnimHeader,

    /// Indicates an individual animation (e.g., `Run~slide.txt`)
    AddAnim,

    /// `<Name>~<clip_id>`
    /// - e.g. `Jump~42`
    ///
    /// NOTE: Unlike Motion, Anim sometimes references the same clip_id, so it cannot be used as an id.
    /// Therefore, Name is used instead
    EditAnim(&'a str),

    /// Indicates a motion ID add(e.g., `slide$10.txt`)
    AddMotion,

    /// Indicates a motion ID replacement or override (e.g., `10.txt` 10 is AnimInfo index)
    /// - include 1-based index
    EditMotion(&'a str),
}

/// Represents the parsed result of an animation patch path.
///
/// This contains the mod ID, the animation target (e.g., `Default` or `$header$`),
/// the type of data being patched, and whether it's an add or replace operation.
#[derive(Debug, PartialEq)]
pub struct ParsedAdsfPatchPath<'a> {
    /// Unique ID corresponding to the mod(e.g. `slide`)
    pub id: &'a str,
    /// `project_name~index` (e.g. `DefaultMale~1`)
    ///
    /// # What is meant by index here is
    /// project_names ends with `.txt` and it is sometimes a duplicate name. So, it seems to make the index be specified.
    pub target: &'a str,
    /// Type of parser logic required
    pub parser_type: ParserType<'a>,
}

/// Parses an ADSF(`animationdatasinglefile`) patch path and extracts the relevant metadata.
///
/// # Returns
/// Returns a [`ParsedAdsfPatchPath`] with extracted metadata or a [`ParseError`] if the format is invalid.
pub fn parse_adsf_path<'a>(path: &'a Path) -> Result<ParsedAdsfPatchPath<'a>, ParseError> {
    let components: Vec<&'a str> = path
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();

    let anim_data_index = components
        .iter()
        .position(|comp| comp.eq_ignore_ascii_case("animationdatasinglefile"))
        .ok_or_else(|| ParseError::MissingAnimationData {
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

    let target_component = components[anim_data_index + 1];
    let target = if target_component.eq_ignore_ascii_case("$header$") {
        "$header$"
    } else if target_component.contains('~') {
        target_component
    } else {
        return Err(ParseError::SplitTilde {
            path: path.to_path_buf(),
        });
    };

    let file_stem = path.file_stem().and_then(|s| s.to_str()).ok_or_else(|| {
        ParseError::TooShortPathFormat {
            path: path.to_path_buf(),
        }
    })?;
    let is_header_file = file_stem.eq_ignore_ascii_case("$header$");

    let parser_type = if target == "$header$" && is_header_file {
        ParserType::TxtProjectHeader
    } else if is_header_file {
        ParserType::AnimHeader
    } else if file_stem.contains('~') {
        // e.g. Jump~42
        let mut parts = file_stem.rsplitn(2, '~');
        // rsplitn is reverse getter. -> 42, jump
        if let (Some(index_str), Some(_name)) = (parts.next(), parts.next()) {
            match index_str.parse::<usize>() {
                Ok(_) => ParserType::EditAnim(file_stem),
                Err(_) => ParserType::AddAnim,
            }
        } else {
            ParserType::AddAnim
        }
    } else if file_stem.contains('$') {
        ParserType::AddMotion
    } else {
        let _index: usize = file_stem.parse().with_context(|_| IndexMustBeNumberSnafu {
            index_str: (*file_stem).to_string(),
            path,
        })?;
        ParserType::EditMotion(file_stem)
    };

    Ok(ParsedAdsfPatchPath {
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
        "The path '{}' does not contain the required 'animationdatasinglefile' directory.\n\
Expected a structure like: D:/mod/<id>/animationdatasinglefile/...",
        path.display()
    ))]
    MissingAnimationData { path: PathBuf },

    /// Path does not have enough segments to extract data
    #[snafu(display(
        "The path '{}' is too short to extract the mod ID and target.\n\
Expected format: D:/mod/<id>/animationdatasinglefile/<target>~1/...",
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

    fn parse(p: &str) -> ParsedAdsfPatchPath<'_> {
        parse_adsf_path(Path::new(p)).expect("should parse correctly")
    }

    #[test]
    fn test_txt_project_header_add() {
        let parsed = parse(
            "/some/mods/Nemesis_Engine/mod/slide/animationdatasinglefile/$header$/$header$.txt",
        );
        assert_eq!(
            parsed,
            ParsedAdsfPatchPath {
                id: "/some/mods/Nemesis_Engine/mod/slide",
                target: "$header$",
                parser_type: ParserType::TxtProjectHeader,
            }
        );
    }

    #[test]
    fn test_anim_header_add() {
        let parsed = parse(
            "/some/mods/Nemesis_Engine/mod/slide/animationdatasinglefile/DefaultMale~3/$header$.txt",
        );
        assert_eq!(
            parsed,
            ParsedAdsfPatchPath {
                id: "/some/mods/Nemesis_Engine/mod/slide",
                target: "DefaultMale~3",
                parser_type: ParserType::AnimHeader,
            }
        );
    }

    #[test]
    fn test_anim_add() {
        let parsed = parse(
            "/some/mods/Nemesis_Engine/mod/slide/animationdatasinglefile/Default~1/RunForward~slide$2.txt",
        );
        assert_eq!(
            parsed,
            ParsedAdsfPatchPath {
                id: "/some/mods/Nemesis_Engine/mod/slide",
                target: "Default~1",
                parser_type: ParserType::AddAnim,
            }
        );
    }

    #[test]
    fn test_anim_replace() {
        let parsed = parse(
            "/some/mods/Nemesis_Engine/mod/slide/animationdatasinglefile/Default~1/Jump~42.txt",
        );
        assert_eq!(
            parsed,
            ParsedAdsfPatchPath {
                id: "/some/mods/Nemesis_Engine/mod/slide",
                target: "Default~1",
                parser_type: ParserType::EditAnim("Jump~42"),
            }
        );
    }

    #[test]
    fn test_motion_add() {
        let parsed =
            parse("Nemesis_Engine/mod/slide/animationdatasinglefile/Default~1/slide$10.txt");
        assert_eq!(
            parsed,
            ParsedAdsfPatchPath {
                id: "Nemesis_Engine/mod/slide",
                target: "Default~1",
                parser_type: ParserType::AddMotion,
            }
        );
    }

    #[test]
    fn test_motion_replace() {
        let parsed = parse("Nemesis_Engine/mod/slide/animationdatasinglefile/Default~1/10.txt");
        assert_eq!(
            parsed,
            ParsedAdsfPatchPath {
                id: "Nemesis_Engine/mod/slide",
                target: "Default~1",
                parser_type: ParserType::EditMotion("10"),
            }
        );
    }

    #[test]
    fn test_invalid_missing_animationdatasinglefile() {
        let err = parse_adsf_path(Path::new("Nemesis_Engine/mod/slide/invalid_path/file.txt"))
            .unwrap_err();
        matches!(err, ParseError::MissingAnimationData { .. });
    }

    #[test]
    fn test_invalid_target_format() {
        let err = parse_adsf_path(Path::new(
            "Nemesis_Engine/mod/slide/animationdatasinglefile/BadTarget/file.txt",
        ))
        .unwrap_err();
        matches!(err, ParseError::SplitTilde { .. });
    }
}
