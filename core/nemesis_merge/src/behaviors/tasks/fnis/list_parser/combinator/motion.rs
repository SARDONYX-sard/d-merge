//! Motion Data parsing: `MD <time> <dx> <dy> <dz>`

use winnow::ascii::{dec_int, float, space1, Caseless};
use winnow::combinator::seq;
use winnow::error::{StrContext, StrContextValue};
use winnow::{ModalResult, Parser};

use crate::behaviors::tasks::fnis::list_parser::combinator::comment::take_till_line_or_eof;

#[derive(Debug, PartialEq)]
pub struct MotionData {
    pub time: f32,
    pub delta_x: i64,
    pub delta_y: i64,
    pub delta_z: i64,
}

pub fn parse_md_data(input: &mut &str) -> ModalResult<MotionData> {
    seq!(MotionData {
        _: Caseless("MD"),
        _: space1,
        time: float
            .context(StrContext::Label("Motion time"))
            .context(StrContext::Expected(StrContextValue::Description(
                "Float value (e.g. 1.5, 2.9333)"
            ))),
        _: space1,
        delta_x: dec_int.context(StrContext::Label("delta_x")),
        _: space1,
        delta_y: dec_int.context(StrContext::Label("delta_y")),
        _: space1,
        delta_z: dec_int.context(StrContext::Label("delta_z")),
        _: take_till_line_or_eof,
    })
    .context(StrContext::Label("MotionData"))
    .context(StrContext::Expected(StrContextValue::Description(
        "Format: MD <time: float> <dx: int> <dy: int> <dz: int>",
    )))
    .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behaviors::tasks::fnis::list_parser::test_helpers::{must_fail, must_parse};

    #[test]
    fn test_parse_md_data_valid() {
        let parsed = must_parse(parse_md_data, "MD 2.5 0 0 30");
        assert_eq!(parsed.time.to_le_bytes(), 2.5_f32.to_le_bytes());
        assert_eq!(parsed.delta_z, 30);
    }

    #[test]
    fn test_parse_md_data_invalid() {
        must_fail(parse_md_data, "MD abc 0 0 30   \n");
    }
}
