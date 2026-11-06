//! `path="#2802/transitions/Element0/enterEventId"`: finding it in a line, and what it leads to.
//!
//! No attribute of these files spans lines, so a line is enough to know the path under the cursor.

use std::ops::Range;

use crate::tree::{Element, parse};

/// The value of a `path` attribute in a line, and the cursor in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathAt<'a> {
    /// e.g. `#2802/transitions/Element0`
    pub path: &'a str,
    /// The part up to the cursor. e.g. `#2802/trans`
    pub typed: &'a str,
    /// The char columns of the part the cursor is in, in the line. e.g. those of `transitions`
    pub part_columns: Range<usize>,
}

impl<'a> PathAt<'a> {
    /// What the parts before the one being typed are.
    ///
    /// e.g. `#2802/transitions/Elem` -> `#2802/transitions`; `#28` -> `None`
    #[must_use]
    pub fn parent(&self) -> Option<&'a str> {
        self.typed.rsplit_once('/').map(|(parent, _)| parent)
    }

    /// The part the cursor is in, whole. e.g. `transitions`
    #[must_use]
    pub fn part_under_cursor(&self) -> &'a str {
        let start = self.typed.rfind('/').map_or(0, |at| at + 1);
        let end = self.path[self.typed.len()..].find('/').map_or(self.path.len(), |at| self.typed.len() + at);
        &self.path[start..end]
    }

    /// The path up to the end of the part the cursor is in. e.g. `#2802/transitions`
    #[must_use]
    pub fn up_to_part(&self) -> &'a str {
        let end = self.path[self.typed.len()..].find('/').map_or(self.path.len(), |at| self.typed.len() + at);
        &self.path[..end]
    }
}

/// The `path` value that char `column` of `line` is in, also while its closing `"` isn't typed yet.
#[must_use]
pub fn path_at(line: &str, column: usize) -> Option<PathAt<'_>> {
    let (value_start, value_end) = value_range(line)?;
    let at = line.char_indices().nth(column).map_or(line.len(), |(at, _)| at);
    if !(value_start..=value_end).contains(&at) {
        return None;
    }
    let (path, typed) = (&line[value_start..value_end], &line[value_start..at]);
    let part_start = typed.rfind('/').map_or(value_start, |slash| value_start + slash + 1);
    let part_end = path[typed.len()..].find('/').map_or(value_end, |slash| at + slash);
    let column_of = |byte: usize| line[..byte].chars().count();
    Some(PathAt { path, typed, part_columns: column_of(part_start)..column_of(part_end) })
}

/// The bytes of the value of the `path` attribute of `line`.
fn value_range(line: &str) -> Option<(usize, usize)> {
    const KEY: &str = "path=\"";
    let start = line.find(KEY)? + KEY.len();
    let end = line[start..].find('"').map_or(line.len(), |length| start + length);
    Some((start, end))
}

/// The value of the `path` attribute of `line`. e.g. `#0001/a`
#[must_use]
pub fn path_of_line(line: &str) -> Option<&str> {
    value_range(line).map(|(start, end)| &line[start..end])
}

/// `#2802/transitions/Element0` -> `#2802` and `transitions`, `Element0`
pub fn split_path(path: &str) -> (&str, impl Iterator<Item = &str>) {
    let (id, keys) = path.split_once('/').unwrap_or((path, ""));
    (id, keys.split('/').filter(|key| !key.is_empty()))
}

/// What `keys` lead to from `object`.
#[must_use]
pub fn resolve<'e, 'a, 'k>(
    object: &'e Element<'a>,
    keys: impl IntoIterator<Item = &'k str>,
) -> Option<&'e Element<'a>> {
    keys.into_iter().try_fold(object, |element, key| element.child(key))
}

/// The 0-based line of `element` in `text`, the text it was read from. `None`: it wasn't.
#[must_use]
pub fn line_of_element(text: &str, element: &Element<'_>) -> Option<usize> {
    let offset = (element.tag.as_ptr() as usize).checked_sub(text.as_ptr() as usize)?;
    let before = text.get(..offset)?;
    Some(before.bytes().filter(|byte| *byte == b'\n').count())
}

/// The 0-based line in `text` of what `path` leads to in `object`, which was read from `text`.
///
/// Or of the nearest level above it that is there: the path of a `Remove` is about an element that is gone.
/// e.g. `#0001/list/Element3` of a list with 3 elements -> the line of `list`.
#[must_use]
pub fn line_of_path(text: &str, object: &Element<'_>, path: &str) -> Option<usize> {
    let keys: Vec<&str> = split_path(path).1.collect();
    let element = (0..=keys.len()).rev().find_map(|depth| resolve(object, keys[..depth].iter().copied()))?;
    line_of_element(text, element)
}

/// Something wrong in a patch file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocProblem {
    /// 0-based.
    pub line: usize,
    /// In chars. Empty: the whole line says it.
    pub columns: Range<usize>,
    pub kind: DocProblemKind,
}

/// What is wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocProblemKind {
    /// The file isn't XML, so Pandora reads nothing of it.
    NotWellFormed,
    /// `#0106` alone: Pandora needs a `/`, and skips it.
    NoField,
    /// No object has this id.
    NoSuchObject(String),
    /// The object has nothing at this path. It may be there at that time if an operation before this
    /// one adds it, so this isn't certain.
    NoSuchPath(String),
}

impl std::fmt::Display for DocProblemKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotWellFormed => f.write_str("This file isn't well formed XML: Pandora would read nothing of it."),
            Self::NoField => f.write_str("A path needs a field: `#0106/field`. Pandora skips this one."),
            Self::NoSuchObject(id) => write!(f, "No object has the id {id}."),
            Self::NoSuchPath(path) => {
                write!(f, "Nothing is at {path}, unless an operation before this one adds it.")
            }
        }
    }
}

/// What is wrong in the patch file `doc`. `object_text`: the text of a template's or the work's object.
#[must_use]
pub fn doc_problems<'t>(doc: &str, object_text: impl Fn(&str) -> Option<&'t str>) -> Vec<DocProblem> {
    let Some(root) = parse(doc) else {
        return vec![DocProblem { line: 0, columns: 0..0, kind: DocProblemKind::NotWellFormed }];
    };
    // Objects this file inserts can be pointed to as well.
    let inserted: Vec<&Element<'_>> =
        root.children.iter().filter(|op| op.attribute("path") == Some("")).flat_map(|op| &op.children).collect();

    let mut problems = Vec::new();
    for (line_no, line) in doc.lines().enumerate() {
        let Some(path) = path_of_line(line).filter(|path| !path.is_empty()) else { continue };
        let at = line.find(path).unwrap_or_default();
        let start = line[..at].chars().count();
        let columns = start..start + path.chars().count();
        if let Some(kind) = path_problem(path, &inserted, &object_text) {
            problems.push(DocProblem { line: line_no, columns, kind });
        }
    }
    problems
}

fn path_problem<'t>(
    path: &str,
    inserted: &[&Element<'_>],
    object_text: &impl Fn(&str) -> Option<&'t str>,
) -> Option<DocProblemKind> {
    if !path.contains('/') {
        return Some(DocProblemKind::NoField);
    }
    let (id, keys) = split_path(path);
    let found = if let Some(object) = inserted.iter().find(|object| object.name() == Some(id)) {
        resolve(object, keys).is_some()
    } else {
        let Some(text) = object_text(id) else {
            return Some(DocProblemKind::NoSuchObject(id.to_owned()));
        };
        // An object that isn't well formed has its own place to say so.
        parse(text).is_none_or(|object| resolve(&object, keys).is_some())
    };
    (!found).then(|| DocProblemKind::NoSuchPath(path.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_path_under_the_cursor_is_found() {
        let line = r##"  <Replace path="#2802/transitions/Element0">"##;
        let column_of = |text: &str| line.find(text).unwrap();

        let at = path_at(line, column_of("trans") + 2).unwrap();
        assert_eq!(at.path, "#2802/transitions/Element0");
        assert_eq!(at.typed, "#2802/tr");
        assert_eq!(at.parent(), Some("#2802"));
        assert_eq!(at.part_under_cursor(), "transitions");
        assert_eq!(at.up_to_part(), "#2802/transitions");
        let start = column_of("transitions");
        assert_eq!(at.part_columns, start..start + "transitions".len());

        let on_id = path_at(line, column_of("#2802") + 1).unwrap();
        assert_eq!(on_id.parent(), None);
        assert_eq!(on_id.part_under_cursor(), "#2802");
        assert_eq!(on_id.up_to_part(), "#2802");

        assert_eq!(path_at(line, 3), None, "on the tag");
        assert_eq!(path_at("<hkparam name=\"a\">", 3), None);
    }

    #[test]
    fn a_path_is_found_while_it_is_being_typed() {
        let line = r##"<Append path="#0001/"##;
        let at = path_at(line, line.len()).unwrap();
        assert_eq!(at.parent(), Some("#0001"));
        assert_eq!(at.part_columns, line.len()..line.len());
    }

    #[test]
    fn columns_are_in_chars() {
        let line = r##"<!-- あ --><Remove path="#1/a" />"##;
        // The `a` of the path, not of `path`.
        let start = line.chars().position(|c| c == '#').unwrap() + "#1/".len();
        let at = path_at(line, start).unwrap();
        assert_eq!(at.part_under_cursor(), "a");
        assert_eq!(at.part_columns, start..start + 1);
    }

    const OBJECT: &str = "<hkobject name=\"#0001\" class=\"c\">\n\t<hkparam name=\"a\">1</hkparam>\n\t<hkparam name=\"l\" numelements=\"1\">\n\t\t<hkobject>\n\t\t\t<hkparam name=\"x\">2</hkparam>\n\t\t</hkobject>\n\t</hkparam>\n</hkobject>";

    #[test]
    fn a_path_that_is_gone_leads_to_the_level_above_it() {
        let object = parse(OBJECT).unwrap();
        assert_eq!(line_of_path(OBJECT, &object, "#0001/l/Element0/x"), Some(4));
        assert_eq!(line_of_path(OBJECT, &object, "#0001/l/Element3"), Some(2));
        assert_eq!(line_of_path(OBJECT, &object, "#0001/none"), Some(0));
    }

    #[test]
    fn a_path_leads_to_its_element_and_line() {
        let object = parse(OBJECT).unwrap();
        let (id, keys) = split_path("#0001/l/Element0/x");
        assert_eq!(id, "#0001");
        let element = resolve(&object, keys).unwrap();
        assert_eq!(element.words, ["2"]);
        assert_eq!(line_of_element(OBJECT, element), Some(4));
        assert_eq!(line_of_element(OBJECT, &object), Some(0));
        assert_eq!(resolve(&object, ["none"]), None);
    }

    #[test]
    fn what_is_wrong_in_a_file_is_told_with_its_place() {
        let doc = "<m>\n\
<Insert path=\"\">\n\
<hkobject name=\"#m$0\" class=\"c\"><hkparam name=\"q\">1</hkparam></hkobject>\n\
</Insert>\n\
<Replace path=\"#0001/a\"><hkparam name=\"a\">2</hkparam></Replace>\n\
<Replace path=\"#m$0/q\"><hkparam name=\"q\">2</hkparam></Replace>\n\
<Replace path=\"#0001/none\"><hkparam name=\"none\">2</hkparam></Replace>\n\
<Replace path=\"#9999/a\"><hkparam name=\"a\">2</hkparam></Replace>\n\
<Remove path=\"#0001\" />\n\
</m>";
        let problems = doc_problems(doc, |id| (id == "#0001").then_some(OBJECT));
        let kinds: Vec<_> = problems.iter().map(|problem| (problem.line, problem.kind.clone())).collect();
        assert_eq!(
            kinds,
            [
                (6, DocProblemKind::NoSuchPath("#0001/none".to_owned())),
                (7, DocProblemKind::NoSuchObject("#9999".to_owned())),
                (8, DocProblemKind::NoField),
            ]
        );
        let start = "<Replace path=\"".len();
        assert_eq!(problems[0].columns, start..start + "#0001/none".len());
    }

    #[test]
    fn a_file_that_is_not_xml_is_told() {
        let problems = doc_problems("<m><a></m>", |_| None);
        assert_eq!(problems[0].kind, DocProblemKind::NotWellFormed);
    }
}
