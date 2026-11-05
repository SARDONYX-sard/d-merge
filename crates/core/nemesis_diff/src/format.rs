//! Formats the text of one Havok XML object the way the templates are written, so that formatting
//! adds nothing to the patch: the diff is against the template, line by line, indentation included.
//!
//! Only what surrounds the values changes; the values are never touched.
//!
//! ```text
//! <hkobject name="#0106" class="hkbModifierList">      \t\t<hkobject name="#0106" class="hkbModifierList">
//! <hkparam name="modifiers" numelements="2">           \t\t\t<hkparam name="modifiers" numelements="2">
//!   #0107 #0108                                  ->    \t\t\t\t#0107
//!                                                      \t\t\t\t#0108
//! </hkparam>                                           \t\t\t</hkparam>
//! </hkobject>                                          \t\t</hkobject>
//! ```
//!
//! Nemesis' markers (`<!-- MOD_CODE ~id~ OPEN -->`, ...) stay as they are: the mod's author may have
//! them at any indentation, and the parser does not mind.

use crate::{CLOSE, ORIGINAL, normalize, open_kept_arrays};

/// How deep a top-level object is in a file: `<hkpackfile>`, `<hksection>`, then the object.
const OBJECT_DEPTH: usize = 2;

/// The formatted text of an object. `None` if its tags do not close (e.g., while it is being typed):
/// the indentation of what follows the mistake would be wrong everywhere.
pub fn format_object(text: &str) -> Option<String> {
    let normalized = normalize(text);
    // The tags of a push that kept the line of an empty array only balance like this.
    let balanced = open_kept_arrays(&normalized);
    let mut out = String::with_capacity(normalized.len() + normalized.len() / 8);
    let mut depth = OBJECT_DEPTH;
    for (line, balanced) in normalized.lines().zip(balanced.lines()) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            out.push('\n');
            continue;
        }
        if is_marker(trimmed) {
            out.push_str(line);
            out.push('\n');
            continue;
        }
        let (opened, closed) = if trimmed.starts_with("<!--") { (0, 0) } else { tags_of(balanced.trim()) };
        let indent = if trimmed.starts_with("</") { depth.checked_sub(1)? } else { depth };
        out.extend(std::iter::repeat_n('\t', indent));
        out.push_str(trimmed);
        out.push('\n');
        depth = (depth + opened).checked_sub(closed)?;
    }
    (depth == OBJECT_DEPTH).then_some(out)
}

/// `<!-- MOD_CODE ~id~ OPEN -->`, `<!-- ORIGINAL -->`, `<!-- CLOSE -->`
fn is_marker(trimmed: &str) -> bool {
    trimmed.starts_with("<!-- MOD_CODE ") || trimmed == ORIGINAL || trimmed == CLOSE
}

/// How many tags the line opens and closes.
///
/// e.g., `<hkparam name="a">` -> `(1, 0)`, `<hkparam name="a">1</hkparam>` -> `(1, 1)`,
/// `<hkparam name="a"/>` -> `(1, 1)`, `</hkobject>` -> `(0, 1)`, `#0107` -> `(0, 0)`,
/// `<hkcstring>FootLeft</hkcstring>` -> `(0, 0)`: an element of an array, on its line.
fn tags_of(trimmed: &str) -> (usize, usize) {
    let opened = trimmed.matches("<hkobject").count() + trimmed.matches("<hkparam").count();
    let closed = trimmed.matches("</hkobject>").count()
        + trimmed.matches("</hkparam>").count()
        + usize::from(trimmed.ends_with("/>"));
    (opened, closed)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FORMATTED: &str = "\t\t<hkobject name=\"#0106\" class=\"hkbModifierList\" signature=\"0xa4180ca1\">\n\
                             \t\t\t<hkparam name=\"variableBindingSet\">null</hkparam>\n\
                             \t\t\t<hkparam name=\"modifiers\" numelements=\"2\">\n\
                             \t\t\t\t#0107\n\
                             \t\t\t\t#0108\n\
                             \t\t\t</hkparam>\n\
                             \t\t\t<hkparam name=\"triggers\" numelements=\"1\">\n\
                             \t\t\t\t<hkobject>\n\
                             \t\t\t\t\t<hkparam name=\"localTime\">0.5</hkparam>\n\
                             \t\t\t\t\t<hkparam name=\"event\"/>\n\
                             \t\t\t\t</hkobject>\n\
                             \t\t\t</hkparam>\n\
                             \t\t</hkobject>\n";

    #[test]
    fn what_is_formatted_stays() {
        assert_eq!(format_object(FORMATTED).as_deref(), Some(FORMATTED));
    }

    #[test]
    fn indents_by_the_tags() {
        let flat: String = FORMATTED.lines().map(|line| format!("  {} \n", line.trim())).collect();
        assert_eq!(format_object(&flat).as_deref(), Some(FORMATTED));
    }

    /// The template's form: otherwise they are changes in the patch.
    #[test]
    fn pointers_get_a_line_each() {
        let joined = FORMATTED.replace("#0107\n\t\t\t\t#0108", "#0107 #0108");
        assert_eq!(format_object(&joined).as_deref(), Some(FORMATTED));
    }

    #[test]
    fn values_and_comments_stay() {
        let text = "<hkobject name=\"#1\" class=\"a\">\n<!-- a  comment <hkparam -->\n\
                    <hkparam name=\"name\">a   b</hkparam>\n</hkobject>\n";
        assert_eq!(
            format_object(text).unwrap(),
            "\t\t<hkobject name=\"#1\" class=\"a\">\n\t\t\t<!-- a  comment <hkparam -->\n\
             \t\t\t<hkparam name=\"name\">a   b</hkparam>\n\t\t</hkobject>\n"
        );
    }

    #[test]
    fn markers_stay_where_they_are() {
        let text = "<hkobject name=\"#1\" class=\"a\">\n<!-- MOD_CODE ~m~ OPEN -->\n<hkparam name=\"a\">1</hkparam>\n\
                    \t<!-- ORIGINAL -->\n<hkparam name=\"a\">0</hkparam>\n<!-- CLOSE -->\n</hkobject>\n";
        assert_eq!(
            format_object(text).unwrap(),
            "\t\t<hkobject name=\"#1\" class=\"a\">\n<!-- MOD_CODE ~m~ OPEN -->\n\t\t\t<hkparam name=\"a\">1</hkparam>\n\
             \t<!-- ORIGINAL -->\n\t\t\t<hkparam name=\"a\">0</hkparam>\n<!-- CLOSE -->\n\t\t</hkobject>\n"
        );
    }

    /// A push to an empty array that keeps its line: see `kept_array`.
    #[test]
    fn a_kept_line_and_its_elements() {
        let formatted = "\t\t<hkobject name=\"#1\" class=\"a\">\n\
                         \t\t\t<hkparam name=\"a\" numelements=\"0\"></hkparam>\n\
                         \t\t\t\t<hkcstring>Hi</hkcstring>\n\
                         \t\t\t</hkparam>\n\
                         \t\t\t<hkparam name=\"b\" numelements=\"0\"></hkparam>\n\
                         \t\t</hkobject>\n";
        assert_eq!(format_object(formatted).as_deref(), Some(formatted));
        let flat: String = formatted.lines().map(|line| format!("{} \n", line.trim())).collect();
        assert_eq!(format_object(&flat).as_deref(), Some(formatted));
    }

    #[test]
    fn tags_that_do_not_close_leave_it_alone() {
        assert_eq!(format_object("<hkobject name=\"#1\" class=\"a\">\n<hkparam name=\"a\">\n</hkobject>\n"), None);
        assert_eq!(format_object("</hkparam>\n</hkobject>\n</hkobject>\n"), None);
    }

    /// Every object of real templates must come out as it went in.
    #[test]
    #[ignore = "needs the templates of the repository's resource"]
    fn the_templates_are_formatted_already() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../resource/xml/templates/meshes/actors/character");
        for file in ["behaviors/0_master.xml", "_1stperson/behaviors/0_master.xml", "characters/defaultmale.xml"] {
            let xml = std::fs::read_to_string(root.join(file)).unwrap();
            let objects = crate::objects(&xml);
            assert!(!objects.is_empty(), "{file}");
            for object in objects {
                let expected = normalize(object.text);
                let formatted = format_object(object.text);
                assert_eq!(formatted.as_deref(), Some(expected.as_str()), "{file} {}", object.id);
            }
        }
    }
}
