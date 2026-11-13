//! The files: `patches/<pack file>.xml` and `info.xml`.

use std::fmt::Write as _;

use crate::{
    diff::{Op, OpKind},
    tree::Element,
};

/// A patch file: the new objects, then the operations.
///
/// `code`: the root element has it, as the mods have it, though Pandora only looks inside.
#[must_use]
pub fn patch_file(code: &str, new_objects: &[Element<'_>], ops: &[Op<'_>]) -> String {
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n");
    let _ = writeln!(out, "<{code}>");
    if !new_objects.is_empty() {
        out.push_str("\t<Insert path=\"\">\n");
        for object in new_objects {
            object.write_to(&mut out, 2);
        }
        out.push_str("\t</Insert>\n");
    }
    for op in ops {
        write_op(&mut out, op);
    }
    let _ = writeln!(out, "</{code}>");
    out
}

/// One operation as it is in a file, at its indentation there: one tab, what it holds two.
#[must_use]
pub fn op_text(op: &Op<'_>) -> String {
    let mut out = String::new();
    write_op(&mut out, op);
    out
}

pub(crate) fn write_op(out: &mut String, op: &Op<'_>) {
    let tag = op.kind.tag();
    let path = &op.path;
    if !op.words.is_empty() {
        // e.g. `<Append path="#0340/states">` / `#dmco$0` / `</Append>`, as the Pandora mods write it.
        let _ = writeln!(out, "\t<{tag} path=\"{path}\">\n\t\t{}\n\t</{tag}>", op.words.join(" "));
        return;
    }
    if op.kind == OpKind::Remove || op.elements.is_empty() {
        let _ = writeln!(out, "\t<{tag} path=\"{path}\" />");
        return;
    }
    let _ = writeln!(out, "\t<{tag} path=\"{path}\">");
    for element in &op.elements {
        element.write_to(out, 2);
    }
    let _ = writeln!(out, "\t</{tag}>");
}

/// `info.xml`. e.g. `<mod code="amco"><name>Attack MCO</name>...</mod>`
///
/// Pandora writes `null` for what is not known.
#[must_use]
pub fn info_xml(code: &str, name: &str, author: &str, site: &str) -> String {
    let or_null = |value: &str| {
        let value = value.trim();
        if value.is_empty() { "null".to_owned() } else { escape(value) }
    };
    format!(
        "<mod code=\"{code}\">\n\t<name>{}</name>\n\t<author>{}</author>\n\t<site>{}</site>\n</mod>\n",
        or_null(name),
        or_null(author),
        or_null(site)
    )
}

/// What can't be in the text of an element.
fn escape(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        apply::apply_ops,
        diff::object_ops,
        tree::{parse, parse_all},
    };

    #[test]
    fn a_file_reads_as_what_was_written() {
        let template = r##"<hkobject name="#0001" class="c"><hkparam name="a">1</hkparam><hkparam name="b" numelements="1"><hkcstring>x</hkcstring></hkparam></hkobject>"##;
        let edited = template
            .replace(r#"name="a">1"#, r#"name="a">2"#)
            .replace("<hkcstring>x</hkcstring>", "<hkcstring>x</hkcstring><hkcstring>y</hkcstring>");
        let ops = object_ops(template, &edited).unwrap();
        let new_object =
            parse(r##"<hkobject name="#m$0" class="d"><hkparam name="q">1</hkparam></hkobject>"##).unwrap();

        let file = patch_file("amco", &[new_object], &ops);

        // As Pandora reads it: the operations under the root.
        let root = parse(&file).unwrap();
        assert_eq!(root.tag, "amco");
        assert_eq!(root.children[0].tag, "Insert");
        assert_eq!(root.children[0].attribute("path"), Some(""));
        assert_eq!(root.children[0].children[0].name(), Some("#m$0"));
        assert_eq!(root.children[1].tag, "Replace");
        assert_eq!(root.children[1].attribute("path"), Some("#0001/a"));
        assert_eq!(root.children[2].tag, "Append");
        assert_eq!(root.children[2].children[0].words, ["y"]);

        let mut object = parse(template).unwrap();
        apply_ops(&mut object, &ops).unwrap();
        assert_eq!(object.child("b").unwrap().children.len(), 2);
        assert_eq!(parse_all(&file).unwrap().len(), 1);
    }

    #[test]
    fn a_remove_has_nothing_in_it() {
        let op =
            Op { kind: OpKind::Remove, path: "#0001/a/Element0".to_owned(), elements: Vec::new(), words: Vec::new() };
        assert!(patch_file("amco", &[], &[op]).contains("<Remove path=\"#0001/a/Element0\" />"));
    }

    #[test]
    fn info_says_null_for_what_is_not_known() {
        let info = info_xml("amco", "A & B", "", " ");
        assert!(info.contains("<mod code=\"amco\">"));
        assert!(info.contains("<name>A &amp; B</name>"));
        assert!(info.contains("<author>null</author>"));
        assert!(info.contains("<site>null</site>"));
    }
}
