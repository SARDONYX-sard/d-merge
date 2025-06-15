//! Utilities for extracting mod identifiers from paths pointing to `Nemesis_Engine` folders.
//!
//! This module includes functionality to extract a unique mod code path from
//! a file path, and convert multiple such paths into a priority map indexed by
//! their order in the input list. It's primarily designed to work with paths
//! from modding tools or engines like Nemesis for Skyrim SE.
//!
//! # Features
//! - Parallel extraction of mod codes from paths using Rayon
//! - Custom parsing logic with detailed error reporting using `winnow`
//! - Friendly, readable error reporting via `ReadableError`

use std::path::PathBuf;

use rayon::prelude::*;
use serde_hkx::errors::readable::ReadableError;
use winnow::ascii::Caseless;
use winnow::combinator::repeat;
use winnow::error::StrContext::*;
use winnow::error::StrContextValue::*;
use winnow::token::any;
use winnow::{combinator::alt, prelude::*, seq, token::take_while};

use crate::types::PriorityMap;

/// Converts a slice of `PathBuf`s into a [`PriorityMap`] by extracting
/// mod identifiers from each path.
///
/// The mod identifier is determined by parsing the path to locate the
/// segment that follows the structure:
/// `.../Nemesis_Engine/mod/<mod_code>/...`
///
/// Paths that do not match the expected format will be skipped.
/// # Returns
/// A `PriorityMap` where keys are the extracted mod codes and values are their index.
pub fn paths_to_priority_map(paths: &[PathBuf]) -> PriorityMap<'_> {
    paths
        .par_iter()
        .enumerate()
        .filter_map(|(index, path)| {
            get_nemesis_id(path.to_str()?)
                .map(|mod_code| (mod_code, index))
                .ok()
        })
        .collect()
}

/// Parses a string path to extract the mod ID in the format:
/// `.../Nemesis_Engine/mod/<mod_code>/...`
///
/// # Errors
///
/// Returns a [`ReadableError`] if the input string does not contain the expected
/// `Nemesis_Engine/mod/<mod_code>` pattern.
///
/// # Examples
///
/// ```txt
/// `D:\\...\\Nemesis_Engine\\mod\\abc\\somefile.txt` -> `D:\\...\\Nemesis_Engine\\mod\\abc`
/// ```
pub fn get_nemesis_id(input: &str) -> Result<&str, ReadableError> {
    _get_nemesis_id
        .parse(input)
        .map_err(|e| ReadableError::from_parse(e))
}

/// take_until implementation using only winnow
pub fn take_until_ext<Input, Output, Error, ParseNext>(
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

fn _get_nemesis_id<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
    // Match either '/' or '\\' as path separator
    let mut sep = alt(('/', '\\'))
        .context(Expected(CharLiteral('/')))
        .context(Expected(CharLiteral('\\')));

    // Build parser for: <any>* Nemesis_Engine/mod/<mod_code>
    let mut parser = seq! {
        take_until_ext(0.., Caseless("Nemesis_Engine")).context(Expected(StringLiteral("Nemesis_Engine"))),
        Caseless("Nemesis_Engine"),
        sep,
        "mod".context(Expected(StringLiteral("mod"))),
        sep,
        take_while(1.., |c: char| c != '/' && c != '\\').context(Expected(Description("mod code"))),
    }
    .take();

    let id = parser.parse_next(input)?;
    repeat::<_, _, (), _, _>(0.., any).parse_next(input)?; // consume remaining input
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_nemesis_id() {
        let input = r"D:\GAME\ModOrganizer Skyrim SE\mods\SomeMod\Nemesis_Engine\mod\abc\0_master\#0001.txt";
        let id = _get_nemesis_id
            .parse(input)
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            id,
            r"D:\GAME\ModOrganizer Skyrim SE\mods\SomeMod\Nemesis_Engine\mod\abc"
        );
    }

    #[test]
    fn test_invalid_path() {
        let input = r"D:\Invalid\Path\To\Something";
        assert!(get_nemesis_id(input).is_err());
    }
}
