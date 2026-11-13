//! Toggling comments on lines, as VS Code's `Ctrl+/` and Vim's `gcc` / `gc{motion}` (commentary).
//!
//! | lines                      | becomes                                        |
//! | -------------------------- | ---------------------------------------------- |
//! | some not commented         | all commented, at the least indented column    |
//! | all commented              | all uncommented (a space after the mark too)   |
//! | blank                      | left as they are                               |
//!
//! e.g. with `//`: `  a\n    b` -> `  // a\n  //   b`, and back.
//! With `<!-- -->` (XML) each line is wrapped: `  <a/>` -> `  <!-- <a/> -->`.

use crate::Comment;

/// What a toggle did to one line: `width` chars were put in (`> 0`) or taken out (`< 0`) at
/// `column`, and (block comments) at its end. For the cursor to follow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LineEdit {
    pub(crate) line: usize,
    pub(crate) column: usize,
    pub(crate) width: isize,
}

/// Toggles the comments of lines `first..=last` of `text`. `None`: nothing to toggle (all blank).
pub(crate) fn toggle(text: &str, first: usize, last: usize, comment: Comment) -> Option<(String, Vec<LineEdit>)> {
    let lines: Vec<&str> = text.split('\n').collect();
    let last = last.min(lines.len().saturating_sub(1));
    let range = first..=last;
    let filled = || range.clone().filter(|&at| !lines[at].trim().is_empty());
    filled().next()?; // all blank
    let uncomment = filled().all(|at| commented(lines[at], comment));
    let indent = filled().map(|at| indent_of(lines[at])).min().unwrap_or(0);
    let mut edits = Vec::new();
    let mut out = String::with_capacity(text.len() + (last + 1 - first) * 8);
    for (at, line) in lines.iter().enumerate() {
        if at > 0 {
            out.push('\n');
        }
        if !range.contains(&at) || line.trim().is_empty() {
            out.push_str(line);
            continue;
        }
        let (changed, column, width) = if uncomment { remove(line, comment) } else { add(line, indent, comment) };
        out.push_str(&changed);
        edits.push(LineEdit { line: at, column, width });
    }
    Some((out, edits))
}

/// The chars of indentation of `line`.
fn indent_of(line: &str) -> usize {
    line.chars().take_while(|c| c.is_whitespace()).count()
}

fn commented(line: &str, comment: Comment) -> bool {
    let line = line.trim();
    match comment {
        Comment::Line(mark) => line.starts_with(mark),
        Comment::Block(open, close) => line.starts_with(open) && line.ends_with(close),
    }
}

/// `line` with the mark put in at char `indent`: `(line, column, chars put in)`.
fn add(line: &str, indent: usize, comment: Comment) -> (String, usize, isize) {
    let at: usize = line.chars().take(indent).map(char::len_utf8).sum();
    let (head, rest) = line.split_at(at);
    let (changed, width) = match comment {
        Comment::Line(mark) => (format!("{head}{mark} {rest}"), mark.chars().count() + 1),
        Comment::Block(open, close) => (format!("{head}{open} {rest} {close}"), open.chars().count() + 1),
    };
    (changed, indent, isize::try_from(width).unwrap_or(0))
}

/// `line` with its mark taken out (and the space after it): `(line, column, -chars taken out)`.
fn remove(line: &str, comment: Comment) -> (String, usize, isize) {
    let indent = indent_of(line);
    let at: usize = line.chars().take(indent).map(char::len_utf8).sum();
    let (head, rest) = line.split_at(at);
    let (open, close) = match comment {
        Comment::Line(mark) => (mark, ""),
        Comment::Block(open, close) => (open, close),
    };
    let body = rest.strip_prefix(open).unwrap_or(rest);
    let body = body.trim_end();
    let body = body.strip_suffix(close).unwrap_or(body);
    let body = if close.is_empty() { body } else { body.strip_suffix(' ').unwrap_or(body) };
    let (body, space) = body.strip_prefix(' ').map_or((body, 0), |body| (body, 1));
    let width = open.chars().count() + space;
    (format!("{head}{body}"), indent, -isize::try_from(width).unwrap_or(0))
}

/// Where `column` of `line` is after `edits`. e.g. `//` put in at column 2: column 4 -> 7
pub(crate) fn follow(line: usize, column: usize, edits: &[LineEdit]) -> usize {
    let Some(edit) = edits.iter().find(|edit| edit.line == line) else { return column };
    if column < edit.column {
        return column;
    }
    column.saturating_add_signed(edit.width).max(edit.column)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SLASHES: Comment = Comment::Line("//");
    const XML: Comment = Comment::Block("<!--", "-->");

    #[test]
    fn lines_are_commented_at_the_least_indent_and_back() {
        let text = "  a\n\n    b\nc";
        let (commented, edits) = toggle(text, 0, 2, SLASHES).unwrap();
        assert_eq!(commented, "  // a\n\n  //   b\nc");
        assert_eq!(edits.len(), 2, "the blank line is left");
        assert_eq!(follow(2, 4, &edits), 7);
        let (back, _) = toggle(&commented, 0, 2, SLASHES).unwrap();
        assert_eq!(back, text);
    }

    #[test]
    fn some_not_commented_are_all_commented() {
        let (out, _) = toggle("// a\nb", 0, 1, SLASHES).unwrap();
        assert_eq!(out, "// // a\n// b");
        assert_eq!(toggle("\n  \n", 0, 2, SLASHES), None, "all blank");
    }

    #[test]
    fn xml_lines_are_wrapped() {
        let text = "\t<a/>\n\t<b/>";
        let (out, _) = toggle(text, 0, 1, XML).unwrap();
        assert_eq!(out, "\t<!-- <a/> -->\n\t<!-- <b/> -->");
        assert_eq!(toggle(&out, 0, 1, XML).unwrap().0, text);
        assert_eq!(toggle("<!--a-->", 0, 0, XML).unwrap().0, "a", "without the spaces too");
    }
}
