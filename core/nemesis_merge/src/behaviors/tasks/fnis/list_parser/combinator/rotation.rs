//! Rotation Data parsing: `RD ...`
//!
//! FNIS rotation data can be in two formats. This module parses the line into a structured format.
//!
//! Format:
//! - `RD <time: f32> <quat_1: f32> <quat_2: f32> <quat_3: f32> <quat_4: f32>`
//! - `RD <time: f32> <delta_z_angle: f32>`

use winnow::ascii::{float, line_ending, space0, space1, Caseless};
use winnow::combinator::{alt, seq};
use winnow::error::{StrContext, StrContextValue};
use winnow::{ModalResult, Parser};

/// Rotation data for an animation with a common `time` and specific format data.
#[derive(Debug, PartialEq)]
pub struct RotationData {
    /// Time at which this rotation occurs
    pub time: f32,
    /// Rotation format (quaternion or delta z)
    pub format: RotationFormat,
}

/// The type of rotation following the common `time`
#[derive(Debug, PartialEq)]
pub enum RotationFormat {
    /// Quaternion-based rotation
    Quaternion {
        quat_1: f32,
        quat_2: f32,
        quat_3: f32,
        quat_4: f32,
    },
    /// Single-axis Z rotation
    DeltaZAngle { delta_z_angle: f32 },
}

/// Parse `RD ...` line into a structured `RotationData`
/// - `RD <time: f32> <quat_1: f32> <quat_2: f32> <quat_3: f32> <quat_4: f32>`
/// - `RD <time: f32> <delta_z_angle: f32>`
pub fn parse_rd_data(input: &mut &str) -> ModalResult<RotationData> {
    seq! {
        _: Caseless("RD"),
        _: space1,
        float.context(StrContext::Label("Rotation time")),
        _: space1,
        alt((parse_rd_data1, parse_rd_data2)),
        _: space0,
        _: line_ending,
    }
    .map(|(time, format)| RotationData { time, format })
    .context(StrContext::Label("Rotation"))
    .context(StrContext::Expected(StrContextValue::Description(
        "Format: `RD <time: f32> <quat_1: f32> <quat_2: f32> <quat_3: f32> <quat_4: f32>` or `RD <time: f32> <delta_z_angle: f32>`",
    )))
    .parse_next(input)
}

/// Parse quaternion-based rotation: RD <time> <quat_1> <quat_2> <quat_3> <quat_4>
fn parse_rd_data1(input: &mut &str) -> ModalResult<RotationFormat> {
    seq! {
        RotationFormat::Quaternion {
            quat_1: float,
            _: space1,
            quat_2: float,
            _: space1,
            quat_3: float,
            _: space1,
            quat_4: float,
        }
    }
    .parse_next(input)
}

/// Parse single-axis Z rotation: RD <time> <delta_z_angle>
fn parse_rd_data2(input: &mut &str) -> ModalResult<RotationFormat> {
    seq! {
        RotationFormat::DeltaZAngle {
            delta_z_angle: float,
        }
    }
    .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behaviors::tasks::fnis::list_parser::test_helpers::{must_fail, must_parse};

    #[test]
    fn parse_format1_rotation() {
        let input = "RD 1.5 0.0 0.0 0.0 1.0\n";
        let expected = RotationData {
            time: 1.5,
            format: RotationFormat::Quaternion {
                quat_1: 0.0,
                quat_2: 0.0,
                quat_3: 0.0,
                quat_4: 1.0,
            },
        };
        let parsed = must_parse(parse_rd_data, input);
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_format2_rotation() {
        let input = "RD 2.0 90\n";
        let expected = RotationData {
            time: 2.0,
            format: RotationFormat::DeltaZAngle {
                delta_z_angle: 90.0,
            },
        };
        let parsed = must_parse(parse_rd_data, input);
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_incomplete_rotation_fails() {
        let input = "RD 3.0\n";
        must_fail(parse_rd_data, input);
    }

    #[test]
    fn parse_multiple_rotations() {
        let input1 = "RD 1.5 0.0 0.0 0.0 1.0\n";
        let expected1 = RotationData {
            time: 1.5,
            format: RotationFormat::Quaternion {
                quat_1: 0.0,
                quat_2: 0.0,
                quat_3: 0.0,
                quat_4: 1.0,
            },
        };
        let parsed1 = must_parse(parse_rd_data, input1);
        assert_eq!(parsed1, expected1);

        let input2 = "RD 2.0 45\n";
        let expected2 = RotationData {
            time: 2.0,
            format: RotationFormat::DeltaZAngle {
                delta_z_angle: 45.0,
            },
        };
        let parsed2 = must_parse(parse_rd_data, input2);
        assert_eq!(parsed2, expected2);
    }
}
