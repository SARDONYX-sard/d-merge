//! Checks of an object's text, shown as underlines in the editor.

use egui_code_editor::{Diagnostic, Severity};

use super::parser_error::ParserError;
use crate::workspace::Behavior;

/// [`nemesis_diff::check_fields`] (names) and [`nemesis_diff::schema::check_pointers`] (pointer
/// targets: they exist and have the right class) as editor underlines (the trimmed line).
///
/// `is_new`: also the fields it lacks. A new object is written as is, so it must be complete,
/// while an edited template object starts complete and its patch only holds the changes.
///
/// `code`: this work's mod code. Pointers to new objects of other mods are not looked for.
pub fn field_diagnostics(text: &str, behavior: &Behavior, is_new: bool, code: &str) -> Vec<Diagnostic> {
    let lines: Vec<&str> = text.lines().collect();
    let class_of = |id: &str| behavior.class_of(id);
    let names = nemesis_diff::check_fields(text).into_iter().map(|p| (p.line, p.message, Severity::Warning));
    let pointers = nemesis_diff::schema::check_pointers(text, code, &class_of)
        .into_iter()
        .map(|p| (p.line, p.message, Severity::Error));
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

/// The message of [`not_linked`].
pub const NOT_LINKED: &str = "No object links to this new object: Nemesis never uses it.";

/// A warning on the first line (`<hkobject name="#mymod$0" ...>`) of a new object the root does not
/// reach: see [`crate::tree::Graph::unlinked`]. It is written to the patch all the same.
pub fn not_linked(text: &str) -> Diagnostic {
    let line = text.lines().next().unwrap_or_default();
    let start = line.chars().take_while(|c| c.is_whitespace()).count();
    Diagnostic {
        line: 0,
        columns: start..line.chars().count(),
        severity: Severity::Warning,
        message: NOT_LINKED.to_owned(),
    }
}

/// Is `problem` a [`not_linked`] warning?
pub fn is_not_linked(problem: &Diagnostic) -> bool {
    problem.message == NOT_LINKED
}

/// `problems` with the parser's `error` added, wherever it says.
///
/// It goes to the line `to_editor_line` tells for a line of the patch. An error that says no place is
/// on the first line: there is no other place to show why a patch was rejected.
pub fn add_parser_error(
    problems: &mut Vec<Diagnostic>,
    text: &str,
    error: &ParserError,
    to_editor_line: impl FnOnce(usize) -> Option<usize>,
) {
    match error.at {
        Some((line, column)) => {
            let line = to_editor_line(line).unwrap_or(line);
            add_parser_problem(problems, text, (line, column), &error.message);
        }
        None => problems.insert(
            0,
            Diagnostic { line: 0, columns: 0..0, severity: Severity::Error, message: error.message.clone() },
        ),
    }
}

/// `problems` with the parser's error added at `(line, column)` of the editor text. It says more than
/// the warnings of that line, which it replaces: the same mistake should not be listed twice.
fn add_parser_problem(problems: &mut Vec<Diagnostic>, text: &str, (line, column): (usize, usize), message: &str) {
    let source = text.lines().nth(line).unwrap_or_default();
    let (start, end) = (column.min(source.chars().count()), source.chars().count());
    problems.retain(|other| other.line != line);
    problems.push(Diagnostic { line, columns: start..end, severity: Severity::Error, message: message.to_owned() });
    problems.sort_by_key(|problem| problem.line);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The parser's word on `  <x>`, the third line of a patch.
    fn error() -> ParserError {
        ParserError { at: Some((2, 2)), message: "Unknown field.".to_owned() }
    }

    #[test]
    fn a_new_object_nothing_links_to_is_warned_of_at_its_first_line() {
        let problem = not_linked("\t\t<hkobject name=\"#m$0\">\n\t\t</hkobject>");
        assert_eq!((problem.line, problem.columns.clone(), problem.severity), (0, 2..24, Severity::Warning));
        assert!(is_not_linked(&problem));
    }

    #[test]
    fn an_error_that_says_no_place_is_still_listed() {
        let mut problems = Vec::new();
        add_parser_error(&mut problems, "a\nb", &ParserError::without_place("[I/O Error] no such file"), Some);
        assert_eq!(problems.len(), 1);
        assert_eq!((problems[0].line, problems[0].message.as_str()), (0, "[I/O Error] no such file"));
        assert_eq!(problems[0].severity, Severity::Error);
    }

    #[test]
    fn the_parser_error_goes_to_the_line_of_the_editor() {
        let mut problems = Vec::new();
        add_parser_error(&mut problems, "a\nb\n  <x>\nc", &error(), |line| Some(line + 1));
        assert_eq!((problems[0].line, problems[0].message.as_str()), (3, "Unknown field."));
    }

    #[test]
    fn the_parser_error_replaces_warnings_of_its_line() {
        let warning = |line| Diagnostic { line, columns: 0..1, severity: Severity::Warning, message: String::new() };
        let mut problems = vec![warning(0), warning(2), warning(5)];
        add_parser_error(&mut problems, "a\nb\n  <x>\nc", &error(), Some);

        assert_eq!(problems.iter().map(|p| p.line).collect::<Vec<_>>(), [0, 2, 5]);
        assert_eq!(problems[1].severity, Severity::Error);
        assert_eq!(problems[1].columns, 2..5);
    }
}
