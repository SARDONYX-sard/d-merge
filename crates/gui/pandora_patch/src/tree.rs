//! A small XML reader for what Havok objects are made of: tags, attributes, text and comments.
//!
//! Borrowed from the object's text, as a file has many of them and they are thrown away right after.
//! What is in the text stays as it is: `&lt;` is not unescaped, it only has to be written back.

use std::fmt::Write as _;

/// One element. e.g. `<hkparam name="enable">true</hkparam>`
#[derive(Debug, Clone)]
pub struct Element<'a> {
    /// e.g. `hkparam`
    pub tag: &'a str,
    /// In the order they were written: the first one is what Pandora knows the element by.
    pub attributes: Vec<(&'a str, &'a str)>,
    /// The words of its text. Whitespace between them means nothing in these files.
    ///
    /// e.g. `#0001 #0002` -> `#0001`, `#0002`
    pub words: Vec<&'a str>,
    pub children: Vec<Self>,
    /// Its text in what it was read from, while it is as it was. See [`Self::write_preserved`].
    pub raw: Option<&'a str>,
}

/// What it says, not how it was written.
impl PartialEq for Element<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.tag == other.tag
            && self.attributes == other.attributes
            && self.words == other.words
            && self.children == other.children
    }
}

impl Eq for Element<'_> {}

impl<'a> Element<'a> {
    /// The value of an attribute.
    #[must_use]
    pub fn attribute(&self, name: &str) -> Option<&'a str> {
        self.attributes.iter().find(|(key, _)| *key == name).map(|(_, value)| *value)
    }

    /// What its parent calls it, if not by a position. See [`crate::tree::child_key`].
    #[must_use]
    pub fn name(&self) -> Option<&'a str> {
        self.attributes.first().map(|(_, value)| *value)
    }

    /// The child a part of a path stands for. e.g. `enable`, `Element3`
    #[must_use]
    pub fn child(&self, key: &str) -> Option<&Self> {
        let position = self.child_position(key)?;
        self.children.get(position)
    }

    /// Where in [`Self::children`] the child a part of a path stands for is.
    #[must_use]
    pub fn child_position(&self, key: &str) -> Option<usize> {
        if let Some(index) = key.strip_prefix("Element").and_then(|index| index.parse::<usize>().ok()) {
            let mut unnamed = self.children.iter().enumerate().filter(|(_, child)| child.name().is_none());
            if let Some((position, _)) = unnamed.nth(index) {
                return Some(position);
            }
        }
        self.children.iter().position(|child| child.name() == Some(key))
    }

    /// The parts of a path this element's children can be reached by, in order.
    ///
    /// e.g. `<hkparam name="a"/>` -> `a`; the second `<hkobject>` of a row -> `Element1`
    #[must_use]
    pub fn child_keys(&self) -> Vec<std::borrow::Cow<'a, str>> {
        let mut unnamed = 0;
        self.children
            .iter()
            .map(|child| {
                if let Some(name) = child.name() {
                    std::borrow::Cow::Borrowed(name)
                } else {
                    unnamed += 1;
                    std::borrow::Cow::Owned(format!("Element{}", unnamed - 1))
                }
            })
            .collect()
    }

    /// A copy that doesn't say it is as it was read, also in what it holds.
    #[must_use]
    pub fn without_raw(&self) -> Self {
        Self { raw: None, children: self.children.iter().map(Self::without_raw).collect(), ..self.clone() }
    }

    /// Writes it as [`Self::write_to`], but what is still as it was read stays as it was: a line changes
    /// only where something changed. A list says how many it has now.
    pub fn write_preserved(&self, out: &mut String, level: usize) {
        push_indent(out, level);
        if let Some(raw) = self.raw {
            out.push_str(raw);
            out.push('\n');
            return;
        }
        let _ = write!(out, "<{}", self.tag);
        for (key, value) in &self.attributes {
            match self.count().filter(|_| *key == "numelements") {
                Some(count) => {
                    let _ = write!(out, " {key}=\"{count}\"");
                }
                None => {
                    let _ = write!(out, " {key}=\"{value}\"");
                }
            }
        }
        out.push('>');
        if self.children.is_empty() {
            out.push_str(&self.words.join(" "));
        } else {
            out.push('\n');
            for child in &self.children {
                child.write_preserved(out, level + 1);
            }
            push_indent(out, level);
        }
        let _ = writeln!(out, "</{}>", self.tag);
    }

    /// How many a list has, if that can be told: not of a list of numbers, whose values may be grouped.
    fn count(&self) -> Option<usize> {
        if !self.children.is_empty() {
            return Some(self.children.len());
        }
        let is_pointer = |word: &&str| word.starts_with('#') || word.eq_ignore_ascii_case("null");
        self.words.iter().all(is_pointer).then_some(self.words.len())
    }

    /// Writes it, one element per line, each level `\t` further in.
    pub fn write_to(&self, out: &mut String, level: usize) {
        push_indent(out, level);
        let _ = write!(out, "<{}", self.tag);
        for (key, value) in &self.attributes {
            let _ = write!(out, " {key}=\"{value}\"");
        }
        if self.words.is_empty() && self.children.is_empty() {
            out.push_str("></");
            out.push_str(self.tag);
            out.push_str(">\n");
            return;
        }
        out.push('>');
        if self.children.is_empty() {
            out.push_str(&self.words.join(" "));
        } else {
            out.push('\n');
            if !self.words.is_empty() {
                push_indent(out, level + 1);
                out.push_str(&self.words.join(" "));
                out.push('\n');
            }
            for child in &self.children {
                child.write_to(out, level + 1);
            }
            push_indent(out, level);
        }
        let _ = writeln!(out, "</{}>", self.tag);
    }
}

fn push_indent(out: &mut String, level: usize) {
    for _ in 0..level {
        out.push('\t');
    }
}

/// Reads the first element of `text`. `None`: it isn't well formed.
///
/// e.g. `<hkobject name="#0106" class="hkbStateMachine">...</hkobject>`
#[must_use]
pub fn parse(text: &str) -> Option<Element<'_>> {
    let mut rest = text;
    skip_misc(&mut rest)?;
    element(&mut rest)
}

/// Reads all the elements in a row, e.g. of the root of a patch file, until a closing tag or the end.
#[must_use]
pub fn parse_all(text: &str) -> Option<Vec<Element<'_>>> {
    let mut rest = text;
    let mut elements = Vec::new();
    loop {
        skip_misc(&mut rest)?;
        if rest.is_empty() || rest.starts_with("</") {
            return Some(elements);
        }
        elements.push(element(&mut rest)?);
    }
}

/// Skips whitespace, comments and `<?xml ...?>`. `None`: one of those doesn't end.
fn skip_misc(rest: &mut &str) -> Option<()> {
    loop {
        *rest = rest.trim_start();
        if let Some(comment) = rest.strip_prefix("<!--") {
            *rest = comment.split_once("-->")?.1;
        } else if let Some(declaration) = rest.strip_prefix("<?") {
            *rest = declaration.split_once("?>")?.1;
        } else {
            return Some(());
        }
    }
}

fn element<'a>(rest: &mut &'a str) -> Option<Element<'a>> {
    let start = *rest;
    let inner = rest.strip_prefix('<')?;
    let end = tag_end(inner)?;
    let (tag_text, after) = inner.split_at(end);
    let (self_closing, tag_text) = tag_text.strip_suffix('/').map_or((false, tag_text), |tag_text| (true, tag_text));
    let after = &after[1..]; // '>'

    let (tag, attributes_text) = tag_text.find(char::is_whitespace).map_or((tag_text, ""), |at| tag_text.split_at(at));
    if tag.is_empty() || tag.starts_with('/') {
        return None;
    }
    let attributes = attributes(attributes_text)?;
    let mut element = Element { tag, attributes, words: Vec::new(), children: Vec::new(), raw: None };
    *rest = after;
    if self_closing {
        element.raw = Some(&start[..start.len() - rest.len()]);
        return Some(element);
    }

    loop {
        // Text up to what comes next, which may be a comment in the middle of it.
        let text_end = rest.find('<')?;
        let (text, after_text) = rest.split_at(text_end);
        element.words.extend(text.split_whitespace());
        *rest = after_text;

        if let Some(closing) = rest.strip_prefix("</") {
            let (name, after_closing) = closing.split_once('>')?;
            if name.trim() != tag {
                return None;
            }
            *rest = after_closing;
            element.raw = Some(&start[..start.len() - rest.len()]);
            return Some(element);
        }
        if rest.starts_with("<!--") || rest.starts_with("<?") {
            skip_misc(rest)?;
            continue;
        }
        element.children.push(self::element(rest)?);
    }
}

/// Where the `>` of a tag is: not one in the value of an attribute.
fn tag_end(tag: &str) -> Option<usize> {
    let mut in_value = false;
    tag.char_indices().find_map(|(at, c)| {
        match c {
            '"' => in_value = !in_value,
            '>' if !in_value => return Some(at),
            _ => {}
        }
        None
    })
}

/// ` name="a" class="b"` -> `(name, a)`, `(class, b)`
fn attributes(mut text: &str) -> Option<Vec<(&str, &str)>> {
    let mut attributes = Vec::new();
    loop {
        text = text.trim_start();
        if text.is_empty() {
            return Some(attributes);
        }
        let (key, after) = text.split_once('=')?;
        let (value, after) = after.trim_start().strip_prefix('"')?.split_once('"')?;
        attributes.push((key.trim(), value));
        text = after;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OBJECT: &str = r##"<hkobject name="#0106" class="hkbStateMachine" signature="0x816c1dcb">
	<!-- a comment -->
	<hkparam name="enable">true</hkparam>
	<hkparam name="states" numelements="2">
		#0001
		#0002
	</hkparam>
	<hkparam name="names" numelements="2">
		<hkcstring>a b</hkcstring>
		<hkcstring>c</hkcstring>
	</hkparam>
	<hkparam name="empty"></hkparam>
</hkobject>"##;

    #[test]
    fn an_object_is_read_with_its_parts() {
        let object = parse(OBJECT).unwrap();
        assert_eq!(object.name(), Some("#0106"));
        assert_eq!(object.attribute("class"), Some("hkbStateMachine"));
        assert_eq!(object.children.len(), 4);
        assert_eq!(object.child("states").unwrap().words, ["#0001", "#0002"]);
        assert_eq!(object.child("names").unwrap().child("Element1").unwrap().words, ["c"]);
        assert_eq!(object.child("names").unwrap().child("Element2"), None);
        assert_eq!(object.child("none"), None);
    }

    #[test]
    fn the_keys_of_children_are_names_or_positions() {
        let object = parse(OBJECT).unwrap();
        assert_eq!(object.child_keys(), ["enable", "states", "names", "empty"]);
        assert_eq!(object.child("names").unwrap().child_keys(), ["Element0", "Element1"]);
    }

    #[test]
    fn what_was_written_reads_the_same() {
        let object = parse(OBJECT).unwrap();
        let mut out = String::new();
        object.write_to(&mut out, 0);
        assert_eq!(parse(&out).unwrap(), object);
    }

    #[test]
    fn what_is_not_well_formed_is_none() {
        assert_eq!(parse("<a><b></a>"), None);
        assert_eq!(parse("<a>"), None);
        assert_eq!(parse("no tag"), None);
        assert_eq!(parse("<a x=y></a>"), None);
    }

    #[test]
    fn a_value_may_have_what_closes_a_tag() {
        let element = parse(r#"<a x="1>2"/>"#).unwrap();
        assert_eq!(element.attribute("x"), Some("1>2"));
    }
}
