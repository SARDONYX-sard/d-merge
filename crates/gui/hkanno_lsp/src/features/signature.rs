//! Signature help: "which argument am I typing?" shown while editing a line.
//!
//! Only the text **before** the cursor is parsed, so the active parameter follows the cursor.
//! e.g., `0.5 animmotion 1 |` (cursor at `|`) -> `animmotion <x> <y> <z>` with `y` active.

use super::byte_offset;
use crate::{
    Span,
    parser::{LineNode, MotionNode, PieNode, Syntax, parse_line},
    pie,
};

/// A signature to show above/below the cursor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureHelp {
    /// e.g., `animmotion <x: f32> <y: f32> <z: f32>`
    pub label: String,
    /// Extra Markdown.
    pub documentation: Option<&'static str>,
    /// Parameter names.
    pub params: Vec<String>,
    /// Index into `params` of the parameter under the cursor.
    pub active: usize,
}

/// Signature for `cursor` (char column) in `line`.
pub fn signature_help(line: &str, cursor: usize) -> Option<SignatureHelp> {
    let before = &line[..byte_offset(line, cursor)];

    match parse_line(before, Syntax::Extended) {
        LineNode::Pie(node) => pie_signature(&node),
        LineNode::Motion(node) => Some(motion(&node, cursor)),
        LineNode::Rotation(node) => Some(signature(
            "animrotation <angle: f32>",
            "Applies a rotation (in degrees) to the animation.",
            &["angle"],
            usize::from(node.degrees.is_some()),
        )),
        LineNode::Text(node) => {
            // Until `<time><space>` is complete, the time is being typed.
            let (label, param) = if node.time.is_none() || node.space_after_time.is_none() {
                ("<time: f32>", "time")
            } else {
                ("<text: string>", "text")
            };
            Some(SignatureHelp {
                label: label.to_string(),
                documentation: Some(if param == "time" {
                    "Timestamp in seconds (e.g., 0.100000)"
                } else {
                    "Annotation label or event name (e.g., `MCO_DodgeOpen`, `animmotion`, `animrotation`)"
                }),
                params: vec![param.to_string()],
                active: 0,
            })
        }
        LineNode::Comment(_) | LineNode::TrackName(_) | LineNode::IFrame(_) => None,
    }
}

fn signature(
    label: &str,
    documentation: &'static str,
    params: &[&str],
    active: usize,
) -> SignatureHelp {
    SignatureHelp {
        label: label.to_string(),
        documentation: Some(documentation),
        params: params.iter().map(ToString::to_string).collect(),
        active: active.min(params.len().saturating_sub(1)),
    }
}

fn motion(node: &MotionNode, cursor: usize) -> SignatureHelp {
    // A value counts as "being typed" until the cursor moves past its end.
    let typing = |span: Option<Span>| span.is_none_or(|s| s.end >= cursor);
    let active = if node.x.is_none() || typing(node.x.as_ref().map(|f| f.span)) {
        0
    } else if typing(node.y.as_ref().map(|f| f.span)) {
        1
    } else {
        2
    };

    signature(
        "animmotion <x: f32> <y: f32> <z: f32>",
        "Applies linear motion offset to the animation.",
        &["x", "y", "z"],
        active,
    )
}

/// `PIE.@SGVB|x|` -> `SGVB|<graphVariable>|<bool>` with `bool` active.
fn pie_signature(node: &PieNode) -> Option<SignatureHelp> {
    let ins = node.instruction.as_ref()?;
    let def = pie::find_native(ins.name.as_ref()?.value)?;

    // Parameter names come from the snippet placeholders: `${1:graphVariable}` -> `graphVariable`.
    let params: Vec<String> = def
        .snippet
        .split("${")
        .skip(1)
        .filter_map(|part| part.split_once(':').and_then(|(_, rest)| rest.split_once('}')))
        .map(|(name, _)| name.to_string())
        .collect();

    // Each `|` starts the next parameter.
    let separators = ins.parameters.as_ref().map_or(0, |p| p.items.len());
    let label = format!(
        "{}|{}",
        def.name,
        params.iter().map(|p| format!("<{p}>")).collect::<Vec<_>>().join("|")
    );

    Some(SignatureHelp {
        label,
        documentation: Some(def.documentation),
        active: separators.saturating_sub(1).min(params.len().saturating_sub(1)),
        params,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn motion_follows_cursor() {
        let line = "0.5 animmotion 1 2";
        assert_eq!(signature_help(line, 16).map(|s| s.active), Some(0));
        assert_eq!(signature_help(line, 17).map(|s| s.active), Some(1));
        assert_eq!(signature_help("0.5 animmotion 1 2 ", 19).map(|s| s.active), Some(2));
    }

    #[test]
    fn pie_params() {
        let help = signature_help("0.1 PIE.@SGVB|x|", 16).expect("signature");
        assert_eq!(help.params, ["graphVariable", "bool"]);
        assert_eq!(help.active, 1);
    }
}
