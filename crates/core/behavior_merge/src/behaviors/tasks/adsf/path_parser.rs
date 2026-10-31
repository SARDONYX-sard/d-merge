//! Parses an adsf path and returns target and id as &str references.
//!
//! # Path rules
//!
//! ## Project names header
//!
//! ```text
//! <any>/<id>/animationdatasinglefile/$header$/$header$.txt
//! ```
//!
//! ## Animation data header
//!
//! ```text
//! <any>/<id>/animationdatasinglefile/<target>~<index>/$header$.txt
//! ```
//!
//! ## Add animation
//!
//! ```text
//! <any>/<id>/animationdatasinglefile/<target>~<index>/<name>~<anim_data_clip_id>.txt
//! ```
//!
//! Example:
//!
//! ```text
//! DefaultFemale~1/SprintSlide~slide$0.txt
//! ```
//!
//! ## Indexed animation
//!
//! ```text
//! <any>/<id>/animationdatasinglefile/<target>~<index>/<name>~<clip_id>.txt
//! ```
//!
//! Example:
//!
//! ```text
//! FirstPerson~1/TKDodgeRight~348.txt
//! ```
//!
//! An indexed animation path may represent either an edit or an addition.
//! The presence of `MOD_CODE` in the patch content determines which operation
//! is performed.
//!
//! ## Add motion
//!
//! ```text
//! <any>/<id>/animationdatasinglefile/<target>~<index>/<anim_data_clip_id>.txt
//! ```
//!
//! Example:
//!
//! ```text
//! DefaultFemale~1/slide$10.txt
//! ```
//!
//! ## Indexed motion
//!
//! ```text
//! <any>/<id>/animationdatasinglefile/<target>~<index>/<index>.txt
//! ```
//!
//! Example:
//!
//! ```text
//! DefaultFemale~1/50.txt
//! ```
//!
//! An indexed motion path may also represent either an edit or an addition.
//! The presence of `MOD_CODE` in the patch content determines which operation
//! is performed.

use std::path::{Path, PathBuf};

use crate::behaviors::priority_ids::get_nemesis_id;

/// Represents the type of parser required for a given animation patch path.
#[derive(Debug, PartialEq)]
pub(crate) enum ParserType<'a> {
    /// Indicates the special `$header$/$header$.txt` override.
    TxtProjectHeader,

    /// Indicates the special `<target>~<index>/$header$.txt` override.
    AnimHeader,

    /// Indicates an animation block whose clip ID is assigned during
    /// serialization.
    AddAnim,

    /// Indicates an animation block with an explicitly specified clip ID.
    ///
    /// The presence of `MOD_CODE` is checked later to determine whether this
    /// is an edit or an addition.
    IndexedAnim {
        /// Animation name.
        name_clip: &'a str,
    },

    /// Indicates a motion block whose clip ID is assigned during
    /// serialization.
    AddMotion,

    /// Indicates a motion block with an explicitly specified index.
    ///
    /// The presence of `MOD_CODE` is checked later to determine whether this
    /// is an edit or an addition.
    IndexedMotion {
        /// Explicitly specified motion index.
        index: &'a str,
    },
}

/// Represents the parsed result of an animation patch path.
///
/// This contains the mod ID, the animation target, and the type of data being
/// patched.
#[derive(Debug, PartialEq)]
pub(crate) struct ParsedAdsfPatchPath<'a> {
    /// Unique ID corresponding to the mod.
    ///
    /// Examples:
    ///
    /// - `slide`
    /// - `/some/Nemesis_Engine/mod/slide`
    pub id: &'a str,

    /// `project_name~index`.
    ///
    /// Example: `DefaultMale~1`.
    pub target: &'a str,

    /// Type of parser logic required.
    pub parser_type: ParserType<'a>,
}

/// Parses an ADSF (`animationdatasinglefile`) patch path.
///
/// # Errors
///
/// Returns [`ParseError`] when the path does not contain the required ADSF
/// structure or when the Nemesis mod ID cannot be extracted.
pub(crate) fn parse_adsf_path<'a>(path: &'a Path) -> Result<ParsedAdsfPatchPath<'a>, ParseError> {
    let components: Vec<&'a str> =
        path.components().filter_map(|component| component.as_os_str().to_str()).collect();

    let anim_data_index = components
        .iter()
        .position(|component| component.eq_ignore_ascii_case("animationdatasinglefile"))
        .ok_or_else(|| ParseError::MissingAnimationData { path: path.to_path_buf() })?;

    if anim_data_index < 1 || components.len() <= anim_data_index + 2 {
        return Err(ParseError::TooShortPathFormat { path: path.to_path_buf() });
    }

    let path_str =
        path.to_str().ok_or_else(|| ParseError::NonUtf8Path { path: path.to_path_buf() })?;

    let id = get_nemesis_id(path_str)?;

    let target_component = components[anim_data_index + 1];

    let target = if target_component.eq_ignore_ascii_case("$header$") {
        "$header$"
    } else if target_component.contains('~') {
        target_component
    } else {
        return Err(ParseError::SplitTilde { path: path.to_path_buf() });
    };

    let file_stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .ok_or_else(|| ParseError::TooShortPathFormat { path: path.to_path_buf() })?;

    let is_header_file = file_stem.eq_ignore_ascii_case("$header$");

    let parser_type = if target == "$header$" && is_header_file {
        ParserType::TxtProjectHeader
    } else if is_header_file {
        ParserType::AnimHeader
    } else if file_stem.contains('~') {
        let mut parts = file_stem.rsplitn(2, '~');

        let clip_id = parts.next();
        let name_clip = parts.next();

        match (name_clip, clip_id) {
            (Some(name_clip), Some(clip_id)) if !name_clip.is_empty() && !clip_id.is_empty() => {
                if clip_id.contains('$') {
                    ParserType::AddAnim
                } else {
                    ParserType::IndexedAnim { name_clip: file_stem }
                }
            }
            _ => ParserType::AddAnim,
        }
    } else if file_stem.contains('$') {
        ParserType::AddMotion
    } else if !file_stem.is_empty() {
        ParserType::IndexedMotion { index: file_stem }
    } else {
        return Err(ParseError::TooShortPathFormat { path: path.to_path_buf() });
    };

    Ok(ParsedAdsfPatchPath { target, id, parser_type })
}

/// Represents parsing errors from [`parse_adsf_path`].
#[derive(Debug, snafu::Snafu)]
#[snafu(module)]
#[allow(clippy::enum_variant_names)]
pub enum ParseError {
    #[snafu(transparent)]
    MissingID { source: winnow_ext::ReadableError },

    /// The path is not valid UTF-8.
    #[snafu(display("Non-UTF-8 path: {}", path.display()))]
    NonUtf8Path { path: PathBuf },

    /// `animationdatasinglefile` was not found in the path.
    #[snafu(display(
        "The path '{}' does not contain the required 'animationdatasinglefile' directory.\n\
         Expected a structure like: D:/mod/<id>/animationdatasinglefile/...",
        path.display()
    ))]
    MissingAnimationData { path: PathBuf },

    /// The path does not contain enough segments to extract the mod ID and
    /// target.
    #[snafu(display(
        "The path '{}' is too short to extract the mod ID and target.\n\
         Expected format: D:/mod/<id>/animationdatasinglefile/<target>~1/...",
        path.display()
    ))]
    TooShortPathFormat { path: PathBuf },

    /// The target component does not follow the expected `Target~1` format.
    #[snafu(display(
        "The target segment in path '{}' does not follow the expected '<target>~1' format. \
         Example: 'DefaultFemale~1'",
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
    fn test_anim_indexed() {
        let parsed = parse(
            "/some/mods/Nemesis_Engine/mod/slide/animationdatasinglefile/Default~1/Jump~42.txt",
        );

        assert_eq!(
            parsed,
            ParsedAdsfPatchPath {
                id: "/some/mods/Nemesis_Engine/mod/slide",
                target: "Default~1",
                parser_type: ParserType::IndexedAnim { name_clip: "Jump~42" },
            }
        );
    }

    #[test]
    fn test_tk_dodge_indexed_anim() {
        let parsed = parse(
            "/some/mods/Nemesis_Engine/mod/tkds/animationdatasinglefile/FirstPerson~1/TKDodgeRight~348.txt",
        );

        assert_eq!(
            parsed,
            ParsedAdsfPatchPath {
                id: "/some/mods/Nemesis_Engine/mod/tkds",
                target: "FirstPerson~1",
                parser_type: ParserType::IndexedAnim { name_clip: "TKDodgeRight~348" },
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
    fn test_motion_indexed() {
        let parsed = parse("Nemesis_Engine/mod/slide/animationdatasinglefile/Default~1/10.txt");

        assert_eq!(
            parsed,
            ParsedAdsfPatchPath {
                id: "Nemesis_Engine/mod/slide",
                target: "Default~1",
                parser_type: ParserType::IndexedMotion { index: "10" },
            }
        );
    }

    #[test]
    fn test_invalid_missing_animationdatasinglefile() {
        let err = parse_adsf_path(Path::new("Nemesis_Engine/mod/slide/invalid_path/file.txt"))
            .unwrap_err();

        assert!(matches!(err, ParseError::MissingAnimationData { .. }));
    }

    #[test]
    fn test_invalid_target_format() {
        let err = parse_adsf_path(Path::new(
            "Nemesis_Engine/mod/slide/animationdatasinglefile/BadTarget/file.txt",
        ))
        .unwrap_err();

        assert!(matches!(err, ParseError::SplitTilde { .. }));
    }
}
