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
    let string_data =
        push(string_data, "eventNames", &[&format!("<hkcstring>{name}</hkcstring>")])?;
    let graph_data = push(
        graph_data,
        "eventInfos",
        &["<hkobject>", "\t<hkparam name=\"flags\">0</hkparam>", "</hkobject>"],
    )?;
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
    let string_data =
        push(string_data, "variableNames", &[&format!("<hkcstring>{name}</hkcstring>")])?;
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
    let value_set =
        push(value_set, "wordVariableValues", &["<hkobject>", &value_line, "</hkobject>"])?;
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
                line.trim() == "</hkparam>"
                    && line.starts_with(&indent)
                    && line[indent.len()..].starts_with('<')
            })
            .map(|i| at + 1 + i)?;
        let mut elements = Vec::new();
        let mut i = at + 1;
        while i < close {
            let line = lines[i];
            if line.trim() == "<hkobject>" {
                let element_indent: String =
                    line.chars().take_while(|c| c.is_whitespace()).collect();
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

    const STRING_DATA: &str = "\t\t<hkobject name=\"#0092\" class=\"hkbBehaviorGraphStringData\" signature=\"0xc713064e\">
\t\t\t<hkparam name=\"eventNames\" numelements=\"2\">
\t\t\t\t<hkcstring>moveStart</hkcstring>
\t\t\t\t<hkcstring>moveStop</hkcstring>
\t\t\t</hkparam>
\t\t\t<hkparam name=\"variableNames\" numelements=\"0\"></hkparam>
\t\t</hkobject>
";
    const GRAPH_DATA: &str =
        "\t\t<hkobject name=\"#0094\" class=\"hkbBehaviorGraphData\" signature=\"0x95aca5d\">
\t\t\t<hkparam name=\"eventInfos\" numelements=\"1\">
\t\t\t\t<hkobject>
\t\t\t\t\t<hkparam name=\"flags\">0</hkparam>
\t\t\t\t</hkobject>
\t\t\t</hkparam>
\t\t\t<hkparam name=\"variableInfos\" numelements=\"0\"></hkparam>
\t\t</hkobject>
";
    const VALUE_SET: &str =
        "\t\t<hkobject name=\"#0093\" class=\"hkbVariableValueSet\" signature=\"0x27812d8d\">
\t\t\t<hkparam name=\"wordVariableValues\" numelements=\"0\"></hkparam>
\t\t</hkobject>
";

    #[test]
    fn add_and_remove() {
        assert_eq!(names(STRING_DATA, "eventNames"), ["moveStart", "moveStop"]);

        let (string_data, graph_data) = add_event(STRING_DATA, GRAPH_DATA, "MyEvent").unwrap();
        assert_eq!(names(&string_data, "eventNames"), ["moveStart", "moveStop", "MyEvent"]);
        assert_eq!(Array::find(&graph_data, "eventInfos").unwrap().elements.len(), 2);

        let (string_data, graph_data, value_set) = add_variable(
            &string_data,
            &graph_data,
            VALUE_SET,
            "myVar",
            VariableType::Real,
            1_045_220_557,
        )
        .unwrap();
        assert_eq!(names(&string_data, "variableNames"), ["myVar"]);
        assert!(graph_data.contains("VARIABLE_TYPE_REAL"));
        assert!(value_set.contains("\t\t\t\t\t<hkparam name=\"value\">1045220557</hkparam>\n\t\t\t\t</hkobject>\n\t\t\t</hkparam>"));

        let removed = remove(&string_data, "eventNames", 2).unwrap();
        assert_eq!(names(&removed, "eventNames"), ["moveStart", "moveStop"]);
        assert_eq!(VariableType::Real.word_value("0.2"), Some(1_045_220_557));
    }
}
