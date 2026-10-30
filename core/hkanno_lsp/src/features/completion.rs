//! Completion candidates for the cursor position.
//!
//! # How the candidates are chosen
//! The identifier being typed (e.g., `SG` in `PIE.@SG|`) is cut off first,
//! and the line **before it** decides the context.
//! So the list stays the same while the user keeps typing, and the GUI only filters it.
//!
//! ```text
//! "0.1 PIE.@SG"      context = "0.1 PIE.@"  -> native instructions, filtered by "SG"
//! "0.1 "             context = "0.1 "       -> event names (animmotion, ...)
//! ""                 context = ""           -> <time>, trackName:, # numAnnotations:
//! ```

use std::borrow::Cow;

use super::byte_offset;
use crate::{
    Span,
    parser::{LineNode, MotionNode, PieNode, RotationNode, Syntax, TextNode, parse_line},
    pie,
};

/// Icon/color category of a candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionKind {
    /// e.g., `trackName:`
    Keyword,
    /// e.g., a number placeholder.
    Value,
    /// e.g., `animmotion`, `SGVB`
    Function,
    /// e.g., `<eventName>`
    Snippet,
    /// e.g., `@`, `$`, `!`
    EnumMember,
}

/// One candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionItem {
    /// Text shown in the list.
    pub label: &'static str,
    /// Category.
    pub kind: CompletionKind,
    /// Text to insert, in [`crate::snippet`] syntax.
    pub insert: Cow<'static, str>,
    /// Markdown shown next to the list.
    pub documentation: &'static str,
}

/// Result of [`complete`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CompletionList {
    /// Columns to replace with the chosen item (= the identifier being typed).
    pub replace: Span,
    /// Candidates already filtered by the typed identifier.
    pub items: Vec<CompletionItem>,
}

/// Computes candidates for `cursor` (char column) in `line`.
///
/// # Example
/// ```
/// use hkanno_lsp::features::completion::complete;
///
/// let list = complete("0.1 anim", 8);
/// let labels: Vec<_> = list.items.iter().map(|i| i.label).collect();
/// assert_eq!(labels, ["animmotion", "animrotation"]);
/// ```
pub fn complete(line: &str, cursor: usize) -> CompletionList {
    let word_start = word_start(line, cursor);
    let context = &line[..byte_offset(line, word_start)];
    let typed: String = line.chars().skip(word_start).take(cursor - word_start).collect();

    let items = match parse_line(context, Syntax::Extended) {
        LineNode::Motion(node) => motion(&node, word_start),
        LineNode::Rotation(node) => rotation(&node),
        LineNode::Text(node) => text(&node, word_start),
        LineNode::Pie(node) => pie_items(&node, word_start),
        LineNode::Comment(_) | LineNode::TrackName(_) | LineNode::IFrame(_) => Vec::new(),
    };

    let typed = typed.to_lowercase();
    let items = items
        .into_iter()
        .filter(|item| typed.is_empty() || item.label.to_lowercase().contains(&typed))
        .collect();

    CompletionList { replace: Span::new(word_start, cursor), items }
}

/// Start column of the identifier (`[A-Za-z0-9_]*`) that ends at `cursor`.
fn word_start(line: &str, cursor: usize) -> usize {
    let before: Vec<char> = line.chars().take(cursor).collect();
    let word_len = before.iter().rev().take_while(|c| c.is_alphanumeric() || **c == '_').count();
    cursor.min(before.len()) - word_len
}

/// Is the cursor at or after the end of `span`?
fn is_after(span: Option<Span>, cursor: usize) -> bool {
    span.is_some_and(|span| cursor >= span.end)
}

const fn item(
    label: &'static str,
    kind: CompletionKind,
    insert: &'static str,
    documentation: &'static str,
) -> CompletionItem {
    CompletionItem { label, kind, insert: Cow::Borrowed(insert), documentation }
}

// ── Line kinds ───────────────────────────────────────────────────────────────

/// `<time> <event>` lines (the most common case).
fn text(node: &TextNode, cursor: usize) -> Vec<CompletionItem> {
    use CompletionKind::*;

    // Empty line: start of an annotation or a header.
    if node.time.is_none() {
        return vec![
            item(
                "<time>",
                Value,
                "0.0",
                "```hkanno\n<time: f32>\n```\nThe timestamp at which this annotation occurs.",
            ),
            item(
                "# numAnnotations:",
                Keyword,
                "# numAnnotations: ${1:usize}",
                "```hkanno\n# numAnnotations: <number>\n```\nDeclare the number of annotations in this document.",
            ),
            item(
                "trackName:",
                Keyword,
                "trackName: ${1:Name}",
                "```hkanno\ntrackName: <name>\n```\nDeclare the track name for this annotation track.",
            ),
        ];
    }

    // Event names only make sense once `<time><space>` is written.
    if !is_after(node.space_after_time, cursor) {
        return Vec::new();
    }

    vec![
        item(
            "<eventName>",
            Snippet,
            "${1:eventName}",
            "```hkanno\n${1:eventName}\n```\nAnnotation text event name(e.g. `weaponSwing`).",
        ),
        item(
            "animmotion",
            Function,
            "animmotion ${1:0.0} ${2:0.0} ${3:0.0}",
            "```hkanno\nanimmotion <x: f32> <y: f32> <z: f32>\n```\nInsert an animmotion event with X, Y, Z coordinates.\n(Need `AMR` Mod)",
        ),
        item(
            "animrotation",
            Function,
            "animrotation ${1:0}",
            "```hkanno\nanimrotation <degrees: f32>\n```\nInsert an animrotation event with a rotation in degrees.\n(Need `AMR` Mod)",
        ),
        item(
            "SpecialFrames_Invincible",
            Function,
            r#"SpecialFrames_Invincible{"Duration":${1:0.5}}"#,
            "```hkanno\nSpecialFrames_Invincible{\"<key>\": <value>}\n```\nInsert an IFrame event (Duration supported).",
        ),
        item(
            "PIE",
            Function,
            "PIE.",
            "```hkanno\nPIE.@<instruction>|<param1>|...\n```\nPayload Interpreter instruction.",
        ),
    ]
}

/// `animmotion x y z`: suggests the current value of the axis under the cursor.
fn motion(node: &MotionNode, cursor: usize) -> Vec<CompletionItem> {
    use CompletionKind::Value;

    let end_of = |span: Option<Span>| span.map_or(0, |s| s.end);
    let start_of = |span: Option<Span>| span.map_or(usize::MAX, |s| s.start);
    let value = |field: Option<&crate::parser::Field<f64>>| {
        Cow::Owned(field.map_or(0.0, |f| f.value).to_string())
    };

    let (label, insert, documentation) =
        if cursor >= end_of(node.space_after_event) && cursor < start_of(node.space_after_x) {
            ("x", value(node.x.as_ref()), "X coordinate for animmotion event.")
        } else if cursor >= end_of(node.space_after_x) && cursor < start_of(node.space_after_y) {
            ("y", value(node.y.as_ref()), "Y coordinate for animmotion event.")
        } else if cursor >= end_of(node.space_after_y) {
            ("z", value(node.z.as_ref()), "Z coordinate for animmotion event.")
        } else {
            return Vec::new();
        };

    vec![CompletionItem { label, kind: Value, insert, documentation }]
}

/// `animrotation <degrees>`: offers a placeholder when the angle is missing.
fn rotation(node: &RotationNode) -> Vec<CompletionItem> {
    if node.degrees.is_none() && node.space_after_event.is_some() {
        vec![item(
            "degrees",
            CompletionKind::Value,
            "0.0",
            "<time: f32> animrotation <degrees: f32>\nInsert an animrotation event with a rotation in degrees",
        )]
    } else {
        Vec::new()
    }
}

/// `PIE` -> `.` -> `@`/`$`/`!` -> instruction name.
fn pie_items(node: &PieNode, cursor: usize) -> Vec<CompletionItem> {
    use CompletionKind::*;

    if !node.event.is_some_and(|e| e.value.eq_ignore_ascii_case("pie")) {
        return Vec::new();
    }
    if node.dot.is_none() {
        return if is_after(node.event.map(|e| e.span), cursor) {
            vec![item(".", EnumMember, ".", "")]
        } else {
            Vec::new()
        };
    }

    let Some(ins) = &node.instruction else {
        return if is_after(node.dot, cursor) {
            vec![
                item("@", EnumMember, "@", "Native Instruction prefix"),
                item("$", EnumMember, "$", "Custom Instruction prefix"),
                item("!", EnumMember, "!", "Async payload prefix"),
            ]
        } else {
            Vec::new()
        };
    };

    if ins.name.is_none() && is_after(Some(ins.prefix.span), cursor) {
        return pie::instructions_for(ins.prefix.value)
            .iter()
            .map(|def| item(def.name, Function, def.snippet, def.documentation))
            .collect();
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(line: &str) -> Vec<&'static str> {
        complete(line, line.chars().count()).items.into_iter().map(|i| i.label).collect()
    }

    #[test]
    fn empty_line() {
        assert_eq!(labels(""), ["<time>", "# numAnnotations:", "trackName:"]);
        assert_eq!(labels("tr"), ["trackName:"]);
    }

    #[test]
    fn pie_flow() {
        assert_eq!(labels("0.1 PIE."), ["@", "$", "!"]);
        assert_eq!(labels("0.1 PIE.@SG"), ["SGVB", "SGVF", "SGVI"]);
    }

    #[test]
    fn replace_range_is_the_typed_word() {
        let list = complete("0.1 PIE.@SG", 11);
        assert_eq!(list.replace, Span::new(9, 11));
    }
}
