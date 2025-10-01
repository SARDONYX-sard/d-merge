//! FNIS namespace parser

use serde_hkx::errors::readable::ReadableError;
use winnow::{
    ascii::Caseless,
    combinator::alt,
    error::{StrContext, StrContextValue},
    seq,
    token::take_while,
    ModalResult, Parser,
};

use crate::behaviors::{priority_ids::take_until_ext, tasks::fnis::collect::owned::FnisError};

/// Parse FNIS path to extract mod_code (directory after `meshes/character/animations`)
///
/// # Note
/// Must be unique name
pub fn get_fnis_namespace(input: &str) -> Result<&str, FnisError> {
    parse_fnis_namespace
        .parse(input)
        .map_err(|e| FnisError::FailedToGetFnisNamespace {
            source: ReadableError::from_parse(e),
        })
}

/// Find `animations` then grab the next path component
pub fn parse_fnis_namespace<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
    let (namespace,) = seq! {
        _: take_until_ext(0.., Caseless("animations")).context(StrContext::Expected(StrContextValue::Description("animations"))),
        _: Caseless("animations").context(StrContext::Expected(StrContextValue::Description("animations"))),
        _: alt(('/', '\\')).context(StrContext::Expected(StrContextValue::Description("path separator: /"))),
        take_until_ext(1.., alt(('/' ,'\\'))),
        _: take_while(0.., |_| true), // skip else
    }
    .parse_next(input)?;

    Ok(namespace)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_parse_fnis(path: &str) -> &str {
        get_fnis_namespace(path).unwrap_or_else(|e| panic!("{e}"))
    }

    #[test]
    fn parse_fnis_basic() {
        let actual = test_parse_fnis(
            r"D:\Programming\rust\d-merge\dummy\fnis_test_mods\FNIS Flyer SE 7.0\Data\Meshes\actors\character\animations\FNISFlyer\FNISfl_Back_ac.hkx",
        );
        assert_eq!(actual, "FNISFlyer");
    }

    #[test]
    fn parse_fnis_unix_path() {
        let actual = test_parse_fnis(
            "/some/path/Meshes/actors/character/animations/FNISFlyer/FNISfl_Back_ac.hkx",
        );
        assert_eq!(actual, "FNISFlyer");
    }

    #[test]
    fn parse_fnis_invalid() {
        // Missing "animations"
        let input_path = "/Meshes/actors/character/behaviors/FNIS_FNISFlyer_Behavior.hkx";
        assert!(get_fnis_namespace(input_path).is_err());
    }
}
