//! -     Behavior Variable: AnimVar <AnimVar> [ BOOL | INT32 | REAL ] <numeric_value>

use winnow::ascii::{float, line_ending, space0, space1, Caseless};
use winnow::combinator::{alt, seq};
use winnow::error::{StrContext, StrContextValue};
use winnow::token::take_till;
use winnow::{ModalResult, Parser};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ValueType {
    Bool,
    Int32,
    Real,
}

#[derive(Debug, PartialEq)]
pub struct AnimVar<'a> {
    pub anim_var: &'a str,
    pub value_type: ValueType,
    pub default_value: f32,
}

pub fn parse_anim_var_line<'a>(input: &mut &'a str) -> ModalResult<AnimVar<'a>> {
    seq! {
        AnimVar {
            _: Caseless("AnimVar"),
            _: space1,
            anim_var: take_till(1.., [' ' , '\t']).context(StrContext::Label("Anim var name: str")),
            _: space1,
            value_type: alt((
                Caseless("BOOL").value(ValueType::Bool),
                Caseless("INT32").value(ValueType::Int32),
                Caseless("REAL").value(ValueType::Real)
            ))
            .context(StrContext::Label("value_type"))
            .context(StrContext::Expected(StrContextValue::StringLiteral("BOOL")))
            .context(StrContext::Expected(StrContextValue::StringLiteral("INT32")))
            .context(StrContext::Expected(StrContextValue::StringLiteral("REAL"))) ,
            _: space1,
            default_value: parse_default_value(value_type),
            _: space0,
            _: line_ending,
        }
    }
    .context(StrContext::Label("AnimVar"))
    .context(StrContext::Expected(StrContextValue::Description(
        "Format: AnimVar <AnimVar> [ BOOL | INT32 | REAL ] <numeric_value>",
    )))
    .parse_next(input)
}

fn parse_default_value<'a>(value_type: ValueType) -> impl FnMut(&mut &'a str) -> ModalResult<f32> {
    move |input: &mut &'a str| {
        Ok(match value_type {
            ValueType::Bool => float
                .verify(|n: &f32| matches!(*n, 0.0 | 1.0))
                .context(StrContext::Label("default_value: 0 | 1"))
                .parse_next(input)?,
            ValueType::Int32 => float
                .context(StrContext::Label("default_value: i32"))
                .parse_next(input)?,
            ValueType::Real => float
                .context(StrContext::Label("default_value: f32"))
                .parse_next(input)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behaviors::tasks::fnis::list_parser::test_helpers::{must_fail, must_parse};

    #[test]
    fn test_parse_bool_valid_0() {
        let parsed = must_parse(parse_anim_var_line, "AnimVar MyFlag BOOL 0\n");
        assert_eq!(
            parsed,
            AnimVar {
                anim_var: "MyFlag",
                value_type: ValueType::Bool,
                default_value: 0.0,
            }
        );
    }

    #[test]
    fn test_parse_bool_valid_1() {
        let parsed = must_parse(parse_anim_var_line, "AnimVar Enabled BOOL 1\n");
        assert_eq!(
            parsed,
            AnimVar {
                anim_var: "Enabled",
                value_type: ValueType::Bool,
                default_value: 1.0,
            }
        );
    }

    #[test]
    fn test_parse_bool_invalid_other_number() {
        must_fail(parse_anim_var_line, "AnimVar Something BOOL 2\n");
        must_fail(parse_anim_var_line, "AnimVar Something BOOL -1\n");
        must_fail(parse_anim_var_line, "AnimVar Something BOOL 0.5\n");
    }

    #[test]
    fn test_parse_int32_valid() {
        let parsed = must_parse(parse_anim_var_line, "AnimVar Counter INT32 42\n");
        assert_eq!(
            parsed,
            AnimVar {
                anim_var: "Counter",
                value_type: ValueType::Int32,
                default_value: 42.0,
            }
        );
    }

    #[test]
    fn test_parse_real_valid() {
        let parsed = must_parse(parse_anim_var_line, "AnimVar Speed REAL 2.5\n");
        assert_eq!(
            parsed,
            AnimVar {
                anim_var: "Speed",
                value_type: ValueType::Real,
                default_value: 2.5,
            }
        );
    }

    #[test]
    fn test_parse_real_valid_with_spaces() {
        let parsed = must_parse(parse_anim_var_line, "AnimVar Accel   REAL   0.123   \n");
        assert_eq!(
            parsed,
            AnimVar {
                anim_var: "Accel",
                value_type: ValueType::Real,
                default_value: 0.123,
            }
        );
    }

    #[test]
    fn test_parse_invalid_keyword() {
        must_fail(parse_anim_var_line, "AniVar ValueName BOOL 1\n");
    }

    #[test]
    fn test_parse_invalid_value_type() {
        must_fail(parse_anim_var_line, "AnimVar Foo STRING 1\n");
    }
}
