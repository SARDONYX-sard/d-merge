//! AnimVar Parser
//!
//! This module parses lines of the form:
//!
//! ```text
//! AnimVar <Name> [ BOOL | INT32 | REAL ] <numeric_value>
//! ```
//!
//! - **Name**: Identifier of the variable.
//! - **BOOL**: Must be `0` or `1`. Parsed into `Value::Bool(false|true)`.
//! - **INT32**: Parsed as `i32`, stored as `Value::Int32`.
//! - **REAL**: Parsed as `f32`, stored as `Value::Real`.
//!
//! Example:
//!
//! ```text
//! AnimVar Enabled BOOL 1
//! AnimVar Counter INT32 42
//! AnimVar Speed REAL 2.5
//! ```
use winnow::ascii::{dec_int, float, space1, Caseless};
use winnow::combinator::{alt, seq};
use winnow::error::{StrContext, StrContextValue};
use winnow::{ModalResult, Parser};

use crate::behaviors::tasks::fnis::list_parser::combinator::comment::skip_ws_and_comments;
use crate::behaviors::tasks::fnis::list_parser::combinator::take_till_space;

/// Value stored in AnimVar
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Value {
    Bool(bool),
    Int32(i32),
    Real(f32),
}

/// Parsed AnimVar definition
#[derive(Debug, Clone, PartialEq)]
pub struct AnimVar<'a> {
    /// Variable name
    pub name: &'a str,
    /// Typed value
    pub value: Value,
}

/// Parse a single AnimVar line
///
/// Expected format:
/// ```text
/// AnimVar <Name> [ BOOL | INT32 | REAL ] <numeric_value>
/// ```
pub fn parse_anim_var_line<'a>(input: &mut &'a str) -> ModalResult<AnimVar<'a>> {
    seq! {
        AnimVar {
            _: Caseless("AnimVar"),
            _: space1,
            name: take_till_space.context(StrContext::Label("name: str")),
            _: space1,
            value: parse_value,
            _: skip_ws_and_comments,
        }
    }
    .context(StrContext::Label("AnimVar"))
    .context(StrContext::Expected(StrContextValue::Description(
        "Format: AnimVar <AnimVar> [ BOOL | INT32 | REAL ] <numeric_value>",
    )))
    .parse_next(input)
}

/// Parse the value portion: `[ BOOL | INT32 | REAL ] <value>`
fn parse_value<'a>(input: &mut &'a str) -> ModalResult<Value> {
    alt((
        move |input: &mut &'a str|{
            let (v,) = seq!{
                _: Caseless("BOOL"),
                _: space1,
                alt(("0".value(false), "1".value(true))).map(Value::Bool).context(StrContext::Label("value: 0 | 1"))
            }.parse_next(input)?;
            Ok(v)
        },
        move |input: &mut &'a str|{
            let (v,) = seq!{
                _: Caseless("INT32"),
                _: space1,
                dec_int.map(Value::Int32).context(StrContext::Label("value: i32")),
            }.parse_next(input)?;
            Ok(v)
        },
        move |input: &mut &'a str|{
            let (v,) = seq!{
                _: Caseless("REAL"),
                _: space1,
                float.map(Value::Real).context(StrContext::Label("value: f32")),
            }.parse_next(input)?;
            Ok(v)
        }
    ))
    .context(StrContext::Label("value"))
    .context(StrContext::Expected(StrContextValue::StringLiteral("BOOL")))
    .context(StrContext::Expected(StrContextValue::StringLiteral("INT32")))
    .context(StrContext::Expected(StrContextValue::StringLiteral("REAL")))
    .parse_next(input)
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
                name: "MyFlag",
                value: Value::Bool(false),
            }
        );
    }

    #[test]
    fn test_parse_bool_valid_1() {
        let parsed = must_parse(parse_anim_var_line, "AnimVar Enabled BOOL 1\n");
        assert_eq!(
            parsed,
            AnimVar {
                name: "Enabled",
                value: Value::Bool(true),
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
                name: "Counter",
                value: Value::Int32(42),
            }
        );
    }

    #[test]
    fn test_parse_real_valid() {
        let parsed = must_parse(parse_anim_var_line, "AnimVar Speed REAL 2.5\n");
        assert_eq!(
            parsed,
            AnimVar {
                name: "Speed",
                value: Value::Real(2.5),
            }
        );
    }

    #[test]
    fn test_parse_real_valid_with_spaces() {
        let parsed = must_parse(parse_anim_var_line, "AnimVar Accel   REAL   0.123   \n");
        assert_eq!(
            parsed,
            AnimVar {
                name: "Accel",
                value: Value::Real(0.123),
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
