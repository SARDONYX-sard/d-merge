//! Behavior events and variables (what Nemesis' `$eventID[name]$` / `$variableID[name]$` refer to).
//!
//! An event or variable lives in several parallel arrays, one entry each at the same index:
//!
//! | object class                 | event          | variable                                     |
//! | ---------------------------- | -------------- | -------------------------------------------- |
//! | `hkbBehaviorGraphStringData` | `eventNames`   | `variableNames`                              |
//! | `hkbBehaviorGraphData`       | `eventInfos`   | `variableInfos`                              |
//! | `hkbVariableValueSet`        |                | `wordVariableValues` (the initial value)     |
//!
//! These functions edit one array of one object text; [`add_event`] / [`add_variable`] edit all
//! of them. A Nemesis mod appends (never inserts), so entries are always added at the end.
//! `numelements` is left as is: Nemesis / d-merge count the elements themselves, and leaving it
//! keeps patches of different mods from conflicting on that line.

use std::borrow::Cow;

use crate::kept_array::is_kept_line;

/// How patches refer to an event: `$eventID[name]$`.
pub const EVENT_ID_PREFIX: &str = "$eventID[";
/// How patches refer to a variable: `$variableID[name]$`.
pub const VARIABLE_ID_PREFIX: &str = "$variableID[";

/// Variable types that keep their initial value in `wordVariableValues`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariableType {
    Bool,
    Int8,
    Int16,
    Int32,
    Real,
}

impl VariableType {
    pub const ALL: [Self; 5] = [Self::Bool, Self::Int8, Self::Int16, Self::Int32, Self::Real];

    /// e.g. `VARIABLE_TYPE_REAL`
    pub const fn havok_name(self) -> &'static str {
        match self {
            Self::Bool => "VARIABLE_TYPE_BOOL",
            Self::Int8 => "VARIABLE_TYPE_INT8",
            Self::Int16 => "VARIABLE_TYPE_INT16",
            Self::Int32 => "VARIABLE_TYPE_INT32",
            Self::Real => "VARIABLE_TYPE_REAL",
        }
    }

    /// The type as [`variable_types`] gives it. e.g. `REAL`
    pub const fn name(self) -> &'static str {
        match self {
            Self::Bool => "BOOL",
            Self::Int8 => "INT8",
            Self::Int16 => "INT16",
            Self::Int32 => "INT32",
            Self::Real => "REAL",
        }
    }

    /// The type named so by [`variable_types`]. `None`: one without a word value (`POINTER`,
    /// `VECTOR4`, `QUATERNION`). e.g. `REAL` -> `Real`
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.name() == name)
    }

    /// The `wordVariableValues` value `word` as it is typed: the other way of [`Self::word_value`].
    ///
    /// e.g. `Real` + `1045220557` -> `0.2`, `Bool` + `1` -> `true`, `Int32` + `-3` -> `-3`
    pub fn show(self, word: i64) -> String {
        match self {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // the f32 bits, as written
            Self::Real => format!("{:?}", f32::from_bits(word as u32)),
            Self::Bool => (word != 0).to_string(),
            Self::Int8 | Self::Int16 | Self::Int32 => word.to_string(),
        }
    }

    /// The `wordVariableValues` value of `input` (a REAL is stored as its f32 bits).
    ///
    /// e.g. `Real` + `0.2` -> `1045220557`, `Bool` + `true` -> `1`
    pub fn word_value(self, input: &str) -> Option<i64> {
        let input = input.trim();
        match self {
            Self::Real if input.is_empty() => Some(0), // the bits of `0.0`
            Self::Real => input.parse::<f32>().ok().map(|f| i64::from(f.to_bits())),
            Self::Bool => match input.to_ascii_lowercase().as_str() {
                "" | "0" | "false" => Some(0),
                "1" | "true" => Some(1),
                _ => None,
            },
            Self::Int8 | Self::Int16 | Self::Int32 => {
                if input.is_empty() {
                    Some(0)
                } else {
                    input.parse::<i64>().ok()
                }
            }
        }
    }
}

/// The `<hkcstring>` names of an array, e.g. `eventNames` of a `hkbBehaviorGraphStringData`.
pub fn names<'a>(object: &'a str, param: &str) -> Vec<&'a str> {
    let Some(array) = Array::find(object, param) else { return Vec::new() };
    array
        .elements
        .iter()
        .filter_map(|lines| {
            // (Only an element just written is not in `object`, and that is not here.)
            let Cow::Borrowed(line) = lines.first()? else { return None };
            line.trim().strip_prefix("<hkcstring>")?.strip_suffix("</hkcstring>")
        })
        .collect()
}

/// The 0-based line of `name` in array `param` of `object` (ignoring ASCII case, as Nemesis does),
/// to open the place an event or variable is defined at.
///
/// e.g., `eventNames` + `moveStop` -> the line of `<hkcstring>moveStop</hkcstring>`
pub fn name_line(object: &str, param: &str, name: &str) -> Option<usize> {
    let open = format!("<hkparam name=\"{param}\"");
    let mut lines = object.lines().enumerate();
    lines.find(|(_, line)| line.trim_start().starts_with(&open))?;
    let (line, _) = lines.take_while(|(_, line)| !line.trim_start().starts_with("</hkparam>")).find(|(_, line)| {
        let value = line.trim().strip_prefix("<hkcstring>").and_then(|v| v.strip_suffix("</hkcstring>"));
        value.is_some_and(|value| value.eq_ignore_ascii_case(name))
    })?;
    Some(line)
}

/// The type of each entry of `variableInfos` of a `hkbBehaviorGraphData`, by index, without the
/// `VARIABLE_TYPE_` prefix.
///
/// e.g., `<hkparam name="type">VARIABLE_TYPE_REAL</hkparam>` -> `REAL`
pub fn variable_types(graph_data: &str) -> Vec<&str> {
    const OPEN: &str = "<hkparam name=\"type\">";
    let mut lines = graph_data.lines();
    if !lines.any(|line| line.trim_start().starts_with("<hkparam name=\"variableInfos\"")) {
        return Vec::new();
    }
    // The array ends at a `</hkparam>` with its own indent, or a field on one line when it is empty.
    lines
        .take_while(|line| !line.trim_start().starts_with("<hkparam name=\"") || line.contains(OPEN) || is_nested(line))
        .filter_map(|line| line.trim().strip_prefix(OPEN)?.strip_suffix("</hkparam>"))
        .map(|kind| kind.strip_prefix("VARIABLE_TYPE_").unwrap_or(kind))
        .collect()
}

/// The `value` of each element of `wordVariableValues` of a `hkbVariableValueSet`, by index: the
/// initial value of each variable. See [`VariableType::show`].
///
/// e.g. `<hkobject><hkparam name="value">1045220557</hkparam></hkobject>` -> `1045220557`
pub fn word_values(value_set: &str) -> Vec<i64> {
    let Some(array) = Array::find(value_set, "wordVariableValues") else { return Vec::new() };
    let value_of = |lines: &Vec<Cow<'_, str>>| {
        lines.iter().find_map(|line| {
            let value = line.trim().strip_prefix("<hkparam name=\"value\">")?.strip_suffix("</hkparam>")?;
            value.trim().parse::<i64>().ok()
        })
    };
    array.elements.iter().map(|lines| value_of(lines).unwrap_or_default()).collect()
}

/// The 0-based line element `index` of the array `param` of `object` starts at.
///
/// e.g. `wordVariableValues`, 1 -> the line of the second `<hkobject>` of it
pub fn element_line(object: &str, param: &str, index: usize) -> Option<usize> {
    let array = Array::find(object, param)?;
    let before: usize = array.elements.get(..index)?.iter().map(Vec::len).sum();
    array.elements.get(index)?;
    Some(array.head.len() + before)
}

/// `object` with the field `field` of element `index` of the array `param` set to `value`. `None`:
/// there is no such element or field.
///
/// e.g. `variableInfos`, 3, `type`, `VARIABLE_TYPE_REAL`;
/// `wordVariableValues`, 3, `value`, `1045220557`
pub fn set_element_field(object: &str, param: &str, index: usize, field: &str, value: &str) -> Option<String> {
    let mut array = Array::find(object, param)?;
    let open = format!("<hkparam name=\"{field}\">");
    let line = array.elements.get_mut(index)?.iter_mut().find(|line| line.trim_start().starts_with(&open))?;
    *line = Cow::Owned(format!("{}{open}{value}</hkparam>", indent_of(line)));
    Some(array.join())
}

/// A field inside a `variableInfos` element, not the next field of the object.
fn is_nested(line: &str) -> bool {
    ["\"role\"", "\"flags\""].iter().any(|name| line.contains(name))
}

/// Element `index` of the name array `param` with a new name.
///
/// e.g., `eventNames`, 1, `run` turns `<hkcstring>moveStop</hkcstring>` into `<hkcstring>run</hkcstring>`
pub fn rename(object: &str, param: &str, index: usize, new_name: &str) -> Option<String> {
    let mut array = Array::find(object, param)?;
    let line = array.elements.get_mut(index)?.first_mut()?;
    let renamed = format!("{}<hkcstring>{new_name}</hkcstring>", indent_of(line));
    *line = Cow::Owned(renamed);
    Some(array.join())
}

/// `text` with every use of a name changed, or `None` if it has none. Case is ignored, as Nemesis
/// and d-merge do.
///
/// e.g., `$eventID[`, `old`, `new`: `<hkparam name="id">$EVENTID[OLD]$</hkparam>` ->
/// `<hkparam name="id">$EVENTID[new]$</hkparam>`
pub fn rename_uses(text: &str, prefix: &str, old: &str, new: &str) -> Option<String> {
    let mut out = String::new();
    let mut copied = 0;
    for (at, _) in text.match_indices('$') {
        let rest = &text[at..];
        let after_prefix = rest
            .get(prefix.len()..)
            .filter(|_| rest.get(..prefix.len()).is_some_and(|p| p.eq_ignore_ascii_case(prefix)));
        let name = after_prefix.and_then(|after| after.get(..old.len())).filter(|name| name.eq_ignore_ascii_case(old));
        let closed = after_prefix.and_then(|after| after.get(old.len()..)).is_some_and(|tail| tail.starts_with("]$"));
        if name.is_some() && closed {
            let name_start = at + prefix.len();
            out.push_str(&text[copied..name_start]);
            out.push_str(new);
            copied = name_start + old.len();
        }
    }
    if copied == 0 {
        return None;
    }
    out.push_str(&text[copied..]);
    Some(out)
}

/// Appends an element (lines relative to the element indent) to array `param` of `object`.
pub fn push(object: &str, param: &str, element: &[&str]) -> Option<String> {
    let mut array = Array::find(object, param)?;
    let indent = format!("{}\t", array.indent);
    array.elements.push(element.iter().map(|line| Cow::Owned(format!("{indent}{line}"))).collect());
    Some(array.join())
}

/// Removes element `index` of array `param` of `object`.
#[inline]
pub fn remove(object: &str, param: &str, index: usize) -> Option<String> {
    let mut array = Array::find(object, param)?;
    (index < array.elements.len()).then(|| array.elements.remove(index))?;
    Some(array.join())
}

/// `(string data, graph data)` with event `name` appended.
pub fn add_event(string_data: &str, graph_data: &str, name: &str) -> Option<(String, String)> {
    let string_data = push(string_data, "eventNames", &[&format!("<hkcstring>{name}</hkcstring>")])?;
    let graph_data =
        push(graph_data, "eventInfos", &["<hkobject>", "\t<hkparam name=\"flags\">0</hkparam>", "</hkobject>"])?;
    Some((string_data, graph_data))
}

/// `(string data, graph data, value set)` with variable `name` appended.
pub fn add_variable(
    string_data: &str,
    graph_data: &str,
    value_set: &str,
    name: &str,
    kind: VariableType,
    value: i64,
) -> Option<(String, String, String)> {
    let string_data = push(string_data, "variableNames", &[&format!("<hkcstring>{name}</hkcstring>")])?;
    let type_line = format!("\t<hkparam name=\"type\">{}</hkparam>", kind.havok_name());
    let graph_data = push(
        graph_data,
        "variableInfos",
        &[
            "<hkobject>",
            "\t<hkparam name=\"role\">",
            "\t\t<hkobject>",
            "\t\t\t<hkparam name=\"role\">ROLE_DEFAULT</hkparam>",
            "\t\t\t<hkparam name=\"flags\">0</hkparam>",
            "\t\t</hkobject>",
            "\t</hkparam>",
            &type_line,
            "</hkobject>",
        ],
    )?;
    let value_line = format!("\t<hkparam name=\"value\">{value}</hkparam>");
    let value_set = push(value_set, "wordVariableValues", &["<hkobject>", &value_line, "</hkobject>"])?;
    Some((string_data, graph_data, value_set))
}

/// The leading whitespace of `line`.
fn indent_of(line: &str) -> &str {
    &line[..line.len() - line.trim_start().len()]
}

/// An array field split into its elements. Its lines are the object's, but for what is changed.
struct Array<'a> {
    /// Lines before the field's opening line, and the opening line (`<hkparam name=".." ...>`).
    head: Vec<Cow<'a, str>>,
    /// The field's indent.
    indent: &'a str,
    /// Each element's lines: one line (`<hkcstring>`, a pointer) or an `<hkobject>` block.
    elements: Vec<Vec<Cow<'a, str>>>,
    /// The closing `</hkparam>` line and the rest.
    tail: Vec<Cow<'a, str>>,
}

/// `lines` as they are.
fn borrowed<'a>(lines: &[&'a str]) -> impl Iterator<Item = Cow<'a, str>> {
    lines.iter().map(|line| Cow::Borrowed(*line))
}

impl<'a> Array<'a> {
    fn find(object: &'a str, param: &str) -> Option<Self> {
        let lines: Vec<&str> = object.lines().collect();
        let open = format!("<hkparam name=\"{param}\"");
        let at = lines.iter().position(|line| line.trim_start().starts_with(&open))?;
        let indent = indent_of(lines[at]);
        let mut head: Vec<Cow<'a, str>> = borrowed(&lines[..at]).collect();

        match lines[at].strip_suffix("</hkparam>") {
            // `<hkparam name="x" numelements="0"></hkparam>`: no elements yet, split the line.
            Some(opening) if !is_kept_line(&lines, at) => {
                head.push(Cow::Borrowed(opening));
                let mut tail = vec![Cow::Owned(format!("{indent}</hkparam>"))];
                tail.extend(borrowed(&lines[at + 1..]));
                return Some(Self { head, indent, elements: Vec::new(), tail });
            }
            // A push that kept the line of the empty array has its elements after it.
            Some(opening) => head.push(Cow::Borrowed(opening)),
            None => head.push(Cow::Borrowed(lines[at])),
        }

        let close = lines[at + 1..]
            .iter()
            .position(|line| {
                line.trim() == "</hkparam>" && line.starts_with(indent) && line[indent.len()..].starts_with('<')
            })
            .map(|i| at + 1 + i)?;
        let mut elements = Vec::new();
        let mut i = at + 1;
        while i < close {
            let line = lines[i];
            if line.trim() == "<hkobject>" {
                let element_indent = indent_of(line);
                let end = (i + 1..close)
                    .find(|&j| {
                        lines[j].trim() == "</hkobject>"
                            && lines[j].starts_with(element_indent)
                            && lines[j][element_indent.len()..].starts_with('<')
                    })
                    .unwrap_or(close - 1);
                elements.push(borrowed(&lines[i..=end]).collect());
                i = end + 1;
            } else {
                elements.push(vec![Cow::Borrowed(line)]);
                i += 1;
            }
        }
        let tail = borrowed(&lines[close..]).collect();
        Some(Self { head, indent, elements, tail })
    }

    fn join(&self) -> String {
        let mut out = String::new();
        for line in self.head.iter().chain(self.elements.iter().flatten()).chain(&self.tail) {
            out.push_str(line);
            out.push('\n');
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_initial_value_is_zero_of_any_type() {
        for kind in VariableType::ALL {
            assert_eq!(kind.word_value(""), Some(0), "{kind:?}");
            assert_eq!(kind.word_value("  "), Some(0), "{kind:?}");
        }
        assert_eq!(VariableType::Real.word_value("0.2"), Some(1_045_220_557));
        assert_eq!(VariableType::Real.word_value("x"), None);
    }

    const STRING_DATA: &str =
        "\t\t<hkobject name=\"#0092\" class=\"hkbBehaviorGraphStringData\" signature=\"0xc713064e\">
\t\t\t<hkparam name=\"eventNames\" numelements=\"2\">
\t\t\t\t<hkcstring>moveStart</hkcstring>
\t\t\t\t<hkcstring>moveStop</hkcstring>
\t\t\t</hkparam>
\t\t\t<hkparam name=\"variableNames\" numelements=\"0\"></hkparam>
\t\t</hkobject>
";
    const GRAPH_DATA: &str = "\t\t<hkobject name=\"#0094\" class=\"hkbBehaviorGraphData\" signature=\"0x95aca5d\">
\t\t\t<hkparam name=\"eventInfos\" numelements=\"1\">
\t\t\t\t<hkobject>
\t\t\t\t\t<hkparam name=\"flags\">0</hkparam>
\t\t\t\t</hkobject>
\t\t\t</hkparam>
\t\t\t<hkparam name=\"variableInfos\" numelements=\"0\"></hkparam>
\t\t</hkobject>
";
    const VALUE_SET: &str = "\t\t<hkobject name=\"#0093\" class=\"hkbVariableValueSet\" signature=\"0x27812d8d\">
\t\t\t<hkparam name=\"wordVariableValues\" numelements=\"0\"></hkparam>
\t\t</hkobject>
";

    #[test]
    fn reads_variable_types() {
        let (_, graph_data, _) = add_variable(STRING_DATA, GRAPH_DATA, VALUE_SET, "a", VariableType::Real, 0).unwrap();
        let (string_data, graph_data, value_set) = (STRING_DATA, graph_data, VALUE_SET);
        let (_, graph_data, _) = add_variable(string_data, &graph_data, value_set, "b", VariableType::Bool, 0).unwrap();
        assert_eq!(variable_types(&graph_data), ["REAL", "BOOL"]);
        assert!(variable_types(GRAPH_DATA).is_empty());
    }

    #[test]
    fn renames_names_and_their_uses() {
        let renamed = rename(STRING_DATA, "eventNames", 1, "run").unwrap();
        assert_eq!(names(&renamed, "eventNames"), ["moveStart", "run"]);
        assert!(rename(STRING_DATA, "eventNames", 9, "run").is_none());

        let text = "a $eventID[moveStop]$ b $EVENTID[MOVESTOP]$ $eventID[moveStopped]$ $variableID[moveStop]$";
        let expected = "a $eventID[run]$ b $EVENTID[run]$ $eventID[moveStopped]$ $variableID[moveStop]$";
        assert_eq!(rename_uses(text, "$eventID[", "moveStop", "run").as_deref(), Some(expected));
        assert_eq!(rename_uses("nothing", "$eventID[", "moveStop", "run"), None);
    }

    #[test]
    fn add_and_remove() {
        assert_eq!(names(STRING_DATA, "eventNames"), ["moveStart", "moveStop"]);
        assert_eq!(name_line(STRING_DATA, "eventNames", "movestop"), Some(3));
        assert_eq!(name_line(STRING_DATA, "variableNames", "moveStop"), None);

        let (string_data, graph_data) = add_event(STRING_DATA, GRAPH_DATA, "MyEvent").unwrap();
        assert_eq!(names(&string_data, "eventNames"), ["moveStart", "moveStop", "MyEvent"]);
        assert_eq!(Array::find(&graph_data, "eventInfos").unwrap().elements.len(), 2);

        // The array is empty in the template, and the element follows its line.
        let kept = "\t\t\t<hkparam name=\"eventNames\" numelements=\"0\"></hkparam>\n\t\t\t\t<hkcstring>Hi</hkcstring>\n\t\t\t</hkparam>\n";
        let array = Array::find(kept, "eventNames").unwrap();
        assert_eq!(array.elements, [["\t\t\t\t<hkcstring>Hi</hkcstring>"]]);

        let (string_data, graph_data, value_set) =
            add_variable(&string_data, &graph_data, VALUE_SET, "myVar", VariableType::Real, 1_045_220_557).unwrap();
        assert_eq!(names(&string_data, "variableNames"), ["myVar"]);
        assert!(graph_data.contains("VARIABLE_TYPE_REAL"));
        assert!(
            value_set.contains(
                "\t\t\t\t\t<hkparam name=\"value\">1045220557</hkparam>\n\t\t\t\t</hkobject>\n\t\t\t</hkparam>"
            )
        );

        let removed = remove(&string_data, "eventNames", 2).unwrap();
        assert_eq!(names(&removed, "eventNames"), ["moveStart", "moveStop"]);
        assert_eq!(VariableType::Real.word_value("0.2"), Some(1_045_220_557));
    }

    #[test]
    fn a_variable_s_type_and_initial_value_are_read_and_set() {
        let (_, graph_data, value_set) =
            add_variable(STRING_DATA, GRAPH_DATA, VALUE_SET, "a", VariableType::Real, 1_045_220_557).unwrap();
        assert_eq!(word_values(&value_set), [1_045_220_557]);
        assert_eq!(VariableType::from_name(variable_types(&graph_data)[0]), Some(VariableType::Real));
        assert_eq!(VariableType::Real.show(1_045_220_557), "0.2");
        assert_eq!((VariableType::Bool.show(1), VariableType::Int8.show(-3)), ("true".to_owned(), "-3".to_owned()));
        assert_eq!(VariableType::from_name("POINTER"), None, "no word value");

        let graph_data = set_element_field(&graph_data, "variableInfos", 0, "type", "VARIABLE_TYPE_INT8").unwrap();
        assert_eq!(variable_types(&graph_data), ["INT8"]);
        assert!(graph_data.contains("ROLE_DEFAULT"), "the rest kept");
        assert_eq!(element_line(&value_set, "wordVariableValues", 0), Some(2), "after the object's and the field's");
        assert_eq!(element_line(STRING_DATA, "eventNames", 1), Some(3));
        assert_eq!(element_line(STRING_DATA, "eventNames", 2), None);
        let value_set = set_element_field(&value_set, "wordVariableValues", 0, "value", "7").unwrap();
        assert_eq!(word_values(&value_set), [7]);
        assert!(set_element_field(&value_set, "wordVariableValues", 1, "value", "7").is_none(), "no element 1");
    }
}
