//! Document formatter.
//!
//! # Rules
//! - leading/trailing optional spaces are removed
//! - mandatory spaces are collapsed into one
//! - numbers are written with 6 decimals (same as `hkanno` output), e.g., `0.1` -> `0.100000`
//! - `trackName:` is normalized to `trackName: <name>`
//!
//! Only the core syntax is understood ([`Syntax::Core`]), so mod payloads such as
//! `PIE.@...` are treated as plain text and kept as written.

use crate::parser::{LineNode, Syntax, parse_line};

/// Formats the whole document. Line count is preserved.
///
/// # Example
/// ```
/// let formatted = hkanno_lsp::features::format::format_document("  0.1   animrotation   90  ");
/// assert_eq!(formatted, "0.100000 animrotation 90.000000");
/// ```
pub fn format_document(text: &str) -> String {
    crate::lines(text).map(format_line).collect::<Vec<_>>().join("\n")
}

/// Formats one line.
pub fn format_line(line: &str) -> String {
    let number = |value: f64| format!("{value:.6}");

    // `None` parts are skipped, present ones are joined by exactly one space.
    let join =
        |parts: &[Option<String>]| parts.iter().flatten().cloned().collect::<Vec<_>>().join(" ");
    let field = |f: Option<crate::parser::Field<f64>>| f.map(|f| number(f.value));

    match parse_line(line, Syntax::Core) {
        LineNode::Rotation(n) => {
            join(&[field(n.time), n.event.map(|_| "animrotation".to_string()), field(n.degrees)])
        }
        LineNode::Motion(n) => join(&[
            field(n.time),
            n.event.map(|_| "animmotion".to_string()),
            field(n.x),
            field(n.y),
            field(n.z),
        ]),
        LineNode::Text(n) => join(&[field(n.time), n.text.map(|t| t.value.to_string())]),
        LineNode::TrackName(n) => format!("trackName: {}", n.name.map_or("", |n| n.value)),
        LineNode::Comment(n) => {
            join(&[Some("#".to_string()), n.comment.map(|c| c.value.to_string())])
        }
        // Unreachable with `Syntax::Core`; keep the line untouched just in case.
        LineNode::IFrame(_) | LineNode::Pie(_) => line.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_line_count() {
        let text = "#  numAnnotations: 2\n\ntrackname:   Root\n0.1   MCO_Open\n0.2 PIE.@SGVB|x|1";
        assert_eq!(
            format_document(text),
            "# numAnnotations: 2\n\ntrackName: Root\n0.100000 MCO_Open\n0.200000 PIE.@SGVB|x|1"
        );
    }
}
