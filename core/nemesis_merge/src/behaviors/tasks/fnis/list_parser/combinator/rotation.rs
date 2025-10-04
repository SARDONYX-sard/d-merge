//! Rotation Data parsing: `RD ...`
//!
//! FNIS rotation data can be in two formats. This module parses the line into a structured format.
//!
//! Format:
//! - `RD <time: f32> <quat_1: f32> <quat_2: f32> <quat_3: f32> <quat_4: f32>`
//! - `RD <time: f32> <delta_z_angle: f32>`

use skyrim_anim_parser::adsf::normal::Rotation;
use winnow::ascii::{float, space1, Caseless};
use winnow::combinator::{alt, seq};
use winnow::error::{StrContext, StrContextValue};
use winnow::{ModalResult, Parser};

use crate::behaviors::tasks::fnis::list_parser::combinator::comment::skip_ws_and_comments;

/// Rotation data for an animation with a common `time` and specific format data.
#[derive(Debug, PartialEq)]
pub struct RotationData<'a> {
    /// Time at which this rotation occurs
    pub time: &'a str,
    /// Rotation format (quaternion or delta z)
    pub format: RotationFormat<'a>,
}

impl<'a> RotationData<'a> {
    pub fn into_rotation(self) -> Rotation<'a> {
        use std::borrow::Cow;

        let Self { time, format } = self;
        let time = Cow::Borrowed(time);

        match format {
            RotationFormat::Quaternion {
                quat_x,
                quat_y,
                quat_z,
                quat_w,
            } => Rotation {
                time,
                x: Cow::Borrowed(quat_x),
                y: Cow::Borrowed(quat_y),
                z: Cow::Borrowed(quat_z),
                w: Cow::Borrowed(quat_w),
            },
            RotationFormat::DeltaZAngle { delta_z_angle } => {
                // Heap alloc optimization
                if delta_z_angle == 0.0 {
                    return Rotation {
                        time,
                        x: Cow::Borrowed("0"),
                        y: Cow::Borrowed("0"),
                        z: Cow::Borrowed("0.000000"),
                        w: Cow::Borrowed("1.000000"),
                    };
                }

                let (x, y, z, w) = quat_from_z(delta_z_angle);
                Rotation {
                    time,
                    x: Cow::Owned(x.to_string()),
                    y: Cow::Owned(y.to_string()),
                    z: Cow::Owned(z.to_string()),
                    w: Cow::Owned(w.to_string()),
                }
            }
        }
    }
}

/// Convert Z-axis rotation in degrees to a quaternion.
/// Returns (x, y, z, w).
fn quat_from_z(deg: f32) -> (f32, f32, f32, f32) {
    let theta = deg.to_radians();
    let half = theta * 0.5;
    (0.0, 0.0, half.sin(), half.cos())
}

/// The type of rotation following the common `time`
#[derive(Debug, PartialEq)]
pub enum RotationFormat<'a> {
    /// Quaternion-based rotation
    Quaternion {
        quat_x: &'a str,
        quat_y: &'a str,
        quat_z: &'a str,
        quat_w: &'a str,
    },
    /// Single-axis Z rotation
    DeltaZAngle { delta_z_angle: f32 },
}

/// Parse `RD ...` line into a structured `RotationData`
/// - `RD <time: f32> <quat_1: f32> <quat_2: f32> <quat_3: f32> <quat_4: f32>`
/// - `RD <time: f32> <delta_z_angle: f32>`
pub fn parse_rd_data<'a>(input: &mut &'a str) -> ModalResult<RotationData<'a>> {
    seq! {
        _: Caseless("RD"),
        _: space1,
        float::<_, f32, _>.take().context(StrContext::Label("Rotation time")),
        _: space1,
        alt((parse_rd_data1, parse_rd_data2)),
        _: skip_ws_and_comments,
    }
    .map(|(time, format)| RotationData { time, format })
    .context(StrContext::Label("Rotation"))
    .context(StrContext::Expected(StrContextValue::Description(
        "Format: `RD <time: f32> <quat_x: f32> <quat_y: f32> <quat_z: f32> <quat_w: f32>` or `RD <time: f32> <delta_z_angle: f32>`",
    )))
    .parse_next(input)
}

/// Parse quaternion-based rotation: RD <time> <quat_x> <quat_y> <quat_z> <quat_w>
fn parse_rd_data1<'a>(input: &mut &'a str) -> ModalResult<RotationFormat<'a>> {
    let mut f32_parser = float::<_, f32, _>.take();

    seq! {
        RotationFormat::Quaternion {
            quat_x: f32_parser,
            _: space1,
            quat_y: f32_parser,
            _: space1,
            quat_z: f32_parser,
            _: space1,
            quat_w: f32_parser,
        }
    }
    .parse_next(input)
}

/// Parse single-axis Z rotation: RD <time> <delta_z_angle>
fn parse_rd_data2<'a>(input: &mut &'a str) -> ModalResult<RotationFormat<'a>> {
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
            time: "1.5",
            format: RotationFormat::Quaternion {
                quat_x: "0.0",
                quat_y: "0.0",
                quat_z: "0.0",
                quat_w: "1.0",
            },
        };
        let parsed = must_parse(parse_rd_data, input);
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_format2_rotation() {
        let input = "RD 2.0 90\n";
        let expected = RotationData {
            time: "2.0",
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
            time: "1.5",
            format: RotationFormat::Quaternion {
                quat_x: "0.0",
                quat_y: "0.0",
                quat_z: "0.0",
                quat_w: "1.0",
            },
        };
        let parsed1 = must_parse(parse_rd_data, input1);
        assert_eq!(parsed1, expected1);

        let input2 = "RD 2.0 45\n";
        let expected2 = RotationData {
            time: "2.0",
            format: RotationFormat::DeltaZAngle {
                delta_z_angle: 45.0,
            },
        };
        let parsed2 = must_parse(parse_rd_data, input2);
        assert_eq!(parsed2, expected2);
    }
}
