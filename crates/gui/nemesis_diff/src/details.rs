//! Every field of an object, those of the objects nested in it too, as the rows of a details panel:
//! each value with where it is in the text and its schema, for an input of its type.
//!
//! ```text
//! <hkobject name="#0083" class="hkbBehaviorGraphData">        depth  row
//!   <hkparam name="variableInfos" numelements="1">            0      Group  variableInfos [1]
//!     <hkobject>                                              1      Group  [0]
//!       <hkparam name="role">                                 2      Group  role
//!         <hkobject>
//!           <hkparam name="role">ROLE_DEFAULT</hkparam>       3      Value  role (Enum)
//!           <hkparam name="flags">0</hkparam>                 3      Value  flags (Flags)
//!         </hkobject>
//!       </hkparam>
//!       <hkparam name="type">VARIABLE_TYPE_BOOL</hkparam>     2      Value  type (Enum)
//!     </hkobject>
//!   </hkparam>
//!   <hkparam name="eventNames" numelements="1">               0      Group  eventNames [1]
//!     <hkcstring>Jump</hkcstring>                             1      Value  [0] (an element: StringPtr)
//!   </hkparam>
//! </hkobject>
//! ```
//!
//! A kept line (`numelements="0"></hkparam>` with elements after it) is left to the text.

use crate::{
    attribute,
    fields::Token,
    schema::{FieldSchema, TypeKind, xml_fields},
};

/// A row of the details of an object.
#[derive(Debug, Clone, Copy)]
pub struct DetailRow<'a> {
    /// `0`: a field of the object itself; deeper: of what is above it.
    pub depth: usize,
    pub label: Label<'a>,
    pub kind: DetailKind<'a>,
    /// The byte offset of its line: an id for it, and where it is in the text.
    pub at: usize,
}

/// What a row is called.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Label<'a> {
    Field(&'a str),
    /// An element of a list.
    Element(usize),
}

/// What a row holds.
#[derive(Debug, Clone, Copy)]
pub enum DetailKind<'a> {
    /// A field over lines, or an object of a list: the deeper rows after it are its.
    Group {
        /// Its `numelements`, if a list.
        count: Option<&'a str>,
    },
    /// A value on one line, or an element of a list of values.
    Value {
        token: Token<'a>,
        schema: Option<&'static FieldSchema>,
        /// An element: of the type of the list's elements (`sub_kind`), not of the list.
        element: bool,
    },
}

impl DetailKind<'_> {
    /// The type of a value, by its schema. `None`: a group, or a field the schema does not know.
    #[must_use]
    pub const fn value_type(&self) -> Option<TypeKind> {
        match *self {
            Self::Value { schema: Some(schema), element, .. } => {
                Some(if element { schema.sub_kind } else { schema.kind })
            }
            _ => None,
        }
    }
}

/// What a line is inside of.
#[derive(Debug, Clone, Copy)]
enum Open {
    /// An object, of this class if known; `true`: an element of a list, with a row of its own.
    Object(Option<&'static str>, bool),
    /// A field over lines, how many elements it had so far, and whether it is a list (else a
    /// struct: its one object has no row of its own).
    Field(Option<&'static FieldSchema>, usize, bool),
}

/// How deep the rows inside `open` are: a field and an object of a list each go one deeper.
fn depth_in(open: &[Open]) -> usize {
    open.iter().skip(1).filter(|open| matches!(open, Open::Field(..) | Open::Object(_, true))).count()
}

/// The rows of `object`: see the module docs.
#[must_use]
pub fn object_details(object: &str) -> Vec<DetailRow<'_>> {
    let mut open: Vec<Open> = Vec::new();
    let mut rows = Vec::new();
    let mut at = 0;
    for line in object.split_inclusive('\n') {
        let start = at;
        at += line.len();
        let trimmed = line.trim();
        let depth = depth_in(&open);

        if let Some(rest) = trimmed.strip_prefix("<hkobject") {
            object_line(&mut open, &mut rows, (rest, start, depth));
        } else if trimmed.starts_with("</hkobject>") {
            if matches!(open.last(), Some(Open::Object(..))) {
                open.pop();
            }
        } else if trimmed.starts_with("<hkparam") {
            param_line(&mut open, &mut rows, (line, start, depth));
        } else if trimmed.starts_with("</hkparam>") {
            if matches!(open.last(), Some(Open::Field(..))) {
                open.pop();
            }
        } else if let Some(Open::Field(schema, count, _)) = open.last_mut() {
            element_line(&mut rows, (*schema, count), (line, start, depth));
        }
    }
    rows
}

/// `<hkobject`: the object itself (no row), or an object of a list.
fn object_line(open: &mut Vec<Open>, rows: &mut Vec<DetailRow<'_>>, (rest, start, depth): (&str, usize, usize)) {
    let (class, is_element) = match open.last_mut() {
        None => (attribute(rest, "class").and_then(static_class), false),
        Some(Open::Field(schema, count, is_list)) => {
            if *is_list {
                let kind = DetailKind::Group { count: None };
                rows.push(DetailRow { depth, label: Label::Element(*count), kind, at: start });
                *count += 1;
            }
            (schema.and_then(|schema| schema.class), *is_list)
        }
        Some(Open::Object(..)) => (None, false),
    };
    if !rest.trim_end().ends_with("/>") {
        open.push(Open::Object(class, is_element));
    }
}

/// `<hkparam`: a value on its line, or a field over the next lines.
fn param_line<'a>(open: &mut Vec<Open>, rows: &mut Vec<DetailRow<'a>>, (line, start, depth): (&'a str, usize, usize)) {
    let class = match open.last() {
        Some(Open::Object(class, _)) => *class,
        _ => None,
    };
    let name = attribute(line, "name").unwrap_or_default();
    let schema = class.and_then(|class| xml_fields(class).find(|field| field.name == name));
    let label = Label::Field(name);
    let count = attribute(line, "numelements");

    let tag_end = line.find('>').map_or(line.len(), |end| end + 1);
    if line[..tag_end].ends_with("/>") {
        return;
    }
    if let Some(close) = line[tag_end..].find("</hkparam>") {
        let text = &line[tag_end..tag_end + close];
        let kind = if count.is_some() && text.is_empty() {
            DetailKind::Group { count } // an empty list, or a kept line: the text's
        } else {
            DetailKind::Value { token: Token { at: start + tag_end, text }, schema, element: false }
        };
        rows.push(DetailRow { depth, label, kind, at: start });
        return;
    }
    rows.push(DetailRow { depth, label, kind: DetailKind::Group { count }, at: start });
    open.push(Open::Field(schema, 0, count.is_some()));
}

/// A line of a list's elements: `<hkcstring>a</hkcstring>`, or values apart by spaces.
fn element_line<'a>(
    rows: &mut Vec<DetailRow<'a>>,
    (schema, count): (Option<&'static FieldSchema>, &mut usize),
    (line, start, depth): (&'a str, usize, usize),
) {
    let mut push = |text: &'a str| {
        let token = Token { at: start + offset_in(line, text), text };
        let kind = DetailKind::Value { token, schema, element: true };
        rows.push(DetailRow { depth, label: Label::Element(*count), kind, at: start });
        *count += 1;
    };
    let content = line.trim();
    if let Some(text) = content.strip_prefix("<hkcstring>").and_then(|rest| rest.strip_suffix("</hkcstring>")) {
        push(text);
    } else if !content.starts_with('<') {
        content.split_whitespace().for_each(push);
    }
}

/// `class` as the schema names it, for as long as the schema lives.
fn static_class(class: &str) -> Option<&'static str> {
    nemesis_xml::patch::class_schema::find_class_schema(class).map(|schema| schema.name)
}

/// Where `part` (a slice of `whole`) starts in it.
fn offset_in(whole: &str, part: &str) -> usize {
    part.as_ptr() as usize - whole.as_ptr() as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    const DATA: &str = "<hkobject name=\"#0083\" class=\"hkbBehaviorGraphData\">
\t<hkparam name=\"variableInfos\" numelements=\"1\">
\t\t<hkobject>
\t\t\t<hkparam name=\"role\">
\t\t\t\t<hkobject>
\t\t\t\t\t<hkparam name=\"role\">ROLE_DEFAULT</hkparam>
\t\t\t\t\t<hkparam name=\"flags\">0</hkparam>
\t\t\t\t</hkobject>
\t\t\t</hkparam>
\t\t\t<hkparam name=\"type\">VARIABLE_TYPE_BOOL</hkparam>
\t\t</hkobject>
\t</hkparam>
\t<hkparam name=\"eventInfos\" numelements=\"0\"></hkparam>
</hkobject>
";

    /// `(depth, label, value or "", type)` of each row.
    fn summary(rows: &[DetailRow<'_>]) -> Vec<(usize, String, &'static str, Option<TypeKind>)> {
        rows.iter()
            .map(|row| {
                let label = match row.label {
                    Label::Field(name) => name.to_owned(),
                    Label::Element(index) => format!("[{index}]"),
                };
                let value = match row.kind {
                    DetailKind::Value { token, .. } => DATA.get(token.at..token.end()).map_or("?", |_| "v"),
                    DetailKind::Group { .. } => "",
                };
                (row.depth, label, value, row.kind.value_type())
            })
            .collect()
    }

    #[test]
    fn nested_objects_are_rows_with_their_types() {
        let rows = object_details(DATA);
        assert_eq!(
            summary(&rows),
            [
                (0, "variableInfos".to_owned(), "", None),
                (1, "[0]".to_owned(), "", None),
                (2, "role".to_owned(), "", None),
                (3, "role".to_owned(), "v", Some(TypeKind::Enum)),
                (3, "flags".to_owned(), "v", Some(TypeKind::Flags)),
                (2, "type".to_owned(), "v", Some(TypeKind::Enum)),
                (0, "eventInfos".to_owned(), "", None),
            ]
        );
        let DetailKind::Value { token, .. } = rows[3].kind else { panic!() };
        assert_eq!(&DATA[token.at..token.end()], "ROLE_DEFAULT");
    }

    #[test]
    fn elements_of_lists_are_values_of_their_type() {
        let text = "<hkobject name=\"#1\" class=\"hkbBehaviorGraphStringData\">\n\t<hkparam name=\"eventNames\" numelements=\"2\">\n\t\t<hkcstring>Jump</hkcstring>\n\t\t<hkcstring>Land</hkcstring>\n\t</hkparam>\n</hkobject>\n";
        let rows = object_details(text);
        let values: Vec<(&str, Option<TypeKind>)> = rows
            .iter()
            .filter_map(|row| match row.kind {
                DetailKind::Value { token, .. } => Some((token.text, row.kind.value_type())),
                DetailKind::Group { .. } => None,
            })
            .collect();
        assert_eq!(values, [("Jump", Some(TypeKind::StringPtr)), ("Land", Some(TypeKind::StringPtr))]);
    }
}
