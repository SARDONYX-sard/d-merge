//! Checks of an object's text, shown as underlines in the editor.

use egui_code_editor::{Diagnostic, Severity};

use super::workspace::Behavior;

/// [`nemesis_diff::check_fields`] (names) and [`nemesis_diff::schema::check_pointers`] (pointer
/// targets: they exist and have the right class) as editor underlines (the trimmed line).
///
/// `is_new`: also the fields it lacks. A new object is written as is, so it must be complete,
/// while an edited template object starts complete and its patch only holds the changes.
///
/// `code`: this work's mod code. Pointers to new objects of other mods are not looked for.
pub(super) fn field_diagnostics(text: &str, behavior: &Behavior, is_new: bool, code: &str) -> Vec<Diagnostic> {
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

/// Where d-merge's parser rejected a patch. See [`parser_problem`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ParserProblem {
    /// 0-based line of the patch.
    pub(super) line: usize,
    /// 0-based column (chars).
    pub(super) column: usize,
    pub(super) message: String,
}

/// Reads the parser's error text, which is drawn for a terminal. There are two ways of drawing it:
/// ```text
/// error:
///   |
/// 3 |   <hkparam name="typoField">null</hkparam>
///   |   - Unknown field names: "typoField".
/// ```
/// -> line 2, column 2, `Unknown field names: "typoField".`
/// ```text
/// parse error at line 2, column 1
///   |
/// 2 | abc
///   | ^
/// expected triggers_len: usize
/// ```
/// -> line 1, column 0, `expected triggers_len: usize`. `None`: another kind of error.
pub(super) fn parser_problem(error: &str) -> Option<ParserProblem> {
    let lines: Vec<&str> = error.lines().collect();
    (0..lines.len().saturating_sub(1)).find_map(|at| {
        let (number, _) = lines[at].split_once(" | ")?;
        let line = number.trim().parse::<usize>().ok()?.checked_sub(1)?;
        let (_, marker) = lines[at + 1].split_once("| ")?;
        let column = marker.chars().take_while(|c| *c == ' ').count();
        let marker = marker.trim_start();
        let message = match marker.strip_prefix('-') {
            Some(message) => message.trim().to_owned(),
            None if marker.starts_with('^') => lines[at + 2..].join(" ").trim().to_owned(),
            None => return None,
        };
        Some(ParserProblem { line, column, message })
    })
}

/// `problems` with the parser's `error` added, wherever it says: it goes to the line `to_editor_line`
/// tells for a line of the patch. An error that says no place is on the first line: there is no other
/// place to show why a patch was rejected.
pub(super) fn add_parser_error(
    problems: &mut Vec<Diagnostic>,
    text: &str,
    error: &str,
    to_editor_line: impl FnOnce(usize) -> Option<usize>,
) {
    match parser_problem(error) {
        Some(problem) => {
            let line = to_editor_line(problem.line).unwrap_or(problem.line);
            add_parser_problem(problems, text, line, &problem);
        }
        None => problems
            .insert(0, Diagnostic { line: 0, columns: 0..0, severity: Severity::Error, message: error.to_owned() }),
    }
}

/// `problems` with the parser's error added at `line` of the editor text. It says more than the
/// warnings of that line, which it replaces: the same mistake should not be listed twice.
fn add_parser_problem(problems: &mut Vec<Diagnostic>, text: &str, line: usize, problem: &ParserProblem) {
    let source = text.lines().nth(line).unwrap_or_default();
    let (start, end) = (problem.column.min(source.chars().count()), source.chars().count());
    problems.retain(|other| other.line != line);
    problems.push(Diagnostic {
        line,
        columns: start..end,
        severity: Severity::Error,
        message: problem.message.clone(),
    });
    problems.sort_by_key(|problem| problem.line);
}

#[cfg(test)]
mod tests {
    use super::*;

    const ERROR: &str = "error: \n  |\n3 |   <hkparam name=\"typoField\">null</hkparam>\n  |   - Unknown field.";

    #[test]
    fn reads_the_place_of_a_parser_error() {
        let problem = parser_problem(ERROR).unwrap();
        assert_eq!(problem, ParserProblem { line: 2, column: 2, message: "Unknown field.".to_owned() });
        assert_eq!(parser_problem("[I/O Error] no such file"), None);
    }

    /// The parser of a new animation set draws its errors like this.
    #[test]
    fn reads_the_place_of_an_error_with_a_caret() {
        let error = "parse error at line 2, column 1\n  |\n2 | abc\n  | ^\nexpected triggers_len: usize";
        let expected = "expected triggers_len: usize".to_owned();
        assert_eq!(parser_problem(error), Some(ParserProblem { line: 1, column: 0, message: expected }));
    }

    #[test]
    fn an_error_that_says_no_place_is_still_listed() {
        let mut problems = Vec::new();
        add_parser_error(&mut problems, "a\nb", "[I/O Error] no such file", Some);
        assert_eq!(problems.len(), 1);
        assert_eq!((problems[0].line, problems[0].message.as_str()), (0, "[I/O Error] no such file"));
        assert_eq!(problems[0].severity, Severity::Error);
    }

    #[test]
    fn the_parser_error_goes_to_the_line_of_the_editor() {
        let mut problems = Vec::new();
        add_parser_error(&mut problems, "a\nb\n  <x>\nc", ERROR, |line| Some(line + 1));
        assert_eq!((problems[0].line, problems[0].message.as_str()), (3, "Unknown field."));
    }

    #[test]
    fn the_parser_error_replaces_warnings_of_its_line() {
        let warning = |line| Diagnostic { line, columns: 0..1, severity: Severity::Warning, message: String::new() };
        let mut problems = vec![warning(0), warning(2), warning(5)];
        let problem = parser_problem(ERROR).unwrap();
        add_parser_problem(&mut problems, "a\nb\n  <x>\nc", 2, &problem);

        assert_eq!(problems.iter().map(|p| p.line).collect::<Vec<_>>(), [0, 2, 5]);
        assert_eq!(problems[1].severity, Severity::Error);
        assert_eq!(problems[1].columns, 2..5);
    }
}
