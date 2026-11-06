//! Pushing to an array that is empty in the template, Nemesis' way: its line stays as it is, and the
//! elements follow it with a `</hkparam>` of their own.
//!
//! ```text
//! <hkparam name="eventNames" numelements="0"></hkparam>   <- the template's line, unchanged
//!     <hkcstring>Hi</hkcstring>                           <- what the mod adds
//! </hkparam>                                              <- closes what the line above closed already
//! ```
//!
//! As a patch, only what is added is in the block, so several mods can add to the array:
//!
//! ```text
//! <hkparam name="eventNames" numelements="0"></hkparam>
//! <!-- MOD_CODE ~mymod~ OPEN -->
//!     <hkcstring>Hi</hkcstring>
//! </hkparam>
//! <!-- CLOSE -->
//! ```
//!
//! The tags of such a text do not balance, so what walks them reads it through [`open_kept_arrays`].

use std::borrow::Cow;

/// The line of an array that is empty in the template.
const KEPT_LINE_END: &str = "numelements=\"0\"></hkparam>";

/// `text` with the `</hkparam>` of the kept lines removed, so that its tags balance. Every line stays
/// on its line number. A kept line with no elements after it is simply an empty array, and stays.
///
/// e.g., `numelements="0"></hkparam>\n<hkcstring>Hi</hkcstring>\n</hkparam>\n`
/// -> `numelements="0">\n<hkcstring>Hi</hkcstring>\n</hkparam>\n`
pub fn open_kept_arrays(text: &str) -> Cow<'_, str> {
    if !text.contains(KEPT_LINE_END) {
        return Cow::Borrowed(text);
    }
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let is_kept = |at: usize| is_kept_line(&lines, at);
    if !(0..lines.len()).any(is_kept) {
        return Cow::Borrowed(text);
    }
    let mut out = String::with_capacity(text.len());
    for (at, line) in lines.iter().enumerate() {
        if is_kept(at) {
            out.push_str(line.replacen("</hkparam>", "", 1).as_str());
        } else {
            out.push_str(line);
        }
    }
    Cow::Owned(out)
}

/// Is 0-based `line` part of a push that kept the line of an empty array: that line, an element or
/// the `</hkparam>` that ends them? An array still without its `</hkparam>` only has its line.
pub fn is_in_kept_array(text: &str, line: usize) -> bool {
    if !text.contains(KEPT_LINE_END) {
        return false;
    }
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    (0..lines.len().min(line + 1)).filter(|&at| is_kept_line(&lines, at)).any(|at| {
        let last = array_end(&lines[at + 1..]).map_or(at, |end| at + 1 + end.close);
        line <= last
    })
}

pub(crate) fn is_kept_line(lines: &[&str], at: usize) -> bool {
    lines[at].trim_end().ends_with(KEPT_LINE_END) && elements_follow(&lines[at + 1..])
}

/// Do the lines after a kept line belong to its array? The first line that says anything tells: only
/// the next field, the end of the object or the next object is not an element, as these end an empty
/// array. A `</hkparam>` nothing opened is the end of its elements.
fn elements_follow(rest: &[&str]) -> bool {
    for line in rest {
        let line = line.trim();
        if line.is_empty() || line.starts_with("<!--") {
            continue;
        }
        let closes_something_else = line.starts_with("</hk") && !line.starts_with("</hkparam>");
        return !(line.starts_with("<hkparam") || line.starts_with("<hkobject ") || closes_something_else);
    }
    false
}

/// Where an array ends. `lines`: what follows its opening line.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ArrayEnd {
    /// The index of its `</hkparam>`.
    pub(crate) close: usize,
    /// Is there anything before it?
    pub(crate) has_elements: bool,
}

/// `None`: it does not close (it ran into the next field), e.g. while it is being typed.
pub(crate) fn array_end(lines: &[&str]) -> Option<ArrayEnd> {
    let mut has_elements = false;
    let mut objects = 0_usize; // How deep in the objects of the array.
    for (at, line) in lines.iter().enumerate() {
        let line = line.trim();
        if objects > 0 {
            if line.starts_with("</hkobject>") {
                objects -= 1;
            } else if line.starts_with("<hkobject") {
                objects += usize::from(!line.ends_with("/>"));
            }
        } else if line.starts_with("</hkparam>") {
            return Some(ArrayEnd { close: at, has_elements });
        } else if line.starts_with("<hkparam") || line.starts_with("</hk") {
            return None;
        } else if line.starts_with("<hkobject") {
            has_elements = true;
            objects += usize::from(!line.ends_with("/>"));
        } else if !line.is_empty() && !line.starts_with("<!--") {
            has_elements = true;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEPT: &str = "\t\t\t<hkparam name=\"a\" numelements=\"0\"></hkparam>\n";
    const OPENED: &str = "\t\t\t<hkparam name=\"a\" numelements=\"0\">\n";

    #[test]
    fn opens_a_kept_line_with_elements_after_it() {
        for rest in [
            "\t\t\t\t<hkcstring>Hi</hkcstring>\n\t\t\t</hkparam>\n",
            "\t\t\t\t<hkcstring>Hi</hkcstring>\n", // Being typed: no `</hkparam>` yet.
            "\t\t\t\t<\n\t\t\t<hkparam name=\"b\">1</hkparam>\n", // The first key of an element.
            "\t\t\t\t<hkobject>\n\t\t\t\t</hkobject>\n\t\t\t</hkparam>\n",
            "\n\t\t\t\t#0001\n\t\t\t</hkparam>\n",
            "\t\t\t</hkparam>\n",
        ] {
            let text = format!("{KEPT}{rest}");
            assert_eq!(open_kept_arrays(&text), format!("{OPENED}{rest}"), "{rest:?}");
        }
    }

    #[test]
    fn leaves_an_empty_array_alone() {
        for rest in [
            "\t\t\t<hkparam name=\"b\">1</hkparam>\n",
            "\t\t</hkobject>\n",
            "\t\t<hkobject name=\"#1\" class=\"a\">\n",
            "\n\t\t</hkobject>\n",
            "",
        ] {
            let text = format!("{KEPT}{rest}");
            assert!(matches!(open_kept_arrays(&text), Cow::Borrowed(_)), "{rest:?}");
        }
    }

    #[test]
    fn tells_the_lines_of_a_push() {
        let text = "\t\t<hkobject name=\"#1\" class=\"a\">\n\
                    \t\t\t<hkparam name=\"a\" numelements=\"0\"></hkparam>\n\
                    \t\t\t\t<hkcstring>Hi</hkcstring>\n\
                    \t\t\t</hkparam>\n\
                    \t\t\t<hkparam name=\"b\" numelements=\"0\"></hkparam>\n\
                    \t\t</hkobject>\n";
        let lines: Vec<bool> = (0..7).map(|line| is_in_kept_array(text, line)).collect();
        assert_eq!(lines, [false, true, true, true, false, false, false], "an empty array alone is not one");
        assert!(!is_in_kept_array("\t\t<hkobject>\n", 0));
    }

    #[test]
    fn finds_the_end_of_an_array() {
        let end = |text: &str| array_end(&text.lines().collect::<Vec<_>>());
        assert_eq!(end("</hkparam>\n"), Some(ArrayEnd { close: 0, has_elements: false }));
        assert_eq!(end("\n<!-- a -->\n</hkparam>\n"), Some(ArrayEnd { close: 2, has_elements: false }));
        assert_eq!(end("#0001\n</hkparam>\n#0002\n"), Some(ArrayEnd { close: 1, has_elements: true }));
        let objects = "<hkobject>\n<hkparam name=\"a\">\n1 2\n</hkparam>\n</hkobject>\n</hkparam>\n";
        assert_eq!(end(objects), Some(ArrayEnd { close: 5, has_elements: true }), "not the field inside");
        assert_eq!(end("#0001\n<hkparam name=\"b\">1</hkparam>\n"), None, "never closed");
        assert_eq!(end("#0001\n</hkobject>\n"), None);
    }
}
