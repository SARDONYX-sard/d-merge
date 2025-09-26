//! Animation flags parsing: simple flags and parameterized flags

use winnow::ascii::{alphanumeric1, float, space0};
use winnow::combinator::{alt, opt, preceded, seq};
use winnow::error::{StrContext, StrContextValue};
use winnow::{ModalResult, Parser};

// fn parse_anim_flag_param<'a>(input: &mut &'a str) -> ModalResult<FNISAnimFlagParam<'a>> {
//     seq! {
//         _: "s",

//     }
//     .parse_next(input)
// }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behaviors::tasks::fnis::list_parser::test_helpers::{must_fail, must_parse};

    // #[test]
    // fn parse_single_simple_flag() {
    //     assert_eq!(
    //         must_parse(parse_anim_flag_simple, "a"),
    //         FNISAnimFlags::Acyclic
    //     );
    // }
}
