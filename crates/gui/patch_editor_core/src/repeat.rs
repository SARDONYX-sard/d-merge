//! Making many objects from one, as a loop would: a Nemesis patch often has many classes alike, each
//! with a few values of its own (e.g. a `hkbClipGenerator` per attack).
//!
//! ```text
//! template:  <hkparam name="name">Attack_{{name}}</hkparam>
//!            <hkparam name="animationName">Animations\{{file}}</hkparam>
//! values:    name, file          <- the first line names the columns
//!            A, a.hkx            <- then one object per line
//!            B, b.hkx
//! -> 2 new objects: Attack_A with a.hkx, Attack_B with b.hkx
//! ```
//! - Columns are split by tabs if the line has one (pasted from a spreadsheet), else by commas.
//! - `{{i}}` is the number of the line, from 0. A name no column has stays as it is written.
//! - Each object gets the next free `#code$N`: the `name="..."` of its head line is set to it.
//!
//! Drawn by the page's repeat window.

/// The values of a [`Repeat`]: the column names, and one row of values per object.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Values<'a> {
    names: Vec<&'a str>,
    pub rows: Vec<Vec<&'a str>>,
}

/// Reads the values as written. See the module docs.
///
/// e.g. `name, file\nA, a.hkx\n\nB, b.hkx` -> names `name`, `file`; rows `A a.hkx`, `B b.hkx`
pub fn values(text: &str) -> Values<'_> {
    let mut lines = text.lines().filter(|line| !line.trim().is_empty());
    let names = lines.next().map(split).unwrap_or_default();
    Values { names, rows: lines.map(split).collect() }
}

/// A line of values: split by tabs if it has one (pasted from a spreadsheet), else by commas.
fn split(line: &str) -> Vec<&str> {
    let separator = if line.contains('\t') { '\t' } else { ',' };
    line.split(separator).map(str::trim).collect()
}

/// `template` with the values of row `index` in place of `{{name}}`, and `index` in place of `{{i}}`.
///
/// e.g. `Attack_{{name}}_{{i}}`, names `name`, row `A`, index 2 -> `Attack_A_2`
pub fn expand(template: &str, values: &Values<'_>, index: usize) -> String {
    let mut out = template.replace("{{i}}", &index.to_string());
    let row = values.rows.get(index).map_or(&[][..], Vec::as_slice);
    for (name, value) in values.names.iter().zip(row) {
        out = out.replace(&format!("{{{{{name}}}}}"), value);
    }
    out
}

/// `object` with `id` as the `name` of its head line. e.g. `<hkobject name="#0106" ...>` -> `#m$3`
pub fn with_id(object: &str, id: &str) -> String {
    let Some(start) = object.find("name=\"").map(|at| at + "name=\"".len()) else { return object.to_owned() };
    let Some(len) = object[start..].find('"') else { return object.to_owned() };
    format!("{}{id}{}", &object[..start], &object[start + len..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_are_read_by_tabs_or_commas() {
        let read = values("name, file\nA, a.hkx\n\nB, b.hkx\n");
        assert_eq!(read.names, ["name", "file"]);
        assert_eq!(read.rows, [vec!["A", "a.hkx"], vec!["B", "b.hkx"]]);
        let pasted = values("name\tfile\nA, x\tb.hkx");
        assert_eq!(pasted.rows, [vec!["A, x", "b.hkx"]], "a tab: commas are text");
    }

    #[test]
    fn each_row_makes_an_object() {
        let read = values("name,file\nA,a.hkx\nB,b.hkx");
        let template =
            "<hkobject name=\"#0106\">\n<hkparam name=\"name\">Attack_{{name}}_{{i}}</hkparam>\n{{file}} {{other}}\n";
        let first = with_id(&expand(template, &read, 0), "#m$3");
        assert_eq!(first, "<hkobject name=\"#m$3\">\n<hkparam name=\"name\">Attack_A_0</hkparam>\na.hkx {{other}}\n");
        assert!(expand(template, &read, 1).contains("Attack_B_1"));
    }
}
