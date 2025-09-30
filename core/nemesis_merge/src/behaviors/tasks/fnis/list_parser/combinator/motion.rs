//! Motion Data parsing: `MD <time> <dx> <dy> <dz>`

use std::borrow::Cow;

use skyrim_anim_parser::adsf::normal::Translation;
use winnow::ascii::{float, space1, Caseless};
use winnow::combinator::seq;
use winnow::error::{StrContext, StrContextValue};
use winnow::{ModalResult, Parser};

use crate::behaviors::tasks::fnis::list_parser::combinator::comment::skip_ws_and_comments;

pub fn parse_md_data<'a>(input: &mut &'a str) -> ModalResult<Translation<'a>> {
    seq!(Translation {
        _: Caseless("MD"),
        _: space1,
        time: f32_parser
            .context(StrContext::Label("Motion time"))
            .context(StrContext::Expected(StrContextValue::Description(
                "Float value (e.g. 1.5, 2.9333)"
            ))),
        _: space1,
        x: f32_parser.context(StrContext::Label("delta_x: f32")),
        _: space1,
        y: f32_parser.context(StrContext::Label("delta_y: f32")),
        _: space1,
        z: f32_parser.context(StrContext::Label("delta_z: f32")),
        _: skip_ws_and_comments,
    })
    .context(StrContext::Label("MotionData"))
    .context(StrContext::Expected(StrContextValue::Description(
        "Format: MD <time: float> <dx: int> <dy: int> <dz: int>",
    )))
    .parse_next(input)
}

fn f32_parser<'a>(input: &mut &'a str) -> ModalResult<Cow<'a, str>> {
    float::<_, f32, _>
        .take()
        .map(Cow::Borrowed)
        .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behaviors::tasks::fnis::list_parser::test_helpers::{must_fail, must_parse};

    #[test]
    fn test_parse_md_data_valid() {
        let parsed = must_parse(parse_md_data, "MD 2.5 0 0 30");
        const EXPECTED: Translation = Translation {
            time: Cow::Borrowed("2.5"),
            x: Cow::Borrowed("0"),
            y: Cow::Borrowed("0"),
            z: Cow::Borrowed("30"),
        };

        assert_eq!(parsed, EXPECTED);
    }

    #[test]
    fn test_parse_md_data_invalid() {
        must_fail(parse_md_data, "MD abc 0 0 30   \n");
    }
}
