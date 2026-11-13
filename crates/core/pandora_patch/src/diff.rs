//! What an edit of a template object is as Pandora's operations.
//!
//! Each object is compared with its template level by level, so that a patch says as little as it can:
//! what it doesn't touch, other mods can.
//!
//! | The field                       | changed                         | operation               |
//! | ------------------------------- | ------------------------------- | ----------------------- |
//! | a value                         | anyhow                          | `Replace` of the field  |
//! | a list of pointers, `#1 #2`     | more at the end                 | `Append` of the new ones |
//! |                                 | else                            | `Replace` of the field  |
//! | `<hkobject>`s, `<hkcstring>`s   | an element                      | what is inside of it    |
//! |                                 | more at the end                 | `Append`                |
//! |                                 | fewer at the end                | `Remove`, the last first |
//!
//! # Pointers pushed to a list
//! As the Pandora mods do: `<Append path="#0340/states">#dmco$0</Append>`. Then other mods can push
//! to the same list too: a `Replace` would drop what they pushed.
//!
//! Only to a list that has some already: Pandora reads no pointers at all of a list that says
//! `numelements="0"`, and an `Append` of text doesn't change it. So an empty list is replaced whole.

use std::fmt::Write as _;

use crate::tree::{Element, parse};

/// `Replace`, `Remove`, `Append` are what this writes. `Insert` adds new objects at the root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpKind {
    Insert,
    Replace,
    Remove,
    Append,
}

impl OpKind {
    /// The tag in a patch file.
    #[must_use]
    pub const fn tag(self) -> &'static str {
        match self {
            Self::Insert => "Insert",
            Self::Replace => "Replace",
            Self::Remove => "Remove",
            Self::Append => "Append",
        }
    }
}

/// One operation. e.g. `<Replace path="#2773/flags"><hkparam name="flags">0</hkparam></Replace>`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Op<'a> {
    pub kind: OpKind,
    /// e.g. `#2773/flags`
    pub path: String,
    /// What it puts there. Nothing for a `Remove`.
    pub elements: Vec<Element<'a>>,
    /// The words an `Append` adds to a list of values, e.g. `#dmco$0`. Then `elements` is empty.
    pub words: Vec<&'a str>,
}

/// Why an edit can't be said in this format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjectProblem {
    /// Its text isn't XML.
    NotWellFormed,
    /// Pandora has no operation for the object itself (a path needs a `/`), only for what is in it.
    ///
    /// e.g. another `class`, or a field the template doesn't have. A field left out is not this: as in
    /// a Nemesis patch, it stays as it is.
    ChangedAsAWhole,
}

impl std::fmt::Display for ObjectProblem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::NotWellFormed => "not well formed",
            Self::ChangedAsAWhole => "its class differs from the template's, or it has a field the template doesn't",
        })
    }
}

/// The operations from `template` to `edited`, both the text of one object.
///
/// # Errors
/// See [`ObjectProblem`].
pub fn object_ops<'a>(template: &'a str, edited: &'a str) -> Result<Vec<Op<'a>>, ObjectProblem> {
    let template = parse(template).ok_or(ObjectProblem::NotWellFormed)?;
    let edited = parse(edited).ok_or(ObjectProblem::NotWellFormed)?;
    let id = template.name().ok_or(ObjectProblem::NotWellFormed)?;

    if template.tag != edited.tag || template.attributes != edited.attributes || !has_its_fields(&template, &edited) {
        return Err(ObjectProblem::ChangedAsAWhole);
    }
    let mut ops = Ops::default();
    ops.fields(id, &template, &edited);
    Ok(ops.finish())
}

/// A new object, for the `<Insert path="">` of a file.
///
/// # Errors
/// See [`ObjectProblem`].
pub fn new_object(text: &str) -> Result<Element<'_>, ObjectProblem> {
    parse(text).ok_or(ObjectProblem::NotWellFormed)
}

/// Is each `hkparam` of `edited` one that `template` has too? It may leave some out.
fn has_its_fields(template: &Element<'_>, edited: &Element<'_>) -> bool {
    edited.children.iter().all(|new| template.children.iter().any(|old| old.tag == new.tag && old.name() == new.name()))
}

/// The operations, by the order they have to be done in: what is removed is counted from the elements
/// that were there, and is appended to after that.
#[derive(Debug, Default)]
struct Ops<'a> {
    replaces: Vec<Op<'a>>,
    removes: Vec<Op<'a>>,
    appends: Vec<Op<'a>>,
}

impl<'a> Ops<'a> {
    fn finish(mut self) -> Vec<Op<'a>> {
        self.replaces.append(&mut self.removes);
        self.replaces.append(&mut self.appends);
        self.replaces
    }

    fn replace(&mut self, path: String, element: &Element<'a>) {
        self.replaces.push(Op { kind: OpKind::Replace, path, elements: vec![element.clone()], words: Vec::new() });
    }

    /// What differs in each field `edited` has. See [`has_its_fields`].
    fn fields(&mut self, path: &str, template: &Element<'a>, edited: &Element<'a>) {
        for new in &edited.children {
            let Some(old) = template.children.iter().find(|old| old.name() == new.name()) else { continue };
            let mut field_path = String::with_capacity(path.len() + 16);
            let _ = write!(field_path, "{path}/{}", old.name().unwrap_or_default());
            self.field(field_path, old, new);
        }
    }

    /// One `hkparam`.
    fn field(&mut self, path: String, old: &Element<'a>, new: &Element<'a>) {
        let holds_elements = |element: &Element<'_>| element.words.is_empty();
        let is_list = |element: &Element<'_>| element.attribute("numelements").is_some();
        if !old.children.is_empty() || !new.children.is_empty() {
            let same_kind = old.children.iter().chain(&new.children).all(|child| child.name().is_none());
            if same_kind && holds_elements(old) && holds_elements(new) && is_list(old) == is_list(new) {
                self.elements(&path, old, new);
            } else {
                self.replace(path, new);
            }
            return;
        }
        if old.words == new.words && old.attributes == new.attributes {
            return;
        }
        // Pointers pushed to a list that has some: see the module docs.
        let pushed = is_list(old)
            && is_list(new)
            && !old.words.is_empty()
            && new.words.len() > old.words.len()
            && new.words.starts_with(&old.words);
        if pushed {
            let words = new.words[old.words.len()..].to_vec();
            self.appends.push(Op { kind: OpKind::Append, path, elements: Vec::new(), words });
        } else {
            self.replace(path, new);
        }
    }

    /// `<hkobject>`s or `<hkcstring>`s, compared by their positions.
    fn elements(&mut self, path: &str, old: &Element<'a>, new: &Element<'a>) {
        let common = old.children.len().min(new.children.len());
        for (index, (old_child, new_child)) in old.children.iter().zip(&new.children).enumerate() {
            let element_path = format!("{path}/Element{index}");
            if old_child.tag == new_child.tag
                && old_child.attributes == new_child.attributes
                && has_its_fields(old_child, new_child)
            {
                if old_child.children.is_empty() {
                    if old_child.words != new_child.words {
                        self.replace(element_path, new_child);
                    }
                } else {
                    self.fields(&element_path, old_child, new_child);
                }
            } else {
                self.replace(element_path, new_child);
            }
        }
        for index in (common..old.children.len()).rev() {
            self.removes.push(Op {
                kind: OpKind::Remove,
                path: format!("{path}/Element{index}"),
                elements: Vec::new(),
                words: Vec::new(),
            });
        }
        if new.children.len() > common {
            self.appends.push(Op {
                kind: OpKind::Append,
                path: path.to_owned(),
                elements: new.children[common..].to_vec(),
                words: Vec::new(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn object(fields: &str) -> String {
        format!(r##"<hkobject name="#0001" class="hkbStateMachine" signature="0x1">{fields}</hkobject>"##)
    }

    /// `kind path` of each operation.
    fn ops(template: &str, edited: &str) -> Vec<String> {
        let (template, edited) = (object(template), object(edited));
        let ops = object_ops(&template, &edited).unwrap();
        ops.iter().map(|op| format!("{} {}", op.kind.tag(), op.path)).collect()
    }

    #[test]
    fn nothing_changed_is_no_operation() {
        let fields = r#"<hkparam name="a">1</hkparam>"#;
        assert_eq!(ops(fields, fields), Vec::<String>::new());
        // Only whitespace differs.
        assert_eq!(
            ops(
                r#"<hkparam name="a" numelements="2">#1 #2</hkparam>"#,
                "<hkparam name=\"a\" numelements=\"2\">\n#1\n#2\n</hkparam>"
            ),
            Vec::<String>::new()
        );
    }

    #[test]
    fn a_value_is_replaced() {
        assert_eq!(ops(r#"<hkparam name="a">1</hkparam>"#, r#"<hkparam name="a">2</hkparam>"#), ["Replace #0001/a"]);
    }

    /// Pushed: an `Append` of the new pointers, as the Pandora mods do. Else replaced whole.
    #[test]
    fn pointers_pushed_to_a_list_are_appended() {
        let states = |pointers: &str| {
            let count = pointers.split_whitespace().count();
            object(&format!(r#"<hkparam name="states" numelements="{count}">{pointers}</hkparam>"#))
        };
        let (template, edited) = (states("#1"), states("#1 #m$0 #m$1"));
        let pushed = object_ops(&template, &edited).unwrap();
        assert_eq!(pushed.len(), 1);
        assert_eq!((pushed[0].kind, pushed[0].path.as_str()), (OpKind::Append, "#0001/states"));
        assert_eq!(pushed[0].words, ["#m$0", "#m$1"]);
        assert!(pushed[0].elements.is_empty());

        let kinds = |template: &str, edited: &str| ops(template, edited);
        let list = |pointers: &str| {
            let count = pointers.split_whitespace().count();
            format!(r#"<hkparam name="states" numelements="{count}">{pointers}</hkparam>"#)
        };
        // Pandora reads nothing of a list that says `numelements="0"`.
        assert_eq!(kinds(&list(""), &list("#m$0")), ["Replace #0001/states"], "an empty list");
        assert_eq!(kinds(&list("#1 #2"), &list("#2 #1")), ["Replace #0001/states"], "reordered");
        assert_eq!(kinds(&list("#1 #2"), &list("#1")), ["Replace #0001/states"], "fewer");
        assert_eq!(kinds(&list("#1"), &list("#m$0 #1")), ["Replace #0001/states"], "not at the end");
    }

    #[test]
    fn a_field_in_an_element_has_the_whole_way_to_it() {
        let inner = |value: &str| {
            format!(
                r#"<hkparam name="transitions" numelements="1"><hkobject><hkparam name="interval"><hkobject><hkparam name="id">{value}</hkparam></hkobject></hkparam></hkobject></hkparam>"#
            )
        };
        assert_eq!(ops(&inner("1"), &inner("2")), ["Replace #0001/transitions/Element0/interval/Element0/id"]);
    }

    #[test]
    fn elements_are_appended_and_removed_at_the_end() {
        let names = |names: &[&str]| {
            let names: String = names.iter().map(|name| format!("<hkcstring>{name}</hkcstring>")).collect();
            format!(r#"<hkparam name="names" numelements="{}">{names}</hkparam>"#, names.len())
        };
        assert_eq!(ops(&names(&["a"]), &names(&["a", "b", "c"])), ["Append #0001/names"]);
        // The last one first, so that the others are where they were.
        assert_eq!(
            ops(&names(&["a", "b", "c"]), &names(&["a"])),
            ["Remove #0001/names/Element2", "Remove #0001/names/Element1"]
        );
        assert_eq!(ops(&names(&["a", "b"]), &names(&["x", "b"])), ["Replace #0001/names/Element0"]);
        // What a list starts with.
        assert_eq!(ops(r#"<hkparam name="names" numelements="0"></hkparam>"#, &names(&["a"])), ["Append #0001/names"]);
    }

    #[test]
    fn a_replace_comes_before_what_changes_the_positions() {
        let names = |names: &str, count: usize| {
            format!(r#"<hkparam name="names" numelements="{count}">{names}</hkparam><hkparam name="a">1</hkparam>"#)
        };
        let template = names("<hkcstring>a</hkcstring><hkcstring>b</hkcstring>", 2);
        let edited = names("<hkcstring>x</hkcstring>", 1).replace(r#"name="a">1"#, r#"name="a">2"#);
        assert_eq!(
            ops(&template, &edited),
            ["Replace #0001/names/Element0", "Replace #0001/a", "Remove #0001/names/Element1"]
        );
    }

    #[test]
    fn a_field_left_out_stays_as_it_is() {
        let both = r#"<hkparam name="a">1</hkparam><hkparam name="b">1</hkparam>"#;
        assert_eq!(ops(both, r#"<hkparam name="b">2</hkparam>"#), ["Replace #0001/b"]);
        assert_eq!(ops(both, ""), Vec::<String>::new());
    }

    #[test]
    fn what_has_no_operation_is_a_problem() {
        let a = object(r#"<hkparam name="a">1</hkparam>"#);
        let more = object(r#"<hkparam name="a">1</hkparam><hkparam name="b">1</hkparam>"#);
        assert_eq!(object_ops(&a, &more), Err(ObjectProblem::ChangedAsAWhole));
        let class = a.replace("hkbStateMachine", "hkbClipGenerator");
        assert_eq!(object_ops(&a, &class), Err(ObjectProblem::ChangedAsAWhole));
        assert_eq!(object_ops(&a, "<a>"), Err(ObjectProblem::NotWellFormed));
    }
}
