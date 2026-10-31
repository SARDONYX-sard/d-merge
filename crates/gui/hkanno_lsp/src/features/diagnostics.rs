//! Diagnostics: errors and warnings for the whole document.
//!
//! # What is checked
//! - `animmotion`: `x`, `y`, `z` present
//! - `animrotation`: `degrees` present
//! - numbers that look numeric but do not parse (e.g., `1.2.3`)
//! - text annotation: something after the time
//! - `SpecialFrames_Invincible`: time, event name, valid JSON, only the `Duration` key
//! - `PIE.@<native>`: enough parameters
//! - `SCAR_ActionData{...}`: valid JSON, known keys, value types / ranges, exclusive keys;
//!   `SCAR_ComboStart.<chance>`: chance 0-100

use crate::{
    LineSpan, Span,
    parser::{Field, IFrameNode, LineNode, PieNode, Syntax, parse_line},
    pie, scar,
};

/// How bad a diagnostic is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// The line cannot be written back as intended.
    Error,
    /// Probably a mistake, but the file is still valid.
    Warning,
}

/// Machine-readable tag so the GUI can offer a quick fix (see [`super::code_action`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticCode {
    /// `SpecialFrames_Invincible{"Duratoin": 0.5}` -> rename the key to `Duration`.
    FixIFrameKey,
    /// `SCAR_ActionData{"Chanse": 50}` -> rename the key (the quoted key is the range).
    RenameScarKey(&'static str),
}

/// One problem found in the document.
#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    /// Where to draw the underline.
    pub range: LineSpan,
    /// How bad it is.
    pub severity: Severity,
    /// Human readable message.
    pub message: String,
    /// Set when a quick fix exists.
    pub code: Option<DiagnosticCode>,
}

/// Checks every line of `text`.
///
/// # Example
/// ```
/// let diagnostics = hkanno_lsp::features::diagnostics::diagnostics("0.5 animmotion 1 2");
/// assert_eq!(diagnostics[0].message, "Missing z value in animmotion.");
/// ```
pub fn diagnostics(text: &str) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for (line_no, line) in crate::lines(text).enumerate() {
        let mut ctx = LineCtx { line_no, line_len: line.chars().count(), out: &mut out };
        match parse_line(line, Syntax::Extended) {
            LineNode::Motion(node) => {
                ctx.number(node.time.as_ref());
                ctx.required(node.x.as_ref(), "Missing x value in animmotion.");
                ctx.required(node.y.as_ref(), "Missing y value in animmotion.");
                ctx.required(node.z.as_ref(), "Missing z value in animmotion.");
            }
            LineNode::Rotation(node) => {
                ctx.number(node.time.as_ref());
                ctx.required(node.degrees.as_ref(), "Missing degrees value in animrotation.");
            }
            LineNode::Text(node) => {
                ctx.number(node.time.as_ref());
                if let Some(time) = &node.time
                    && node.text.is_none()
                {
                    let start = node.space_after_time.map_or(time.span.end, |s| s.start);
                    ctx.push(
                        Span::new(start, ctx.line_len),
                        Severity::Warning,
                        "Text annotation is missing.",
                        None,
                    );
                }
                if let Some(text) = &node.text {
                    scar(&mut ctx, text);
                }
            }
            LineNode::IFrame(node) => iframe(&mut ctx, &node),
            LineNode::Pie(node) => pie(&mut ctx, &node),
            LineNode::Comment(_) | LineNode::TrackName(_) => {}
        }
    }
    out
}

/// Per-line helper so each check is one short call.
struct LineCtx<'o> {
    line_no: usize,
    line_len: usize,
    out: &'o mut Vec<Diagnostic>,
}

impl LineCtx<'_> {
    fn push(
        &mut self,
        span: Span,
        severity: Severity,
        message: impl Into<String>,
        code: Option<DiagnosticCode>,
    ) {
        self.out.push(Diagnostic {
            range: span.on_line(self.line_no),
            severity,
            message: message.into(),
            code,
        });
    }

    /// The last char of the line (e.g., where a missing value would follow).
    const fn line_end(&self) -> Span {
        Span::new(self.line_len.saturating_sub(1), self.line_len)
    }

    /// The whole line; used when the missing piece has no position of its own.
    const fn whole_line(&self) -> Span {
        Span::new(0, self.line_len)
    }

    /// Error if `field` is missing, then [`Self::number`] checks its shape.
    fn required(&mut self, field: Option<&Field<f64>>, missing_message: &str) {
        match field {
            Some(_) => self.number(field),
            // Point at the end of the line, where the missing value goes.
            None => self.push(self.line_end(), Severity::Error, missing_message, None),
        }
    }

    /// Error if a written number does not parse (e.g., `1.2.3`).
    fn number(&mut self, field: Option<&Field<f64>>) {
        if let Some(field) = field
            && field.value.is_nan()
        {
            self.push(field.span, Severity::Error, "Invalid number.", None);
        }
    }
}

/// `SCAR_ActionData{...}` JSON and `SCAR_ComboStart.<chance>` (see [`crate::scar`]).
fn scar(ctx: &mut LineCtx, text: &Field<&str>) {
    let value = text.value.trim_end();
    let leading = value.chars().take_while(|c| c.is_whitespace()).count();
    let trimmed = value.trim_start();
    let start = text.span.start + leading;
    let end = start + trimmed.chars().count();

    if let Some(json) = trimmed.strip_prefix(scar::ACTION_DATA) {
        // Underline the JSON body (or the event name if it is missing).
        let json_start = start + scar::ACTION_DATA.chars().count();
        let span = if json.is_empty() { Span::new(start, end) } else { Span::new(json_start, end) };
        if json.is_empty() {
            ctx.push(span, Severity::Error, "SCAR_ActionData needs a JSON body `{...}`.", None);
            return;
        }
        for problem in scar::check_action_data(json) {
            let severity = if problem.is_error { Severity::Error } else { Severity::Warning };
            // Point at the key (e.g., `\"Chance\"`) when the problem is about one.
            let key_span = problem.key.as_deref().and_then(|key| {
                let quoted = format!("\"{key}\"");
                let at = json.find(&quoted)?;
                let start = json_start + json[..at].chars().count();
                Some(Span::new(start, start + quoted.chars().count()))
            });
            let code = problem.rename.map(DiagnosticCode::RenameScarKey);
            ctx.push(key_span.unwrap_or(span), severity, problem.message, code);
        }
    } else if let Some(message) = scar::check_combo_start(trimmed) {
        ctx.push(Span::new(start, end), Severity::Error, message, None);
    }
}

fn iframe(ctx: &mut LineCtx, node: &IFrameNode) {
    if node.time.is_none() {
        ctx.push(ctx.whole_line(), Severity::Error, "Missing time value in IFrame event.", None);
    }
    ctx.number(node.time.as_ref());

    if node.event.is_none() {
        ctx.push(
            ctx.whole_line(),
            Severity::Error,
            "Invalid IFrame event name (expected 'SpecialFrames_Invincible').",
            None,
        );
    }

    let Some(json) = &node.json else {
        ctx.push(
            ctx.whole_line(),
            Severity::Error,
            r#"IFrame JSON is missing or invalid. Only "Duration" is supported (IFrame Generator RE v1.03)."#,
            None,
        );
        return;
    };

    match &json.parsed {
        Err(err) => {
            ctx.push(json.span, Severity::Error, format!("IFrame JSON parse error: {err}"), None);
        }
        Ok(value) => {
            // e.g., `{"Duratoin": 0.5}` -> offer a rename to `Duration`.
            for key in value.as_object().into_iter().flat_map(|obj| obj.keys()) {
                if key != "Duration" {
                    ctx.push(
                        json.span,
                        Severity::Warning,
                        format!(
                            r#"Possible typo: "{key}". Did you mean "Duration"? (IFrame Generator RE v1.03 supports only "Duration")"#
                        ),
                        Some(DiagnosticCode::FixIFrameKey),
                    );
                }
            }
        }
    }
}

/// `PIE.@SGVB|x` -> "expects 2 parameters, but 1 provided".
fn pie(ctx: &mut LineCtx, node: &PieNode) {
    let Some(ins) = &node.instruction else { return };
    let Some(name) = &ins.name else { return };
    let Some(def) = pie::find_native(name.value) else { return };

    let provided = ins
        .parameters
        .as_ref()
        .map_or(0, |params| params.items.iter().filter(|item| item.value.is_some()).count());
    let expected = def.param_count();

    if provided < expected {
        let end = ins.parameters.as_ref().map_or(name.span.end, |params| params.span.end);
        ctx.push(
            Span::new(ins.prefix.span.start, end),
            Severity::Error,
            format!(
                "PIE instruction '{}' expects {expected} parameters, but {provided} provided.",
                def.name
            ),
            None,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn messages(text: &str) -> Vec<String> {
        diagnostics(text).into_iter().map(|d| d.message).collect()
    }

    #[test]
    fn valid_document_has_no_diagnostics() {
        let text = "# numOriginalFrames: 38\ntrackName: Root\n0.100000 MCO_DodgeOpen\n0.2 animmotion 0 1 2\n0.3 animrotation 90";
        assert!(diagnostics(text).is_empty());
    }

    #[test]
    fn missing_text() {
        assert_eq!(messages("0.5 "), ["Text annotation is missing."]);
    }

    #[test]
    fn iframe_typo_has_fix() {
        let d = diagnostics(r#"0.1 SpecialFrames_Invincible{"Duratoin":0.5}"#);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code, Some(DiagnosticCode::FixIFrameKey));
        assert_eq!(d[0].range.span, Span::new(28, 44));
    }

    #[test]
    fn pie_param_count() {
        assert_eq!(
            messages("0.1 PIE.@SGVB|x"),
            ["PIE instruction 'SGVB' expects 2 parameters, but 1 provided."]
        );
        assert!(messages("0.1 PIE.@SGVB|x|1").is_empty());
    }

    #[test]
    fn invalid_number() {
        assert_eq!(messages("0.1 animrotation 9.0.0"), ["Invalid number."]);
    }
}
