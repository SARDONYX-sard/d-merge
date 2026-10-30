//! Syntax nodes produced by the line parser.
//!
//! Every part is `Option` because the editor parses **incomplete** lines while the user is typing.
//! e.g., `"0.5 animmotion 1"` is a [`MotionNode`] whose `y` and `z` are `None`.
//!
//! Spaces are kept as spans because some features need them.
//! e.g., completion only offers event names once the space after `time` exists.

use crate::Span;

/// A value together with where it was written.
///
/// `Copy` when `T` is (`&str`, `f64`, ...), so nodes can be passed around cheaply.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Field<T> {
    /// Parsed value. Numbers that failed to parse are `NaN` (see [`super::cursor::Cursor::number`]).
    pub value: T,
    /// Columns of the written text.
    pub span: Span,
}

/// One parsed line.
///
/// # Patterns
/// ``text
/// <space0> # <comment>                                              -> Comment
/// <space0> trackName: <name>                                        -> TrackName
/// <space0> <time> <space1> animmotion <space1> <x> <space1> <y> <space1> <z>  -> Motion
/// <space0> <time> <space1> animrotation <space1> <degrees>          -> Rotation
/// <space0> <time> <space1> SpecialFrames_Invincible{"Duration":0.5} -> IFrame   (extended only)
/// <space0> <time> <space1> PIE.@<NAME>|<param>|...                  -> Pie      (extended only)
/// <space0> <time> <space1> <text>                                   -> Text (fallback)
/// ``
#[derive(Debug, Clone, PartialEq)]
pub enum LineNode<'a> {
    /// `# comment`
    Comment(CommentNode<'a>),
    /// `trackName: Name`
    TrackName(TrackNameNode<'a>),
    /// `0.5 SomeEvent`
    Text(TextNode<'a>),
    /// `0.5 animmotion 0 1 2`
    Motion(MotionNode),
    /// `0.5 animrotation 90`
    Rotation(RotationNode),
    /// `0.5 SpecialFrames_Invincible{"Duration":0.5}`
    IFrame(IFrameNode),
    /// `0.5 PIE.@SGVB|IsAttacking|1`
    Pie(PieNode<'a>),
}

/// `<space0> # <comment>`
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CommentNode<'a> {
    /// The `#`.
    pub hash: Option<Span>,
    /// Everything after `#` and the following spaces.
    pub comment: Option<Field<&'a str>>,
}

/// `<space0> trackName: <name>`
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TrackNameNode<'a> {
    /// The `trackName:` literal (matched case-insensitively).
    pub literal: Option<Span>,
    /// Track name, trimmed. `span` covers the untrimmed text.
    pub name: Option<Field<&'a str>>,
}

/// `<space0> <time> <space1> <text>`
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TextNode<'a> {
    /// Annotation time in seconds.
    pub time: Option<Field<f64>>,
    /// The space between time and text.
    pub space_after_time: Option<Span>,
    /// Rest of the line (not trimmed).
    pub text: Option<Field<&'a str>>,
}

/// `<space0> <time> <space1> animmotion <space1> <x> <space1> <y> <space1> <z>`
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MotionNode {
    /// Annotation time in seconds.
    pub time: Option<Field<f64>>,
    /// The space between time and `animmotion`.
    pub space_after_time: Option<Span>,
    /// The `animmotion` literal.
    pub event: Option<Span>,
    /// The space between `animmotion` and `x`.
    pub space_after_event: Option<Span>,
    /// X translation.
    pub x: Option<Field<f64>>,
    /// The space between `x` and `y`.
    pub space_after_x: Option<Span>,
    /// Y translation.
    pub y: Option<Field<f64>>,
    /// The space between `y` and `z`.
    pub space_after_y: Option<Span>,
    /// Z translation.
    pub z: Option<Field<f64>>,
}

/// `<space0> <time> <space1> animrotation <space1> <degrees>`
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RotationNode {
    /// Annotation time in seconds.
    pub time: Option<Field<f64>>,
    /// The space between time and `animrotation`.
    pub space_after_time: Option<Span>,
    /// The `animrotation` literal.
    pub event: Option<Span>,
    /// The space between `animrotation` and the angle.
    pub space_after_event: Option<Span>,
    /// Rotation angle in degrees.
    pub degrees: Option<Field<f64>>,
}

/// `<space0> <time> <space1> SpecialFrames_Invincible{"<key>": <value>}`
///
/// Used by `IFrame Generator RE`. As of v1.03 only the `Duration` key is supported.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct IFrameNode {
    /// Annotation time in seconds.
    pub time: Option<Field<f64>>,
    /// The space between time and the event name.
    pub space_after_time: Option<Span>,
    /// The `SpecialFrames_Invincible` literal.
    pub event: Option<Span>,
    /// The JSON written right after the event name (no space in between).
    pub json: Option<IFrameJson>,
}

/// The JSON part of an [`IFrameNode`].
#[derive(Debug, Clone, PartialEq)]
pub struct IFrameJson {
    /// Columns of the raw JSON text.
    pub span: Span,
    /// Parsed JSON, or the parser error message.
    pub parsed: Result<serde_json::Value, String>,
}

/// `<space0> <time> <space1> PIE.<prefix><NAME>|<param>|...` (Payload Interpreter)
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PieNode<'a> {
    /// Annotation time in seconds.
    pub time: Option<Field<f64>>,
    /// The space between time and `PIE`.
    pub space_after_time: Option<Span>,
    /// Usually `PIE` (everything up to the first `.`).
    pub event: Option<Field<&'a str>>,
    /// The `.` after `PIE`.
    pub dot: Option<Span>,
    /// `@NAME|param|...`
    pub instruction: Option<Instruction<'a>>,
}

/// Kind of a PIE instruction, decided by its first char.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PiePrefix {
    /// `@`: native instruction (e.g., `@SGVB`).
    Native,
    /// `$`: custom instruction mapped by the user.
    Custom,
    /// `!`: async (delayed) instruction.
    Async,
}

impl PiePrefix {
    /// Parses the prefix char.
    pub const fn from_char(c: char) -> Option<Self> {
        match c {
            '@' => Some(Self::Native),
            '$' => Some(Self::Custom),
            '!' => Some(Self::Async),
            _ => None,
        }
    }

    /// Human readable kind (used by hover).
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Native => "Native",
            Self::Custom => "Custom",
            Self::Async => "Async",
        }
    }
}

/// `@NAME|param1|param2|...`
#[derive(Debug, Clone, PartialEq)]
pub struct Instruction<'a> {
    /// `@`, `$` or `!`.
    pub prefix: Field<PiePrefix>,
    /// e.g., `SGVB`.
    pub name: Option<Field<&'a str>>,
    /// Everything from the first `|`.
    pub parameters: Option<Parameters<'a>>,
}

/// `|param1|param2|...`
#[derive(Debug, Clone, PartialEq)]
pub struct Parameters<'a> {
    /// Each `|` + value pair.
    pub items: Vec<ParameterItem<'a>>,
    /// Columns of the whole parameter list.
    pub span: Span,
}

/// One `|value` pair. `value` is `None` for `||`.
#[derive(Debug, Clone, PartialEq)]
pub struct ParameterItem<'a> {
    /// The `|` before the value.
    pub separator: Option<Span>,
    /// The value text.
    pub value: Option<Field<&'a str>>,
}

impl LineNode<'_> {
    /// The annotation time, if this kind of line has one.
    pub const fn time(&self) -> Option<&Field<f64>> {
        match self {
            Self::Comment(_) | Self::TrackName(_) => None,
            Self::Text(n) => n.time.as_ref(),
            Self::Motion(n) => n.time.as_ref(),
            Self::Rotation(n) => n.time.as_ref(),
            Self::IFrame(n) => n.time.as_ref(),
            Self::Pie(n) => n.time.as_ref(),
        }
    }
}
