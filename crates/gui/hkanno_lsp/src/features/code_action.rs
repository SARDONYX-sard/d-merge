//! Quick fixes for diagnostics that carry a [`DiagnosticCode`].

use super::diagnostics::{Diagnostic, DiagnosticCode};
use crate::LineSpan;

/// A text replacement the GUI should apply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEdit {
    /// Range to replace.
    pub range: LineSpan,
    /// Replacement text.
    pub new_text: String,
}

/// A fix the user can choose (e.g., from a button next to the diagnostic).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeAction {
    /// Button/menu title.
    pub title: &'static str,
    /// What to change.
    pub edit: TextEdit,
}

/// Returns the quick fix for `diagnostic`, if it has one.
///
/// `text` must be the document the diagnostic was computed from.
pub fn code_action(text: &str, diagnostic: &Diagnostic) -> Option<CodeAction> {
    match diagnostic.code? {
        DiagnosticCode::FixIFrameKey => {
            let line = crate::lines(text).nth(diagnostic.range.line)?;
            let span = diagnostic.range.span;
            let json: String = line.chars().skip(span.start).take(span.len()).collect();

            Some(CodeAction {
                title: r#"Fix IFrame key → "Duration""#,
                edit: TextEdit { range: diagnostic.range, new_text: fix_iframe_key(&json)? },
            })
        }
    }
}

/// Renames every key of the JSON object to `Duration`.
///
/// e.g., `{"Duratoin":0.5}` -> `{"Duration":0.5}`
///
/// When several keys exist, the last value wins (same as the former TS implementation).
pub fn fix_iframe_key(json: &str) -> Option<String> {
    let object = serde_json::from_str::<serde_json::Value>(json.trim()).ok()?;
    let value = object.as_object()?.values().next_back()?.clone();

    let mut fixed = serde_json::Map::new();
    fixed.insert("Duration".to_string(), value);
    serde_json::to_string(&fixed).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::diagnostics::diagnostics;

    #[test]
    fn fixes_typo() {
        let text = r#"0.1 SpecialFrames_Invincible{"Duratoin":0.5}"#;
        let diagnostic = &diagnostics(text)[0];
        let action = code_action(text, diagnostic).expect("quick fix");
        assert_eq!(action.edit.new_text, r#"{"Duration":0.5}"#);
    }
}
