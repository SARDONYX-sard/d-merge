use snafu::Snafu;
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq)]
pub struct ParsedAdsfPatchPath<'a> {
    pub target: &'a str,
    pub id: &'a str,
    pub parser_type: ParserType,
}

#[derive(Debug, PartialEq)]
pub enum ParserType {
    Anim,
    Motion,
}

/// Parses an adsf path and returns target and id as &str references.
///
// rule:
// anim path:
// - format: <any>/<id>/animationdatasinglefile/<target>~1/<anim_data_clip_id>.txt
//   (e.g. D:/mod\slide\animationdatasinglefile\DefaultFemale~1\slide$0.txt)
//
// motion path:
// - format: <any>/<id>/animationdatasinglefile/<target>~1/<name>~<anim_data_clip_id>.txt
//   (e.g. D:\mod\slide\animationdatasinglefile\DefaultFemale~1\SprintSlide~slide$0.txt)
/// Parses an adsf path and returns target, id, and parser type.
pub fn parse_adsf_path<'a>(path: &'a Path) -> Result<ParsedAdsfPatchPath<'a>, ParseError> {
    let components: Vec<&'a str> = path
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();

    let anim_data_index = components
        .iter()
        .position(|comp| comp.eq_ignore_ascii_case("animationdatasinglefile"))
        .ok_or_else(|| ParseError::InvalidPathFormatMissingAnimationData {
            path: path.to_path_buf(),
        })?;

    if anim_data_index < 1 || components.len() <= anim_data_index + 2 {
        return Err(ParseError::InvalidPathFormatTooShort {
            path: path.to_path_buf(),
        });
    }

    let id = components[anim_data_index - 1];
    let target_component = components[anim_data_index + 1];

    let (target, _) =
        target_component
            .split_once('~')
            .ok_or_else(|| ParseError::InvalidTargetFormat {
                path: path.to_path_buf(),
            })?;

    let file_name = components
        .last()
        .ok_or_else(|| ParseError::InvalidPathFormatTooShort {
            path: path.to_path_buf(),
        })?;

    let parser_type = if file_name.contains('~') {
        ParserType::Motion
    } else {
        ParserType::Anim
    };

    Ok(ParsedAdsfPatchPath {
        id,
        target,
        parser_type,
    })
}

#[derive(Debug, Snafu)]
#[snafu(module)]
#[allow(clippy::enum_variant_names)]
pub enum ParseError {
    #[snafu(display(
        "InvalidPathFormat: '{}' does not contain 'animationdatasinglefile'",
        path.display()
    ))]
    InvalidPathFormatMissingAnimationData { path: PathBuf },

    #[snafu(display(
        "InvalidPathFormat: '{}' does not have enough components to extract id and target",
        path.display()
    ))]
    InvalidPathFormatTooShort { path: PathBuf },

    #[snafu(display("InvalidPathFormat: '{}' failed to split target~1", path.display()))]
    InvalidTargetFormat { path: PathBuf },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_adsf_path() {
        let path = r"/Users/Steam/Skyrim SE/MO2/mods/Dodge MCO-DXP/Nemesis_Engine/mod/dmco/animationdatasinglefile/DefaultFemale~1/dmco$1.txt";
        let result = parse_adsf_path(Path::new(path));
        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert_eq!(parsed.target, "DefaultFemale");
        assert_eq!(parsed.id, "dmco");
    }

    #[test]
    fn test_anim_path() {
        let path = r"/Users/Steam/Skyrim SE/MO2/mods/Dodge/Nemesis_Engine/mod/dmco/animationdatasinglefile/DefaultFemale~1/dmco$1.txt";
        let parsed = parse_adsf_path(Path::new(path)).unwrap();
        assert_eq!(
            parsed,
            ParsedAdsfPatchPath {
                target: "DefaultFemale",
                id: "dmco",
                parser_type: ParserType::Anim
            }
        );
    }

    #[test]
    fn test_motion_path() {
        let path = Path::new(
            r"/Users/Steam/Skyrim SE/MO2/mods/Dodge/Nemesis_Engine/mod/dmco/animationdatasinglefile/DefaultFemale~1/MCO_ClipGenerator_Dodge_B_Dodge1~dmco$11.txt",
        );
        let parsed = parse_adsf_path(path).unwrap();
        assert_eq!(parsed.target, "DefaultFemale");
        assert_eq!(parsed.id, "dmco");
        assert_eq!(parsed.parser_type, ParserType::Motion);
    }
}
