//! Nemesis patch path specification and parser.
//!
//! This module defines a structured representation of Nemesis patch paths.
//!
//! ## Supported layouts
//!
//! ### Normal
//! ```text
//! Nemesis_Engine/mod/<mod_code>/(optional _1stperson/)<template_name>/#<file>.txt
//! ```
//! - Targets a specific Nemesis template (e.g. `0_master`).
//! - `_1stperson` is a semantic prefix handled only in this mode.
//!
//! ### EngineExt
//! ```text
//! Nemesis_EngineExt/mod/<mod_code>/meshes/<any path>/#<file>.txt
//! ```
//! - Everything under `meshes/` is considered patchable.
//! - There is no concept of template name or first-person flag.
//! - The patch applies freely to arbitrary hkx paths.
//!
//! The parser guarantees which data is available by encoding the difference
//! in the `NemesisPath` enum variants.

use std::path::Path;

use serde_hkx::errors::readable::ReadableError;
use snafu::OptionExt;
use winnow::{
    ascii::Caseless,
    combinator::{alt, opt, repeat},
    error::{StrContext, StrContextValue},
    seq,
    token::{any, take_while},
    ModalResult, Parser,
};

use crate::behaviors::priority_ids::take_until_ext;
use crate::behaviors::tasks::patches::paths::NemesisPath;
use crate::errors::{Error, NonUtf8PathSnafu};

/// Parse nemesis patch path and return structured information.
pub fn parse_nemesis_path(path: &Path) -> Result<NemesisPath<'_>, Error> {
    let input = path.to_str().with_context(|| NonUtf8PathSnafu { path })?;

    parse_components
        .parse(input)
        .map_err(|e| Error::FailedParseNemesisPatchPath {
            source: ReadableError::from_parse(e),
        })
}

fn parse_components<'a>(input: &mut &'a str) -> ModalResult<NemesisPath<'a>> {
    alt((parse_ext, parse_normal)).parse_next(input)
}

/// `/` or `\`
fn path_separator(input: &mut &str) -> ModalResult<()> {
    alt(('/', '\\')).parse_next(input)?;
    Ok(())
}

/// Parse `Nemesis_Engine/mod/<mod_code>/(optional _1stperson/)<template_name>/#<file>.txt`
fn parse_normal<'a>(input: &mut &'a str) -> ModalResult<NemesisPath<'a>> {
    seq! {
        NemesisPath::Normal {
            _: take_until_ext(0.., Caseless("Nemesis_Engine")),
            _: Caseless("Nemesis_Engine").context(StrContext::Expected(StrContextValue::StringLiteral("Nemesis_Engine"))),
            _: path_separator,
            _: Caseless("mod").context(StrContext::Expected(StrContextValue::StringLiteral("mod"))),
            _: path_separator,
            _: take_while(1.., |c| !matches!(c, '/' | '\\')), // <mod_code>
            _: path_separator,

            is_1st_person: opt((Caseless("_1stperson"), path_separator)).map(|s| s.is_some()), // `_1stperson/`
            template_name: take_while(1.., |c| c != '/' && c != '\\'), // e.g., `0_master`

            _: repeat::<_, _, (), _, _>(0.., any),
        }
    }
    .parse_next(input)
}

/// `Nemesis_EngineExt/mod/<mod_code>/meshes/<any path>/#<file>.txt`
fn parse_ext<'a>(input: &mut &'a str) -> ModalResult<NemesisPath<'a>> {
    seq! {
        NemesisPath::EngineExt {
            _: take_until_ext(0.., Caseless("Nemesis_EngineExt")),
            _: Caseless("Nemesis_EngineExt").context(StrContext::Expected(StrContextValue::StringLiteral("Nemesis_EngineExt"))),
            _: path_separator,
            _: Caseless("mod").context(StrContext::Expected(StrContextValue::StringLiteral("mod"))),
            _: path_separator,
            _: take_while(1.., |c| !matches!(c, '/' | '\\')), // <mod_code>
            _: path_separator,

            // capture `meshes/...` but stop before `/#` or `\#`
            meshes_path: seq!(
                "meshes",
                alt(('/', '\\')),
                take_until_ext(0.., alt(("/#", "\\#"))),
            ).take().context(StrContext::Expected(StrContextValue::Description("starts_with(\"meshes\")"))),

            _: alt(("/#", "\\#")),
            _: repeat::<_, _, (), _, _>(0.., any),
        }
    }
    .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn parse(path: &str) -> NemesisPath<'_> {
        parse_nemesis_path(Path::new(path)).unwrap_or_else(|e| panic!("{e}"))
    }

    #[test]
    fn normal_basic() {
        let p = parse("Nemesis_Engine/mod/flinch/0_master/#0106.txt");
        assert_eq!(
            p,
            NemesisPath::Normal {
                template_name: "0_master",
                is_1st_person: false
            }
        );
    }

    #[test]
    fn normal_1stperson() {
        let p = parse("Nemesis_Engine/mod/flinch/_1stperson/0_master/#0106.txt");
        assert_eq!(
            p,
            NemesisPath::Normal {
                template_name: "0_master",
                is_1st_person: true
            }
        );
    }

    #[test]
    fn engine_ext_basic() {
        let p = parse("Nemesis_EngineExt/mod/test/meshes/actors/character/behaviors/character assets/skeleton.bin/#test.txt");
        assert_eq!(
            p,
            NemesisPath::EngineExt {
                meshes_path: "meshes/actors/character/behaviors/character assets/skeleton.bin"
            }
        );
    }

    #[test]
    fn invalid_missing_engine() {
        let p = Path::new("Engine/mod/a/0_master");
        assert!(parse_nemesis_path(p).is_err());
    }
}
