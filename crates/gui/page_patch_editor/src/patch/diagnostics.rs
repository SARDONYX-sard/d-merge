//! Checks of an object's text, shown as underlines in the editor.

use egui_code_editor::{Diagnostic, Severity};

use super::workspace::Behavior;

/// [`nemesis_diff::check_fields`] (names) and [`nemesis_diff::schema::check_pointers`] (pointer
/// targets: they exist and have the right class) as editor underlines (the trimmed line).
///
/// `is_new`: also the fields it lacks. A new object is written as is, so it must be complete,
/// while an edited template object starts complete and its patch only holds the changes.
pub(super) fn field_diagnostics(text: &str, behavior: &Behavior, is_new: bool) -> Vec<Diagnostic> {
    let lines: Vec<&str> = text.lines().collect();
    let class_of = |id: &str| behavior.class_of(id);
    let names = nemesis_diff::check_fields(text).into_iter().map(|p| (p.line, p.message, Severity::Warning));
    let pointers =
        nemesis_diff::schema::check_pointers(text, &class_of).into_iter().map(|p| (p.line, p.message, Severity::Error));
    let missing = if is_new { nemesis_diff::schema::missing_fields(text) } else { Vec::new() };
    let missing = missing.into_iter().map(|field| (0, format!("Missing field `{field}`."), Severity::Warning));
    names
        .chain(pointers)
        .chain(missing)
        .map(|(line_no, message, severity)| {
            let line = lines.get(line_no).copied().unwrap_or_default();
            let start = line.chars().take_while(|c| c.is_whitespace()).count();
            Diagnostic { line: line_no, columns: start..line.chars().count(), severity, message }
        })
        .collect()
}
