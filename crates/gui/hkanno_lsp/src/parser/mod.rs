//! One-line parser for the hkanno text format.
//!
//! The format is line oriented, so every feature parses only the lines it needs.
//! The parser never fails: unknown or half-typed input becomes a [`LineNode::Text`]
//! or a node with `None` parts, which diagnostics then report.
//!
//! # Example
//! ```
//! use hkanno_lsp::parser::{parse_line, LineNode, Syntax};
//!
//! let LineNode::Motion(motion) = parse_line("0.5 animmotion 1 2", Syntax::Extended) else {
//!     panic!("expected a motion line");
//! };
//! assert_eq!(motion.y.map(|y| y.value), Some(2.0));
//! assert!(motion.z.is_none()); // still being typed
//! ```

mod cursor;
mod lines;
pub mod nodes;
mod pie;

pub use self::nodes::*;

/// Which line kinds the parser should recognize.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Syntax {
    /// Only the core syntax. Used by the formatter so that it never rewrites
    /// mod-specific payloads (e.g., `PIE.@...`) that it does not understand.
    Core,
    /// Core syntax + mod extensions (`SpecialFrames_Invincible`, `PIE`).
    /// Used by every interactive feature.
    Extended,
}

/// Parses one line (without the trailing newline).
///
/// Line kinds are tried in this order; the first match wins:
/// 1. (extended) `SpecialFrames_Invincible` anywhere in the line
/// 2. (extended) a `PIE` event token (see [`is_pie_line`])
/// 3. `#` comment
/// 4. `trackName:`
/// 5. `animrotation` anywhere in the line
/// 6. `animmotion` anywhere in the line
/// 7. otherwise: plain text annotation
pub fn parse_line(line: &str, syntax: Syntax) -> LineNode<'_> {
    let lower = line.to_lowercase();

    if syntax == Syntax::Extended {
        if lower.contains("specialframes_invincible") {
            return LineNode::IFrame(lines::iframe(line));
        }
        if is_pie_line(line) {
            return LineNode::Pie(pie::pie(line));
        }
    }

    let trimmed = lower.trim_start();
    if trimmed.starts_with('#') {
        LineNode::Comment(lines::comment(line))
    } else if trimmed.starts_with("trackname:") {
        LineNode::TrackName(lines::track_name(line))
    } else if lower.contains("animrotation") {
        LineNode::Rotation(lines::rotation(line))
    } else if lower.contains("animmotion") {
        LineNode::Motion(lines::motion(line))
    } else {
        LineNode::Text(lines::text(line))
    }
}

/// Parses every line of a document.
///
/// e.g., `parse_document("a\nb", Syntax::Extended).len() == 2`
pub fn parse_document(text: &str, syntax: Syntax) -> Vec<LineNode<'_>> {
    crate::lines(text).map(|line| parse_line(line, syntax)).collect()
}

/// Is the event token (the first token after the time) a `PIE` event?
///
/// e.g., `0.1 PIE.@SGVB|x|1` and the half-typed `0.1 PIE` are, but `0.1 MCO_Pierce` is not.
///
/// # Why not `contains("pie")`?
/// The old TS implementation used a plain substring check, which mis-detected ordinary
/// events such as `MCO_Pierce`. Only the event token is checked here.
pub fn is_pie_line(line: &str) -> bool {
    let mut tokens = line.split_whitespace();
    let Some(first) = tokens.next() else { return false };
    // Skip the time token when present.
    let event = if lines::looks_like_number(first) { tokens.next() } else { Some(first) };

    event.is_some_and(|event| {
        let lower = event.to_lowercase();
        lower == "pie" || lower.starts_with("pie.")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatch() {
        let kind = |line| match parse_line(line, Syntax::Extended) {
            LineNode::Comment(_) => "comment",
            LineNode::TrackName(_) => "track",
            LineNode::Text(_) => "text",
            LineNode::Motion(_) => "motion",
            LineNode::Rotation(_) => "rotation",
            LineNode::IFrame(_) => "iframe",
            LineNode::Pie(_) => "pie",
        };

        assert_eq!(kind("# numAnnotations: 3"), "comment");
        assert_eq!(kind("  trackName: Foot_L"), "track");
        assert_eq!(kind("0.1 animmotion 0 0 1"), "motion");
        assert_eq!(kind("0.1 animrotation 90"), "rotation");
        assert_eq!(kind(r#"0.1 SpecialFrames_Invincible{"Duration":0.5}"#), "iframe");
        assert_eq!(kind("0.1 PIE.@SGVB|IsAttacking|1"), "pie");
        assert_eq!(kind("0.1 MCO_Pierce"), "text");
        assert_eq!(kind(""), "text");
    }

    #[test]
    fn core_syntax_ignores_extensions() {
        assert!(matches!(parse_line("0.1 PIE.@SGVB|x|1", Syntax::Core), LineNode::Text(_)));
    }
}
