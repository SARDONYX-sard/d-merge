//! Vim's `%` between the start and the end tag of an element, as Vim's `matchit` plugin does.
//!
//! ```text
//! <hkparam name="states" numelements="2">   <- `%` on this tag (or after the cursor on its line)
//!     <hkparam name="x">1</hkparam>            ...a nested one of the same name is skipped
//! </hkparam>                                <- goes here, and back
//! ```
//!
//! A `<` that starts no tag (`a < b`) is not one: `%` looks further for a bracket. Comments
//! (`<!--`), declarations (`<?xml`) and self-closed tags (`<a/>`) have no pair.

use super::buffer::Buffer;

/// A tag of the text: where its `<` is, and its name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Tag<'b> {
    /// The index of its `<`.
    at: usize,
    /// e.g. `hkparam`
    name: &'b [char],
    /// `</hkparam>`
    closing: bool,
}

/// The tag whose `<` is at `at`, if it is one that has a pair. e.g. `<hkparam name="a">` ->
/// `hkparam`; `</hkparam>` -> `hkparam`, closing; `<a/>`, `<!--`, `< b` -> `None`
fn tag_at(buf: &Buffer, at: usize) -> Option<Tag<'_>> {
    let chars = &buf.chars;
    if chars.get(at) != Some(&'<') {
        return None;
    }
    let closing = chars.get(at + 1) == Some(&'/');
    let start = at + 1 + usize::from(closing);
    let is_name = |c: &char| c.is_alphanumeric() || matches!(c, '_' | '-' | ':' | '.');
    let len = chars[start..].iter().take_while(|c| is_name(c)).count();
    if len == 0 || !chars[start].is_alphabetic() && chars[start] != '_' {
        return None;
    }

    // Self-closed (`/>` before its `>`): nothing to go to.
    let end = chars[start..].iter().position(|&c| c == '>').map(|i| start + i)?;
    if !closing && chars[end - 1] == '/' {
        return None;
    }
    Some(Tag { at, name: &chars[start..start + len], closing })
}

/// The tag the cursor `cur` is in on its line (from its `<` to its `>`), or else the first tag after
/// it on its line before any bracket. `None`: a bracket comes first, or there is no tag.
pub(super) fn tag_near(buf: &Buffer, cur: usize) -> Option<usize> {
    let (start, end) = (buf.line_start(cur), buf.line_end(cur));
    // In a tag: its `<` is before the cursor with no `>` between (the cursor may be on the `>`).
    for j in (start..=cur.min(end.saturating_sub(1))).rev() {
        match buf.chars[j] {
            '>' if j < cur => break,
            '<' => {
                if tag_at(buf, j).is_some() {
                    return Some(j);
                }
                break;
            }
            _ => {}
        }
    }
    for j in cur..end {
        match buf.chars[j] {
            '(' | ')' | '[' | ']' | '{' | '}' => return None,
            '<' if tag_at(buf, j).is_some() => return Some(j),
            _ => {}
        }
    }
    None
}

/// The `<` of the tag that pairs with the tag at `at`: its end tag, or its start tag. Nested
/// elements of the same name are counted.
pub(super) fn matching_tag(buf: &Buffer, at: usize) -> Option<usize> {
    let tag = tag_at(buf, at)?;
    let mut depth = 0_usize;
    let mut step = |j: usize| -> Option<Option<usize>> {
        let other = tag_at(buf, j).filter(|other| other.name == tag.name)?;
        if other.closing == tag.closing {
            depth += 1;
            return Some(None);
        }
        if depth == 0 {
            return Some(Some(j));
        }
        depth -= 1;
        Some(None)
    };
    if tag.closing {
        (0..at).rev().filter(|&j| buf.chars[j] == '<').find_map(|j| step(j).flatten())
    } else {
        (at + 1..buf.chars.len()).filter(|&j| buf.chars[j] == '<').find_map(|j| step(j).flatten())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const XML: &str = "<a n=\"1\">\n\t<a>x</a>\n\t<b/>\n</a>\n";

    #[test]
    fn a_start_tag_goes_to_its_end_tag_and_back() {
        let buf = Buffer::new(XML);
        let close = XML.rfind("</a>").unwrap();
        assert_eq!(matching_tag(&buf, 0), Some(close), "the nested `<a>` is skipped");
        assert_eq!(matching_tag(&buf, close), Some(0));

        let inner = XML.find("\t<a>").unwrap() + 1;
        assert_eq!(matching_tag(&buf, inner), Some(XML.find("</a>").unwrap()), "on one line");
        assert_eq!(matching_tag(&buf, XML.find("<b/>").unwrap()), None, "self-closed");
    }

    #[test]
    fn the_cursor_is_in_a_tag_or_before_one() {
        let buf = Buffer::new(XML);
        assert_eq!(tag_near(&buf, 3), Some(0), "in `<a n=\"1\">`");
        assert_eq!(tag_near(&buf, 8), Some(0), "on its `>`");
        let inner = XML.find("\t<a>").unwrap();
        assert_eq!(tag_near(&buf, inner), Some(inner + 1), "after the cursor");

        let text = Buffer::new("a < b (c)");
        assert_eq!(tag_near(&text, 0), None, "`< b` is no tag; the bracket comes first");
        assert_eq!(tag_near(&Buffer::new("<!-- x -->"), 0), None);
    }
}
