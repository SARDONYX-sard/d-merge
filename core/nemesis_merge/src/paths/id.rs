use std::path::PathBuf;

use rayon::prelude::*;
use serde_hkx::errors::readable::ReadableError;
use winnow::combinator::repeat;
use winnow::error::StrContext::*;
use winnow::error::StrContextValue::*;
use winnow::token::any;
use winnow::{
    combinator::alt,
    prelude::*,
    seq,
    token::{take_until, take_while},
};

use crate::types::PriorityMap;

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

/// Parses `"Nemesis_Engine/mod/<mod_code>"`.
/// - [Playground](https://play.rust-lang.org/?version=stable&mode=debug&edition=2024&gist=3339ad634c5d66f91e54ba8bba3bf307)
///
/// # Errors
/// If parsing fails, returns human readable error.
pub fn get_nemesis_id(input: &str) -> Result<&str, ReadableError> {
    _get_nemesis_id
        .parse(input)
        .map_err(|e| ReadableError::from_parse(e))
}

fn _get_nemesis_id<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
    let mut sep = alt(('/', '\\'))
        .context(Expected(CharLiteral('/')))
        .context(Expected(CharLiteral('\\')));

    let mut parser = seq! {
        take_until(0.., "Nemesis_Engine").context(Expected(StringLiteral("Nemesis_Engine"))),
        "Nemesis_Engine",
        sep,
        "mod".context(Expected(StringLiteral("mod"))),
        sep,
        take_while(1.., |c: char| c != '/' && c != '\\').context(Expected(Description("mod code"))),
    }
    .take();

    let id = parser.parse_next(input)?;
    repeat::<_, _, (), _, _>(0.., any).parse_next(input)?;
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
