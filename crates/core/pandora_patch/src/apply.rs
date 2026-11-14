//! What Pandora does with operations, on one object: the check that [`crate::diff`] says what was edited.
//!
//! Pandora itself keeps a map from each path to its element, made before anything is changed. This walks
//! to the element each time instead, which is the same for what `diff` writes: it changes the positions
//! at the end of a list only, the last one first.

use crate::{
    diff::{Op, OpKind},
    tree::Element,
};

/// Does `ops` to `object`. `Err`: the path of what could not be done.
///
/// # Errors
/// If a path doesn't lead anywhere.
pub fn apply_ops<'a>(object: &mut Element<'a>, ops: &[Op<'a>]) -> Result<(), String> {
    for op in ops {
        apply_op(object, op).ok_or_else(|| op.path.clone())?;
    }
    Ok(())
}

fn apply_op<'a>(object: &mut Element<'a>, op: &Op<'a>) -> Option<()> {
    let (_id, keys) = op.path.split_once('/')?;
    let (parent_keys, last) = keys.rsplit_once('/').map_or(("", keys), |(parent, last)| (parent, last));
    let parent = walk(object, parent_keys)?;
    match op.kind {
        OpKind::Replace => {
            let position = parent.child_position(last)?;
            parent.children[position] = op.elements.first()?.clone();
        }
        OpKind::Remove => {
            let position = parent.child_position(last)?;
            parent.children.remove(position);
        }
        OpKind::Append => {
            let position = parent.child_position(last)?;
            let list = &mut parent.children[position];
            list.children.extend(op.elements.iter().cloned());
            list.words.extend(op.words.iter().copied());
        }
        OpKind::Insert => return None,
    }
    Some(())
}

fn walk<'e, 'a>(mut element: &'e mut Element<'a>, keys: &str) -> Option<&'e mut Element<'a>> {
    for key in keys.split('/').filter(|key| !key.is_empty()) {
        let position = element.child_position(key)?;
        element = &mut element.children[position];
    }
    Some(element)
}

/// Is what was edited in `object`? As in [`crate::diff`], a field `edited` leaves out is not looked at.
///
/// `numelements` means nothing here: Pandora counts the elements of a list when it writes it, and a list
/// of values gets its count with its values.
#[must_use]
pub fn has_edit(object: &Element<'_>, edited: &Element<'_>) -> bool {
    let not_count = |attribute: &&(&str, &str)| attribute.0 != "numelements";
    let same_attributes = object.attributes.iter().filter(not_count).eq(edited.attributes.iter().filter(not_count));
    if object.tag != edited.tag || !same_attributes || object.words != edited.words {
        return false;
    }
    let is_list = edited.attribute("numelements").is_some()
        || (!edited.children.is_empty() && edited.children.iter().all(|child| child.name().is_none()));
    if is_list {
        return object.children.len() == edited.children.len()
            && object.children.iter().zip(&edited.children).all(|(a, b)| has_edit(a, b));
    }
    edited
        .children
        .iter()
        .all(|new| object.children.iter().find(|old| old.name() == new.name()).is_some_and(|old| has_edit(old, new)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{diff::object_ops, tree::parse};

    /// The edit, said as operations and done again, is the edit.
    #[track_caller]
    fn round_trip(template: &str, edited: &str) {
        let ops = object_ops(template, edited).unwrap();
        let mut object = parse(template).unwrap();
        apply_ops(&mut object, &ops).unwrap();
        assert!(has_edit(&object, &parse(edited).unwrap()), "{object:#?}");
    }

    const TEMPLATE: &str = r##"<hkobject name="#0001" class="c" signature="0x1">
	<hkparam name="value">1</hkparam>
	<hkparam name="pointers" numelements="2">#1 #2</hkparam>
	<hkparam name="names" numelements="3">
		<hkcstring>a</hkcstring>
		<hkcstring>b</hkcstring>
		<hkcstring>c</hkcstring>
	</hkparam>
	<hkparam name="empty" numelements="0"></hkparam>
	<hkparam name="list" numelements="2">
		<hkobject><hkparam name="x">1</hkparam><hkparam name="y"><hkobject><hkparam name="z">1</hkparam></hkobject></hkparam></hkobject>
		<hkobject><hkparam name="x">2</hkparam><hkparam name="y"><hkobject><hkparam name="z">2</hkparam></hkobject></hkparam></hkobject>
	</hkparam>
</hkobject>"##;

    #[test]
    fn each_kind_of_edit_comes_out_as_it_was() {
        let edits = [
            (r#"name="value">1"#, r#"name="value">2"#),
            (r#"numelements="2">#1 #2"#, r#"numelements="3">#1 #2 #3"#),
            ("<hkcstring>b</hkcstring>", "<hkcstring>x</hkcstring>"),
            ("<hkcstring>c</hkcstring>", ""),
            (r#"<hkparam name="value">1</hkparam>"#, ""),
            ("<hkcstring>c</hkcstring>", "<hkcstring>c</hkcstring><hkcstring>d</hkcstring>"),
            (
                r#"name="empty" numelements="0"></hkparam>"#,
                r#"name="empty" numelements="1"><hkcstring>n</hkcstring></hkparam>"#,
            ),
            (r#"name="z">2"#, r#"name="z">9"#),
        ];
        for (old, new) in edits {
            round_trip(TEMPLATE, &TEMPLATE.replace(old, new));
        }
    }

    #[test]
    fn a_list_can_lose_all_its_elements() {
        let template = r##"<hkobject name="#1" class="c"><hkparam name="l" numelements="2"><hkcstring>a</hkcstring><hkcstring>b</hkcstring></hkparam></hkobject>"##;
        round_trip(
            template,
            r##"<hkobject name="#1" class="c"><hkparam name="l" numelements="0"></hkparam></hkobject>"##,
        );
    }

    #[test]
    fn a_path_that_leads_nowhere_is_told() {
        let mut object = parse(TEMPLATE).unwrap();
        let op = Op {
            kind: OpKind::Remove,
            path: "#0001/none/Element0".to_owned(),
            elements: Vec::new(),
            words: Vec::new(),
        };
        assert_eq!(apply_ops(&mut object, &[op]), Err("#0001/none/Element0".to_owned()));
    }

    #[test]
    fn a_real_difference_is_a_difference() {
        let a = parse(TEMPLATE).unwrap();
        let other = TEMPLATE.replace(r#"name="z">2"#, r#"name="z">3"#);
        let b = parse(&other).unwrap();
        assert!(!has_edit(&a, &b));
        assert!(has_edit(&a, &a));
    }
}
