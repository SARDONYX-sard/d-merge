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
pub fn names(object: &str, param: &str) -> Vec<String> {
    let Some(array) = Array::find(object, param) else { return Vec::new() };
    array
        .elements
        .iter()
        .filter_map(|lines| {
            let line = lines.first()?.trim();
            Some(line.strip_prefix("<hkcstring>")?.strip_suffix("</hkcstring>")?.to_string())
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
    let indent: String = line.chars().take_while(|c| c.is_whitespace()).collect();
    *line = format!("{indent}<hkcstring>{new_name}</hkcstring>");
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
    array.elements.push(element.iter().map(|line| format!("{indent}{line}")).collect());
    Some(array.join())
}

/// Removes element `index` of array `param` of `object`.
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

/// An array field split into its elements.
struct Array {
    /// Lines before the field's opening line, and the opening line (`<hkparam name=".." ...>`).
    head: Vec<String>,
    /// The field's indent.
    indent: String,
    /// Each element's lines: one line (`<hkcstring>`, a pointer) or an `<hkobject>` block.
    elements: Vec<Vec<String>>,
    /// The closing `</hkparam>` line and the rest.
    tail: Vec<String>,
}

impl Array {
    fn find(object: &str, param: &str) -> Option<Self> {
        // A push that kept the line of the empty array has its elements after it.
        let object = crate::open_kept_arrays(object);
        let lines: Vec<&str> = object.lines().collect();
        let open = format!("<hkparam name=\"{param}\"");
        let at = lines.iter().position(|line| line.trim_start().starts_with(&open))?;
        let indent: String = lines[at].chars().take_while(|c| c.is_whitespace()).collect();
        let mut head: Vec<String> = lines[..at].iter().map(|l| (*l).to_string()).collect();

        // `<hkparam name="x" numelements="0"></hkparam>`: no elements yet, split the line.
        if let Some(opening) = lines[at].strip_suffix("</hkparam>") {
            head.push(opening.to_string());
            let mut tail = vec![format!("{indent}</hkparam>")];
            tail.extend(lines[at + 1..].iter().map(|l| (*l).to_string()));
            return Some(Self { head, indent, elements: Vec::new(), tail });
        }
        head.push(lines[at].to_string());

        let close = lines[at + 1..]
            .iter()
            .position(|line| {
                line.trim() == "</hkparam>" && line.starts_with(&indent) && line[indent.len()..].starts_with('<')
            })
            .map(|i| at + 1 + i)?;
        let mut elements = Vec::new();
        let mut i = at + 1;
        while i < close {
            let line = lines[i];
            if line.trim() == "<hkobject>" {
                let element_indent: String = line.chars().take_while(|c| c.is_whitespace()).collect();
                let end = (i + 1..close)
                    .find(|&j| {
                        lines[j].trim() == "</hkobject>"
                            && lines[j].starts_with(&element_indent)
                            && lines[j][element_indent.len()..].starts_with('<')
                    })
                    .unwrap_or(close - 1);
                elements.push(lines[i..=end].iter().map(|l| (*l).to_string()).collect());
                i = end + 1;
            } else {
                elements.push(vec![line.to_string()]);
                i += 1;
            }
        }
        let tail = lines[close..].iter().map(|l| (*l).to_string()).collect();
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
}
