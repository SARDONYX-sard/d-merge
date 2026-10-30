//! Inlay hints: parameter names for the values on a line.
//!
//! e.g., `0.5 animmotion 1 2 3` -> `time`, `event`, `x`, `y`, `z`
//!
//! Each hint carries the column of the value it names. The egui editor draws them
//! at the end of the line (inserting virtual text would break cursor mapping).

use crate::parser::{LineNode, Syntax, parse_line};

/// A label attached to a column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InlayHint {
    /// Column of the value being labeled.
    pub column: usize,
    /// e.g., `x`
    pub label: &'static str,
}

/// Hints for one line. Comments and `trackName:` lines have none.
pub fn inlay_hints(line: &str) -> Vec<InlayHint> {
    let mut hints = Vec::new();
    let mut push = |column: Option<usize>, label| {
        if let Some(column) = column {
            hints.push(InlayHint { column, label });
        }
    };

    let node = parse_line(line, Syntax::Extended);
    push(node.time().map(|t| t.span.start), "time");

    match node {
        LineNode::Motion(n) => {
            push(n.event.map(|s| s.start), "event");
            push(n.x.map(|f| f.span.start), "x");
            push(n.y.map(|f| f.span.start), "y");
            push(n.z.map(|f| f.span.start), "z");
        }
        LineNode::Rotation(n) => {
            push(n.event.map(|s| s.start), "event");
            push(n.degrees.map(|f| f.span.start), "degree");
        }
        LineNode::IFrame(n) => push(n.event.map(|s| s.start), "event"),
        LineNode::Pie(n) => push(n.event.map(|e| e.span.start), "event"),
        LineNode::Text(n) => push(n.text.map(|t| t.span.start), "event"),
        LineNode::Comment(_) | LineNode::TrackName(_) => {}
    }
    hints
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn motion_hints() {
        let labels: Vec<_> = inlay_hints("0.5 animmotion 1 2 3").iter().map(|h| h.label).collect();
        assert_eq!(labels, ["time", "event", "x", "y", "z"]);
    }
}
