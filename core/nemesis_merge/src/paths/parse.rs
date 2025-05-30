use std::path::Path;

use serde_hkx::errors::readable::ReadableError;
use snafu::OptionExt;
use winnow::{
    ascii::Caseless,
    combinator::{alt, repeat},
    seq,
    token::{any, take_until, take_while},
    ModalResult, Parser as _,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NemesisPath<'a> {
    pub mod_code: &'a str,
    pub template_name: &'a str,
    pub index: &'a str,
}

#[derive(Debug, snafu::Snafu, PartialEq, Eq)]
pub enum NemesisPathError {
    /// Path must be utf-8
    InvalidFormat,
    #[snafu(display("{source}"))]
    ReadableError { source: ReadableError },
}

type Result<T, E = NemesisPathError> = core::result::Result<T, E>;

pub fn parse_nemesis_path(input: &Path) -> Result<NemesisPath> {
    let input = input.to_str().context(InvalidFormatSnafu)?;

    parse_components
        .parse(input)
        .map_err(|e| NemesisPathError::ReadableError {
            source: ReadableError::from_parse(e),
        })
}

/// return `_1stperson/0_master`
fn parse_components<'a>(input: &mut &'a str) -> ModalResult<NemesisPath<'a>> {
    let first_person = seq!(
        Caseless("_1stperson"),
        alt(('/', '\\')),
        take_while(1.., |c| !matches!(c, '/' | '\\')),
    )
    .take();

    let mut template_name = alt((first_person, take_while(1.., |c| c != '/' && c != '\\')));

    // Parse prefix to Nemesis_Engine/mod/<mod_code>/
    let mut parser = seq! {
        NemesisPath {
            _: take_until(0.., "Nemesis_Engine"),
            _: "Nemesis_Engine",
            _: alt(('/', '\\')),
            _: "mod",
            _: alt(('/', '\\')),
            mod_code: take_while(1.., |c| !matches!(c, '/' | '\\')),
            _: alt(('/', '\\')),
            template_name: template_name,
            _: alt(('/', '\\')),
            index: take_while(1.., |c| !matches!(c, '.' | '/' | '\\')),
            _: repeat::<_, _, (), _, _>(0.., any),
        }
    };

    parser.parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_parse_nemesis_path(path: &str) -> NemesisPath<'_> {
        parse_nemesis_path(Path::new(path)).unwrap_or_else(|e| panic!("{e}"))
    }

    #[test]
    fn parse_nemesis_path_valid() {
        let actual =
            test_parse_nemesis_path("/some/path/to/Nemesis_Engine/mod/flinch/0_master/#0106.txt");
        assert_eq!(
            actual,
            NemesisPath {
                mod_code: "flinch",
                template_name: "0_master",
                index: "#0106",
            }
        );

        let actual = test_parse_nemesis_path("../Nemesis_Engine/mod/flinch/0_master/#0106.txt");
        assert_eq!(
            actual,
            NemesisPath {
                mod_code: "flinch",
                template_name: "0_master",
                index: "#0106",
            }
        );

        let actual = test_parse_nemesis_path(
            "/some/path/to/Nemesis_Engine/mod/flinch/_1stperson/0_master/#0106.txt",
        );
        assert_eq!(
            actual,
            NemesisPath {
                mod_code: "flinch",
                template_name: "_1stperson/0_master",
                index: "#0106",
            }
        );
    }

    #[test]
    fn parse_nemesis_path_invalid() {
        let input_path = Path::new("/invalid/path/to/Engine/mod/flinch/0_master/#0106.txt");
        assert!(parse_nemesis_path(input_path).is_err());

        let input_path = Path::new("Nemesis_Engine/mod/flinch");
        assert!(parse_nemesis_path(input_path).is_err());
    }
}
