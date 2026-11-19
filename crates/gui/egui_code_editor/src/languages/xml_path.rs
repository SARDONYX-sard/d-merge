//! The path of XML elements to a line, for [`crate::Breadcrumbs`].
//!
//! ```text
//! <hkobject name="#0100" class="hkbStateMachine">   -> `#0100 hkbStateMachine`
//!     <hkparam name="states" numelements="2">        -> `states`
//!         <hkobject>                                 -> `hkobject`
//!             <hkparam name="toStateId">1</hkparam>  -> `toStateId`   <- the line asked for
//! ```
//!
//! A part is named by its `name` attribute, with its `class` if it has one (Havok XML's objects and
//! fields), else by its tag. Each tag is taken to be on one line, as in Havok XML.

use crate::views::breadcrumbs::Crumb;

/// A tag found in a line. e.g. `<hkparam name="a">` at char 3
struct Tag<'a> {
    /// e.g. `hkparam`
    name: &'a str,
    /// From after the name to the `>`, e.g. ` name="a"`.
    attributes: &'a str,
    /// The char its `<` is at in the line.
    column: usize,
    kind: TagKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TagKind {
    Open,
    Close,
    /// `<a/>`
    SelfClosed,
}

/// The tags of `line` in order, without comments and declarations (`<!--`, `<?xml`).
fn tags(line: &str) -> Vec<Tag<'_>> {
    let mut tags = Vec::new();
    let mut rest = 0;
    while let Some(found) = line[rest..].find('<') {
        let at = rest + found;
        let after = &line[at + 1..];
        if after.starts_with("!--") {
            let Some(end) = after.find("-->") else { break };
            rest = at + 1 + end + 3;
            continue;
        }
        let Some(end) = after.find('>') else { break };
        rest = at + 1 + end + 1;

        let inner = &after[..end];
        let (kind, inner) = match inner.strip_prefix('/') {
            Some(inner) => (TagKind::Close, inner),
            None if inner.ends_with('/') => (TagKind::SelfClosed, &inner[..inner.len() - 1]),
            None => (TagKind::Open, inner),
        };
        let name_len = inner.find(char::is_whitespace).unwrap_or(inner.len());
        let name = &inner[..name_len];
        if name.is_empty() || name.starts_with(['?', '!']) {
            continue;
        }
        let column = line[..at].chars().count();
        tags.push(Tag { name, attributes: &inner[name_len..], column, kind });
    }
    tags
}

/// The value of attribute `key` of `attributes`. e.g. ` name="a" class="b"`, `class` -> `b`
fn attribute<'a>(attributes: &'a str, key: &str) -> Option<&'a str> {
    let mut rest = attributes;
    while let Some(at) = rest.find(key) {
        let after = &rest[at + key.len()..];
        let is_whole = at == 0 || rest[..at].ends_with(char::is_whitespace);
        if is_whole && let Some(value) = after.strip_prefix("=\"") {
            return value.split_once('"').map(|(value, _)| value);
        }
        rest = after;
    }
    None
}

/// What a part shows. See the module docs.
fn label(tag: &Tag<'_>) -> String {
    match (attribute(tag.attributes, "name"), attribute(tag.attributes, "class")) {
        (Some(name), Some(class)) => format!("{name} {class}"),
        (Some(name), None) => name.to_owned(),
        (None, _) => tag.name.to_owned(),
    }
}

/// The elements open at 0-based `line` of `text`, outermost first: the path to it. The element the
/// line starts (or ends) is the last.
pub fn xml_path(text: &str, line: usize) -> Vec<Crumb> {
    let mut open: Vec<(&str, Crumb)> = Vec::new();
    for (line_no, source) in text.lines().enumerate().take(line) {
        for tag in tags(source) {
            match tag.kind {
                TagKind::Open => open.push((tag.name, Crumb { label: label(&tag), line: line_no, column: tag.column })),
                TagKind::Close => {
                    if let Some(at) = open.iter().rposition(|(name, _)| *name == tag.name) {
                        open.truncate(at);
                    }
                }
                TagKind::SelfClosed => {}
            }
        }
    }

    // The line itself: an element it starts is the last part; its end tag ends the last one there.
    if let Some(source) = text.lines().nth(line)
        && let Some(first) = tags(source).into_iter().find(|tag| tag.kind != TagKind::Close)
        && !source.trim_start().starts_with("</")
    {
        open.push((first.name, Crumb { label: label(&first), line, column: first.column }));
    }
    open.into_iter().map(|(_, crumb)| crumb).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEXT: &str = "\t\t<hkobject name=\"#0100\" class=\"hkbStateMachine\" signature=\"0x1\">
\t\t\t<!-- a note -->
\t\t\t<hkparam name=\"states\" numelements=\"1\">
\t\t\t\t<hkobject>
\t\t\t\t\t<hkparam name=\"toStateId\">1</hkparam>
\t\t\t\t\t<hkparam name=\"payload\"/>
\t\t\t\t</hkobject>
\t\t\t</hkparam>
\t\t</hkobject>
";

    fn labels(line: usize) -> Vec<(String, usize, usize)> {
        xml_path(TEXT, line).into_iter().map(|crumb| (crumb.label, crumb.line, crumb.column)).collect()
    }

    #[test]
    fn a_line_has_the_path_of_the_elements_around_it() {
        let to_state = labels(4);
        let expected = [
            ("#0100 hkbStateMachine".to_owned(), 0, 2),
            ("states".to_owned(), 2, 3),
            ("hkobject".to_owned(), 3, 4),
            ("toStateId".to_owned(), 4, 5),
        ];
        assert_eq!(to_state, expected);
        assert_eq!(labels(5).len(), 4, "a self-closed one is the last part of its own line");
        assert_eq!(labels(6), expected[..3], "its end tag: the element it ends");
        assert_eq!(labels(1), expected[..1], "a comment is no element");
        assert!(xml_path(TEXT, 9).is_empty(), "after it all");
    }

    #[test]
    fn an_attribute_is_found_by_its_whole_key() {
        assert_eq!(attribute(" classname=\"x\" class=\"y\"", "class"), Some("y"));
        assert_eq!(attribute(" name=\"a\"", "class"), None);
    }
}
