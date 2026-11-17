//! The fields of an object's own level, where their values are in its text, and the edits of them
//! that a node graph makes (a value typed, a pointer connected, added or removed).
//!
//! ```text
//! <hkobject name="#0095" class="hkbStateMachine">
//!   <hkparam name="name">Root</hkparam>                     Inline   "Root"
//!   <hkparam name="startStateId">0</hkparam>                Inline   "0"
//!   <hkparam name="states" numelements="2">                 Pointers [#0097, #0098]
//!     #0097
//!     #0098
//!   </hkparam>
//!   <hkparam name="wildcardTransitions">null</hkparam>      Inline   "null" (a pointer by the schema)
//!   <hkparam name="eventNames" numelements="1">             Block    (strings, nested objects: the text's)
//!     <hkcstring>a</hkcstring>
//!   </hkparam>
//! </hkobject>
//! ```
//!
//! Byte offsets are of the text given, so that an edit replaces just what it changes.

use crate::{
    attribute,
    schema::{FieldSchema, holds_pointers, xml_fields},
};

/// A piece of the text: where it starts, and what it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Token<'a> {
    pub at: usize,
    pub text: &'a str,
}

impl Token<'_> {
    #[inline]
    #[must_use]
    pub const fn end(&self) -> usize {
        self.at + self.text.len()
    }
}

/// A field of the object itself (not of an object nested in it).
#[derive(Debug, Clone)]
pub struct ObjectField<'a> {
    pub name: &'a str,
    /// `None`: a field the schema does not know.
    pub schema: Option<&'static FieldSchema>,
    /// 0-based, of its `<hkparam` line.
    pub line: usize,
    /// The byte offset of that line.
    pub at: usize,
    pub value: FieldValue<'a>,
}

impl ObjectField<'_> {
    /// Does it hold a pointer (or pointers), by the schema?
    #[inline]
    #[must_use]
    pub fn holds_pointers(&self) -> bool {
        self.schema.is_some_and(holds_pointers)
    }
}

/// Where a field's value is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldValue<'a> {
    /// `<hkparam name="x">value</hkparam>` on one line: the value, maybe empty.
    Inline(Token<'a>),
    /// An array of pointers, one or more per line.
    Pointers(PointerList<'a>),
    /// Anything else on more lines: `numelements`, if any. Edited in the text.
    Block(Option<&'a str>),
}

/// The elements of an array of pointers, and what adding or removing one changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PointerList<'a> {
    pub elements: Vec<Token<'a>>,
    /// The start of the `</hkparam>` line: a new element goes before it.
    pub close_at: usize,
    /// The value of `numelements="N"`.
    pub count: Option<Token<'a>>,
    /// Of an element's line.
    pub indent: &'a str,
}

/// The object's own fields, in the order of the text. See the module docs.
#[must_use]
pub fn object_fields(object: &str) -> Vec<ObjectField<'_>> {
    let mut class = None;
    let mut objects = 0_usize;
    let mut open: Option<Open<'_>> = None;
    let mut fields = Vec::new();

    let mut at = 0;
    for (line_index, line) in object.split_inclusive('\n').enumerate() {
        let start = at;
        at += line.len();
        let trimmed = line.trim();

        if open.as_ref().is_some_and(|field| field.ends_before(trimmed)) {
            // An empty array after all: this line is the next field's.
            if let Some(field) = open.take() {
                fields.push(field.close(start));
            }
        } else if let Some(field) = open.as_mut() {
            if field.take_line(line, start)
                && let Some(field) = open.take()
            {
                fields.push(field.close(start));
            }
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("<hkobject") {
            objects += 1;
            if objects == 1 {
                class = attribute(rest, "class");
            }
        } else if trimmed.starts_with("</hkobject>") {
            objects = objects.saturating_sub(1);
        } else if objects == 1 && trimmed.starts_with("<hkparam") {
            match param_line(line, start, (class, line_index)) {
                Param::Done(field) => fields.push(field),
                Param::Open(field) => open = Some(field),
            }
        }
    }
    fields
}

/// What a `<hkparam` line is.
enum Param<'a> {
    Done(ObjectField<'a>),
    /// Its value goes on in the next lines.
    Open(Open<'a>),
}

fn param_line<'a>(line: &'a str, start: usize, (class, line_index): (Option<&str>, usize)) -> Param<'a> {
    let name = attribute(line, "name").unwrap_or_default();
    let schema = class.and_then(|class| xml_fields(class).find(|field| field.name == name));
    let count = attribute(line, "numelements").map(|text| Token { at: start + offset_in(line, text), text });

    let tag_end = line.find('>').map_or(line.len(), |end| end + 1);
    if line[..tag_end].ends_with("/>") {
        let value = FieldValue::Block(count.map(|count| count.text));
        return Param::Done(ObjectField { name, schema, line: line_index, at: start, value });
    }
    if let Some(close) = line[tag_end..].find("</hkparam>") {
        let is_kept_array = count.is_some_and(|count| count.text == "0") && line[tag_end..tag_end + close].is_empty();
        if !is_kept_array {
            let value = Token { at: start + tag_end, text: &line[tag_end..tag_end + close] };
            return Param::Done(ObjectField {
                name,
                schema,
                line: line_index,
                at: start,
                value: FieldValue::Inline(value),
            });
        }
        // `numelements="0"></hkparam>`, then the elements and another `</hkparam>`: the text's.
        return Param::Open(Open {
            name,
            schema,
            line: (line_index, start),
            count,
            elements: Vec::new(),
            indent: "",
            simple: false,
            nested: 0,
            maybe_empty: true,
        });
    }
    let simple = schema.is_some_and(holds_pointers);
    Param::Open(Open {
        name,
        schema,
        line: (line_index, start),
        count,
        elements: Vec::new(),
        indent: "",
        simple,
        nested: 0,
        maybe_empty: false,
    })
}

/// A field whose value is on the lines after its `<hkparam`.
struct Open<'a> {
    name: &'a str,
    schema: Option<&'static FieldSchema>,
    /// Its `<hkparam` line, and where it starts.
    line: (usize, usize),
    count: Option<Token<'a>>,
    elements: Vec<Token<'a>>,
    indent: &'a str,
    /// Only pointers so far.
    simple: bool,
    /// How many `<hkparam`s of nested objects are open: their `</hkparam>` is not this one's.
    ///
    /// e.g. `variableInfos` of `hkbVariableInfo`s, each with `<hkparam name="role">` on lines
    nested: usize,
    /// `numelements="0"></hkparam>` with no line after it yet: the elements of a kept array may
    /// follow, or the next field (an empty array).
    maybe_empty: bool,
}

impl<'a> Open<'a> {
    /// Is the field over before `trimmed`, the next line? Only an empty array is: a field or the
    /// object's end follows.
    fn ends_before(&self, trimmed: &str) -> bool {
        self.maybe_empty && (trimmed.starts_with("<hkparam") || trimmed.starts_with("</hkobject>"))
    }

    /// Takes the pointers of a line inside the field. Anything else makes it a block. Returns
    /// whether the line closes the field.
    fn take_line(&mut self, line: &'a str, start: usize) -> bool {
        let content = line.trim();
        if !content.is_empty() {
            self.maybe_empty = false;
        }
        if content.starts_with("</hkparam>") {
            if self.nested == 0 {
                return true;
            }
            self.nested -= 1;
        } else if content.starts_with("<hkparam") && !content.contains("</hkparam>") && !content.ends_with("/>") {
            self.nested += 1;
        }
        if !self.simple || content.is_empty() {
            return false;
        }
        if content.starts_with('<') {
            self.simple = false;
            return false;
        }
        self.indent = &line[..line.len() - line.trim_start().len()];
        for word in content.split_whitespace() {
            self.elements.push(Token { at: start + offset_in(line, word), text: word });
        }
        false
    }

    fn close(self, close_at: usize) -> ObjectField<'a> {
        let Self { name, schema, line: (line, at), count, elements, indent, simple, .. } = self;
        let value = if simple {
            FieldValue::Pointers(PointerList { elements, close_at, count, indent })
        } else {
            FieldValue::Block(count.map(|count| count.text))
        };
        ObjectField { name, schema, line, at, value }
    }
}

/// Where `part` (a slice of `whole`) starts in it.
fn offset_in(whole: &str, part: &str) -> usize {
    part.as_ptr() as usize - whole.as_ptr() as usize
}

/// `text` with `token` replaced by `with`.
///
/// e.g. `<hkparam name="startStateId">0</hkparam>`, the `0`, `2` -> `...">2</hkparam>`
#[must_use]
pub fn replace(text: &str, token: Token<'_>, with: &str) -> String {
    let mut out = String::with_capacity(text.len() + with.len());
    out.push_str(&text[..token.at]);
    out.push_str(with);
    out.push_str(&text[token.end()..]);
    out
}

/// `text` with `id` added at the end of `list`, and its `numelements` one more.
#[must_use]
pub fn append_pointer(text: &str, list: &PointerList<'_>, id: &str) -> String {
    let indent = if list.elements.is_empty() { "\t\t\t\t" } else { list.indent };
    let mut out = String::with_capacity(text.len() + indent.len() + id.len() + 1);
    out.push_str(&text[..list.close_at]);
    out.push_str(indent);
    out.push_str(id);
    out.push('\n');
    out.push_str(&text[list.close_at..]);
    with_count(out, list.count, list.elements.len() + 1)
}

/// `text` without element `index` of `list` (its line too, if nothing else is on it), and its
/// `numelements` one less.
#[must_use]
pub fn remove_pointer(text: &str, list: &PointerList<'_>, index: usize) -> String {
    let Some(element) = list.elements.get(index) else { return text.to_owned() };
    let line_start = text[..element.at].rfind('\n').map_or(0, |at| at + 1);
    let line_end = text[element.end()..].find('\n').map_or(text.len(), |at| element.end() + at + 1);
    let alone = text[line_start..element.at].trim().is_empty() && text[element.end()..line_end].trim().is_empty();
    let (from, to) = if alone {
        (line_start, line_end)
    } else {
        let end =
            element.end() + text[element.end()..line_end].len() - text[element.end()..line_end].trim_start().len();
        (element.at, end.min(line_end))
    };

    let mut out = String::with_capacity(text.len());
    out.push_str(&text[..from]);
    out.push_str(&text[to..]);
    with_count(out, list.count, list.elements.len().saturating_sub(1))
}

/// `text` (already edited after `count`, which is before the edit) with `numelements` set to `len`.
fn with_count(text: String, count: Option<Token<'_>>, len: usize) -> String {
    match count {
        Some(count) => replace(&text, count, &len.to_string()),
        None => text,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MACHINE: &str = "<hkobject name=\"#0095\" class=\"hkbStateMachine\" signature=\"0x816c1dcb\">
\t\t\t<hkparam name=\"name\">Root</hkparam>
\t\t\t<hkparam name=\"startStateId\">0</hkparam>
\t\t\t<hkparam name=\"states\" numelements=\"2\">
\t\t\t\t#0097
\t\t\t\t#0098
\t\t\t</hkparam>
\t\t\t<hkparam name=\"wildcardTransitions\">null</hkparam>
\t\t\t<hkparam name=\"eventToSendWhenStateOrTransitionChanges\">
\t\t\t\t<hkobject>
\t\t\t\t\t<hkparam name=\"id\">-1</hkparam>
\t\t\t\t</hkobject>
\t\t\t</hkparam>
\t\t</hkobject>
";

    fn field<'f, 'a>(fields: &'f [ObjectField<'a>], name: &str) -> &'f ObjectField<'a> {
        fields.iter().find(|field| field.name == name).unwrap()
    }

    fn states(fields: &[ObjectField<'static>]) -> PointerList<'static> {
        let FieldValue::Pointers(list) = &field(fields, "states").value else { panic!("not pointers") };
        list.clone()
    }

    #[test]
    fn finds_the_object_s_own_fields() {
        let fields = object_fields(MACHINE);
        let names: Vec<&str> = fields.iter().map(|field| field.name).collect();
        assert_eq!(
            names,
            ["name", "startStateId", "states", "wildcardTransitions", "eventToSendWhenStateOrTransitionChanges"]
        );

        assert!(matches!(field(&fields, "name").value, FieldValue::Inline(Token { text: "Root", .. })));
        let wildcard = field(&fields, "wildcardTransitions");
        assert!(wildcard.holds_pointers(), "null is a pointer by the schema");
        assert!(matches!(field(&fields, "eventToSendWhenStateOrTransitionChanges").value, FieldValue::Block(None)));

        let FieldValue::Pointers(list) = &field(&fields, "states").value else { panic!("{fields:?}") };
        let ids: Vec<&str> = list.elements.iter().map(|e| e.text).collect();
        assert_eq!(ids, ["#0097", "#0098"]);
        assert_eq!(list.count.map(|c| c.text), Some("2"));
    }

    /// `hkbBehaviorGraphData.variableInfos` of `0_master`: objects whose fields span lines.
    #[test]
    fn fields_of_nested_objects_are_not_the_object_s() {
        let text = "<hkobject name=\"#0083\" class=\"hkbBehaviorGraphData\">
	<hkparam name=\"variableInfos\" numelements=\"2\">
		<hkobject>
			<hkparam name=\"role\">
				<hkobject>
					<hkparam name=\"role\">ROLE_DEFAULT</hkparam>
					<hkparam name=\"flags\">0</hkparam>
				</hkobject>
			</hkparam>
			<hkparam name=\"type\">VARIABLE_TYPE_BOOL</hkparam>
		</hkobject>
		<hkobject>
			<hkparam name=\"role\">
				<hkobject>
					<hkparam name=\"flags\">0</hkparam>
				</hkobject>
			</hkparam>
		</hkobject>
	</hkparam>
	<hkparam name=\"eventInfos\" numelements=\"0\"></hkparam>
</hkobject>
";
        let names: Vec<&str> = object_fields(text).iter().map(|field| field.name).collect();
        assert_eq!(names, ["variableInfos", "eventInfos"]);
    }

    /// `numelements="0"></hkparam>`, then elements and `</hkparam>`: Nemesis's kept line.
    #[test]
    fn a_kept_array_goes_on_to_its_own_end() {
        let text = "<hkobject name=\"#1\" class=\"hkbStateMachine\">
<hkparam name=\"states\" numelements=\"0\"></hkparam>
	#0097
</hkparam>
<hkparam name=\"startStateId\">1</hkparam>
</hkobject>
";
        let names: Vec<&str> = object_fields(text).iter().map(|field| field.name).collect();
        assert_eq!(names, ["states", "startStateId"]);
    }

    #[test]
    fn a_value_is_replaced_alone() {
        let fields = object_fields(MACHINE);
        let FieldValue::Inline(value) = field(&fields, "startStateId").value else { panic!() };
        let edited = replace(MACHINE, value, "3");
        assert!(edited.contains("<hkparam name=\"startStateId\">3</hkparam>"));
        assert_eq!(edited.len(), MACHINE.len());
    }

    #[test]
    fn pointers_are_added_and_removed_with_their_count() {
        let fields = object_fields(MACHINE);
        let list = states(&fields);

        let added = append_pointer(MACHINE, &list, "#mod$0");
        assert!(
            added.contains("numelements=\"3\">\n\t\t\t\t#0097\n\t\t\t\t#0098\n\t\t\t\t#mod$0\n\t\t\t</hkparam>"),
            "{added}"
        );

        let removed = remove_pointer(MACHINE, &list, 0);
        assert!(removed.contains("numelements=\"1\">\n\t\t\t\t#0098\n\t\t\t</hkparam>"), "{removed}");
    }

    #[test]
    fn of_several_pointers_on_a_line_one_goes() {
        let text = "<hkobject name=\"#1\" class=\"hkbStateMachine\">\n<hkparam name=\"states\" numelements=\"2\">\n\t#0097 #0098\n</hkparam>\n</hkobject>\n";
        let fields = object_fields(text);
        let FieldValue::Pointers(list) = &fields[0].value else { panic!("{fields:?}") };
        assert_eq!(list.elements.len(), 2);
        let removed = remove_pointer(text, list, 0);
        assert!(removed.contains("numelements=\"1\">\n\t#0098\n</hkparam>"), "{removed}");
    }
}
