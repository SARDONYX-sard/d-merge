//! Parsers for each core line kind (+ `SpecialFrames_Invincible`).
//!
//! They all follow the grammar in [`super::nodes::LineNode`] and stop at the first piece
//! that does not match, leaving the remaining fields `None`.

use super::{cursor::Cursor, nodes::*};

/// `<space0> # <comment>`
pub(super) fn comment(line: &str) -> CommentNode<'_> {
    let mut c = Cursor::new(line);
    c.space();
    let hash = c.literal("#", false);
    c.space();
    CommentNode { hash, comment: c.rest() }
}

/// `<space0> trackName: <name>`
pub(super) fn track_name(line: &str) -> TrackNameNode<'_> {
    let mut c = Cursor::new(line);
    c.space();
    let literal = c.literal("trackName:", true);
    c.space();
    let name = c.rest().map(|field| Field { value: field.value.trim(), span: field.span });
    TrackNameNode { literal, name }
}

/// `<space0> <time> <space1> <text>`
pub(super) fn text(line: &str) -> TextNode<'_> {
    let mut c = Cursor::new(line);
    c.space();
    let time = c.number();
    let space_after_time = c.space();
    TextNode { time, space_after_time, text: c.rest() }
}

/// `<space0> <time> <space1> animmotion <space1> <x> <space1> <y> <space1> <z>`
pub(super) fn motion(line: &str) -> MotionNode {
    let mut c = Cursor::new(line);
    c.space();
    MotionNode {
        time: c.number(),
        space_after_time: c.space(),
        event: c.literal("animmotion", false),
        space_after_event: c.space(),
        x: c.number(),
        space_after_x: c.space(),
        y: c.number(),
        space_after_y: c.space(),
        z: c.number(),
    }
}

/// `<space0> <time> <space1> animrotation <space1> <degrees>`
pub(super) fn rotation(line: &str) -> RotationNode {
    let mut c = Cursor::new(line);
    c.space();
    RotationNode {
        time: c.number(),
        space_after_time: c.space(),
        event: c.literal("animrotation", false),
        space_after_event: c.space(),
        degrees: c.number(),
    }
}

/// `<space0> <time> <space1> SpecialFrames_Invincible{...json...}`
pub(super) fn iframe(line: &str) -> IFrameNode {
    let mut c = Cursor::new(line);
    c.space();
    let time = c.number();
    let space_after_time = c.space();
    let event = c.literal("SpecialFrames_Invincible", false);
    let json = c.rest().map(|raw| IFrameJson {
        span: raw.span,
        parsed: serde_json::from_str(raw.value.trim()).map_err(|e| e.to_string()),
    });

    IFrameNode { time, space_after_time, event, json }
}

/// Does `token` start like a number the [`Cursor::number`] would accept?
///
/// e.g., `0.5`, `-1`, `.5` -> `true`; `PIE` -> `false`.
pub(super) fn looks_like_number(token: &str) -> bool {
    let rest = token.strip_prefix(['+', '-']).unwrap_or(token);
    rest.starts_with(|c: char| c.is_ascii_digit() || c == '.')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn motion_partial() {
        let node = motion("  0.5 animmotion 1 2");
        assert_eq!(node.time.map(|t| t.value), Some(0.5));
        assert_eq!(node.event.map(|s| (s.start, s.end)), Some((6, 16)));
        assert_eq!(node.y.map(|y| y.value), Some(2.0));
        assert!(node.z.is_none());
    }

    #[test]
    fn track_name_trims() {
        let node = track_name("trackName:   Foot_L  ");
        assert_eq!(node.name.map(|n| n.value), Some("Foot_L"));
    }

    #[test]
    fn text_keeps_unicode_columns() {
        // `␀` is 3 bytes but 1 char.
        let node = text("0.5 ␀");
        assert_eq!(node.text.map(|t| (t.span.start, t.span.end)), Some((4, 5)));
    }

    #[test]
    fn iframe_json() {
        let node = iframe(r#"0.1 SpecialFrames_Invincible{"Duration":0.5}"#);
        assert!(node.json.is_some_and(|j| j.parsed.is_ok()));

        let node = iframe("0.1 SpecialFrames_Invincible{bad");
        assert!(node.json.is_some_and(|j| j.parsed.is_err()));
    }
}
