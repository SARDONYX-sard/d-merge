//! Parses an adsf path and returns target and id as &str references.
//!
//! rule:
//! anim block header:
//! - format: <any>/<id>/animationdatasinglefile/$header$/$header$.txt
//!   (e.g. D:/mod/slide/animationdatasinglefile/$header$/$header$.txt)
//!
//! add anim block path:
//! - format: <any>/<id>/animationdatasinglefile/<target>~1/<name>~<anim_data_clip_id>.txt
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
use std::path::{Path, PathBuf};

use crate::path_id::get_nemesis_id;

// TODO: Support replace operation

/// Represents the type of parser required for a given animation patch path.
#[derive(Debug, PartialEq)]
pub enum ParserType {
    /// Indicates an individual animation (e.g., `Run~slide.txt`)
    Anim,
    /// Indicates a motion ID replacement or override (e.g., `10.txt`, `slide$10.txt`)
    Motion,
    /// Indicates the special `$header$/$header$.txt` override
    AnimHeader,
}

/// Represents the type of action (add or replace) for a given patch.
#[derive(Debug, PartialEq)]
pub enum Op {
    /// Add a new animation or motion entry
    Add,
    /// Replace an existing entry
    Replace,
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
    pub parser_type: ParserType,
    /// Indicates whether the patch `adds` or `replaces` data
    pub op: Op,
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

    let file_name = components
        .last()
        .ok_or_else(|| ParseError::TooShortPathFormat {
            path: path.to_path_buf(),
        })?;

    let parser_type = if file_name.eq_ignore_ascii_case("$header$.txt") {
        ParserType::AnimHeader
    } else if file_name.contains('~') {
        ParserType::Anim
    } else {
        ParserType::Motion
    };

    let action_type = if file_name.contains('$') || matches!(parser_type, ParserType::AnimHeader) {
        Op::Add
    } else {
        Op::Replace
    };

    Ok(ParsedAdsfPatchPath {
        target,
        id,
        parser_type,
        op: action_type,
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(p: &str) -> ParsedAdsfPatchPath<'_> {
        parse_adsf_path(Path::new(p)).expect("should parse correctly")
    }

    #[test]
    fn test_anim_header_add() {
        let parsed = parse(
            "/some/mods/Nemesis_Engine/mod/slide/animationdatasinglefile/$header$/$header$.txt",
        );
        assert_eq!(
            parsed,
            ParsedAdsfPatchPath {
                id: "/some/mods/Nemesis_Engine/mod/slide",
                target: "$header$",
                parser_type: ParserType::AnimHeader,
                op: Op::Add,
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
                parser_type: ParserType::Anim,
                op: Op::Add,
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
                parser_type: ParserType::Anim,
                op: Op::Replace,
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
                parser_type: ParserType::Motion,
                op: Op::Add,
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
                parser_type: ParserType::Motion,
                op: Op::Replace,
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
