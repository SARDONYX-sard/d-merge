use std::path::Path;

use serde_hkx::errors::readable::ReadableError;
use snafu::OptionExt;
use winnow::{
    ascii::Caseless,
    combinator::{alt, opt, repeat},
    seq,
    token::{any, take_while},
    ModalResult, Parser,
};

use crate::{
    behaviors::priority_ids::take_until_ext,
    errors::{Error, NonUtf8PathSnafu},
};

/// Parse nemesis patch path.
pub fn parse_nemesis_path(path: &Path) -> Result<(&str, bool), Error> {
    let input = path.to_str().with_context(|| NonUtf8PathSnafu { path })?;

    parse_components
        .parse(input)
        .map_err(|e| Error::FailedParseNemesisPatchPath {
            source: ReadableError::from_parse(e),
        })
}

/// return `_1stperson/0_master`
fn parse_components<'a>(input: &mut &'a str) -> ModalResult<(&'a str, bool)> {
    let mut template_name = seq! {
        opt(seq!(Caseless("_1stperson"), alt(('/', '\\'))).take()),
        take_while(1.., |c| c != '/' && c != '\\'),
    };

    // Parse prefix to Nemesis_Engine/mod/<mod_code>/
    let mut parser = seq! {
            _: take_until_ext(0.., Caseless("Nemesis_Engine")),
            _: Caseless("Nemesis_Engine"),
            _: alt(('/', '\\')),
            _: "mod",
            _: alt(('/', '\\')),
            _: take_while(1.., |c| !matches!(c, '/' | '\\')), // mod_code e.g. slide
            _: alt(('/', '\\')),
            template_name,
            _: repeat::<_, _, (), _, _>(0.., any),
    };

    let ((is_1st_person, template_name),) = parser.parse_next(input)?;
    Ok((template_name, is_1st_person.is_some()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn test_parse_nemesis_path(path: &str) -> (&str, bool) {
        parse_nemesis_path(Path::new(path)).unwrap_or_else(|e| panic!("{e}"))
    }

    #[test]
    fn parse_nemesis_path_valid_basic() {
        let actual =
            test_parse_nemesis_path("/some/path/to/Nemesis_engine/mod/flinch/0_master/#0106.txt");
        assert_eq!(actual, ("0_master", false));
    }

    #[test]
    fn parse_nemesis_path_valid_relative_path() {
        let actual = test_parse_nemesis_path("../Nemesis_Engine/mod/flinch/0_master/#0106.txt");
        assert_eq!(actual, ("0_master", false));
    }

    #[test]
    fn parse_nemesis_path_valid_nested_template_name() {
        let actual = test_parse_nemesis_path(
            "/some/path/to/Nemesis_Engine/mod/flinch/_1stperson/0_master/#0106.txt",
        );
        assert_eq!(actual, ("0_master", true));
    }

    #[test]
    fn parse_nemesis_path_invalid_wrong_path() {
        // Missing `Nemesis_Engine`
        let input_path = Path::new("/invalid/path/to/Engine/mod/flinch/0_master/#0106.txt");
        assert!(parse_nemesis_path(input_path).is_err());
    }

    #[test]
    fn parse_nemesis_path_invalid_too_short() {
        let input_path = Path::new("Nemesis_Engine/mod/flinch");
        assert!(parse_nemesis_path(input_path).is_err());
    }
}
