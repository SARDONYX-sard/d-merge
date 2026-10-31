//! Hover: Markdown explaining the token under the cursor.
//!
//! Hovering an **event name** (e.g., `animmotion`) shows its documentation;
//! hovering anywhere else on the line shows the parsed values.

use std::fmt::Write as _;

use crate::{
    Span,
    parser::{
        IFrameNode, LineNode, MotionNode, PieNode, RotationNode, Syntax, TextNode, TrackNameNode,
        parse_line,
    },
    scar,
};

const UNKNOWN: &str = "<unknown>";

/// Markdown for `cursor` (char column) in `line`, or `None` when there is nothing to say.
///
/// # Example
/// ```
/// let md = hkanno_lsp::features::hover::hover("0.5 animrotation 90", 0).unwrap();
/// assert!(md.contains("Degrees: 90°"));
/// ```
pub fn hover(line: &str, cursor: usize) -> Option<String> {
    let markdown = match parse_line(line, Syntax::Extended) {
        LineNode::TrackName(node) => track_name(&node),
        LineNode::Motion(node) => motion(&node, cursor),
        LineNode::Rotation(node) => rotation(&node, cursor),
        LineNode::Text(node) => text(&node, cursor),
        LineNode::IFrame(node) => iframe(&node, cursor),
        LineNode::Pie(node) => pie(&node, cursor),
        LineNode::Comment(_) => return None,
    };
    Some(markdown)
}

/// Formats an optional number field or `<unknown>`.
fn num(value: Option<f64>) -> String {
    value.map_or_else(|| UNKNOWN.to_string(), |v| v.to_string())
}

fn on(span: Option<Span>, cursor: usize) -> bool {
    span.is_some_and(|span| span.touches(cursor))
}

fn track_name(node: &TrackNameNode) -> String {
    let name = node.name.as_ref().map_or("<unnamed>", |n| n.value);
    format!(
        "# Annotation Track\nThis defines a named annotation track. All following annotations belong to this track until the next trackName or end of file.\n\n- Track name: `{name}`"
    )
}

fn motion(node: &MotionNode, cursor: usize) -> String {
    if on(node.event, cursor) {
        return "# Anim Motion\nApplies linear motion to the animation.\n- required: [Animation Motion Revolution](https://www.nexusmods.com/skyrimspecialedition/mods/50258)\n\n# Format\n\n```hkanno\nanimmotion <x: f32> <y: f32> <z: f32>\n```".to_string();
    }

    let value = |f: &Option<crate::parser::Field<f64>>| num(f.as_ref().map(|f| f.value));
    format!(
        "# animmotion values\n- Time: {}s\n- X: {}\n- Y: {}\n- Z: {}",
        value(&node.time),
        value(&node.x),
        value(&node.y),
        value(&node.z)
    )
}

fn rotation(node: &RotationNode, cursor: usize) -> String {
    if on(node.event, cursor) {
        return "# Anim Rotation\nApplies rotation to the animation.\n- required: [Animation Motion Revolution](https://www.nexusmods.com/skyrimspecialedition/mods/50258)\n\n# Format\n\n```hkanno\nanimrotation <degrees: f32>\n```".to_string();
    }

    format!(
        "# animrotation value\n- Time: {}s\n- Degrees: {}°",
        num(node.time.as_ref().map(|f| f.value)),
        num(node.degrees.as_ref().map(|f| f.value))
    )
}

fn text(node: &TextNode, cursor: usize) -> String {
    if let Some(text) = &node.text
        && let Some(markdown) = scar_hover(text.value, cursor.saturating_sub(text.span.start))
    {
        return markdown;
    }
    format!(
        "# Text annotation\n- Time: {}s\n- Text: `{}`",
        num(node.time.as_ref().map(|f| f.value)),
        node.text.as_ref().map_or(UNKNOWN, |t| t.value)
    )
}

/// SCAR annotations: the docs of the JSON key under `column` (in `text`), else of the event.
fn scar_hover(text: &str, column: usize) -> Option<String> {
    let trimmed = text.trim_start();
    if trimmed.starts_with(scar::ACTION_DATA) {
        // e.g., hovering `"MaxDistance"` -> the docs of that key.
        let chars: Vec<char> = text.chars().collect();
        let key_at = scar::ACTION_KEYS.iter().find(|key| {
            let quoted: Vec<char> = format!("\"{}\"", key.name).chars().collect();
            (0..chars.len()).any(|at| {
                chars[at..].starts_with(&quoted) && (at..at + quoted.len()).contains(&column)
            })
        });
        return Some(match key_at {
            Some(key) => format!(
                "# SCAR_ActionData: `{}`\n{}\n\n- See: [SCAR manual]({})",
                key.name,
                key.doc,
                scar::MANUAL_URL
            ),
            None => {
                format!("{}\n- See: [SCAR manual]({})", scar::ACTION_DATA_DOC, scar::MANUAL_URL)
            }
        });
    }
    let doc = if trimmed.starts_with(scar::COMBO_START) {
        scar::COMBO_START_DOC
    } else if trimmed.starts_with(scar::UPDATE_DUMMY) {
        scar::UPDATE_DUMMY_DOC
    } else {
        return None;
    };
    Some(format!("{doc}\n- See: [SCAR manual]({})", scar::MANUAL_URL))
}

fn iframe(node: &IFrameNode, cursor: usize) -> String {
    if on(node.event, cursor) {
        return r#"# Invincibility Frames Annotation
- required: [IFrame Generator RE](https://www.nexusmods.com/skyrimspecialedition/mods/74401)
- See: [tutorial](https://github.com/max-su-2019/MaxsuIFrame/blob/main/doc/en/tutorial.md)

# Format

- As of v1.03, it appears to support only `Duration`.

```hkanno
SpecialFrames_Invincible{"Duration": <value: f32>}
```

# Example

Invincible for 0.1 to 0.5 seconds

```hkanno
0.1 SpecialFrames_Invincible{"Duration": 0.5}
```"#
            .to_string();
    }

    let json = match node.json.as_ref().map(|j| &j.parsed) {
        Some(Ok(value)) => serde_json::to_string_pretty(value).unwrap_or_default(),
        Some(Err(err)) => format!("<{err}>"),
        None => UNKNOWN.to_string(),
    };
    format!(
        "# I-Frame value\n- Time: {}s\n- JSON Data:\n```json\n{json}\n```",
        num(node.time.as_ref().map(|f| f.value))
    )
}

fn pie(node: &PieNode, cursor: usize) -> String {
    if on(node.event.as_ref().map(|e| e.span), cursor) {
        return "# Payload Interpreter Dummy event (PIE)\nPayload instruction.\n- required: [Payload Interpreter](https://www.nexusmods.com/skyrimspecialedition/mods/65089)\n- See: [Reference](https://github.com/D7ry/PayloadInterpreter?tab=readme-ov-file#list-of-instructions)\n\n# Format\n- Native instruction\n```hkanno\nPIE.@<instruction>|<param1>|<param2>|...\n```\n- Custom instruction\n```hkanno\nPIE.$KEY|instruction1|instruction2|...\n```\n- Async instruction\n```hkanno\nPIE.$[time]<rest>\n```".to_string();
    }

    let ins = node.instruction.as_ref();
    let kind = ins.map_or(UNKNOWN, |ins| ins.prefix.value.display_name());
    let name = ins.and_then(|ins| ins.name.as_ref()).map_or(UNKNOWN, |n| n.value);
    let params: Vec<&str> = ins
        .and_then(|ins| ins.parameters.as_ref())
        .map(|params| {
            params.items.iter().map(|p| p.value.as_ref().map_or(UNKNOWN, |v| v.value)).collect()
        })
        .unwrap_or_default();

    let mut md = format!("# PIE Instruction\n- Kind: `{kind}`\n- Name: `{name}`\n");
    if params.is_empty() {
        md.push_str("- No parameters");
    } else {
        let _ = write!(md, "- Parameters: {}", params.join(" | "));
    }
    md
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_docs_vs_values() {
        let line = "0.5 animmotion 1 2 3";
        assert!(hover(line, 6).is_some_and(|md| md.starts_with("# Anim Motion")));
        assert!(hover(line, 0).is_some_and(|md| md.contains("- Z: 3")));
    }

    #[test]
    fn comment_has_no_hover() {
        assert_eq!(hover("# comment", 3), None);
    }
}
