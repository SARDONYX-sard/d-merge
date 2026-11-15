//! A patch file as a whole: what it does to the template's objects, and writing an object's part anew.
//!
//! A work holds edits of objects, whichever format they are exported in. So a Pandora mod is read by doing
//! its operations on the template ([`apply_doc`]), and an object edited afterwards changes only the
//! operations of that object in the file ([`replace_object_ops`]): the rest stays as it was written.
//!
//! # The order of operations
//! Pandora knows each path from before anything changed, so `Element1` is the same element after a
//! `Remove` of `Element0`. To have the same here: what replaces comes first, then what moves elements, the
//! last in a list first, then what appends. What points into what another operation adds is tried again
//! at the end.
//!
//! # `skip`
//! `<Replace path="#1/states" skip="11"><old>#0107</old><new>#m$0</new></Replace>` counts chars in the text
//! as Pandora has it, whose whitespace isn't known here. So the place is an estimate, among the places
//! that have the text. It only matters if the same text is there many times, and is only for what the
//! editor shows: the file keeps `skip` as it is.

use std::{collections::BTreeMap, ops::Range};

use crate::{
    diff::{Op, OpKind, object_ops},
    path::split_path,
    tree::{Element, parse},
    write::{patch_file, write_op},
};

/// One operation as written in a file.
#[derive(Debug, Clone)]
struct DocOp<'d, 'a> {
    kind: OpKind,
    element: &'d Element<'a>,
    path: &'a str,
}

/// What a patch file does.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AppliedDoc {
    /// Template objects it changes, as they are after it: `(id, text)`.
    pub edited: Vec<(String, String)>,
    /// New objects: `(id, text)`.
    pub added: Vec<(String, String)>,
    /// The paths that lead nowhere. e.g. `#0001/none`
    pub failed: Vec<String>,
}

fn kind_of(tag: &str) -> Option<OpKind> {
    [OpKind::Insert, OpKind::Replace, OpKind::Remove, OpKind::Append]
        .into_iter()
        .find(|kind| kind.tag().eq_ignore_ascii_case(tag))
}

/// The operations in the order written, as Pandora looks for them: `<Replace path="...">` anywhere, or
/// `<append><edit path="...">`, where the tag says what is done to each of what is in it.
fn collect_ops<'d, 'a>(parent: &'d Element<'a>, out: &mut Vec<DocOp<'d, 'a>>) {
    for element in &parent.children {
        let Some(kind) = kind_of(element.tag) else {
            collect_ops(element, out);
            continue;
        };
        if let Some(path) = element.attribute("path") {
            out.push(DocOp { kind, element, path });
        } else {
            out.extend(
                element
                    .children
                    .iter()
                    .filter_map(|edit| Some(DocOp { kind, element: edit, path: edit.attribute("path")? })),
            );
        }
    }
}

/// Goes to what `keys` lead to. All it passes will be written anew: see [`Element::write_preserved`].
fn walk_mut<'e, 'a>(mut element: &'e mut Element<'a>, keys: &[&str]) -> Option<&'e mut Element<'a>> {
    element.raw = None;
    for key in keys {
        let position = element.child_position(key)?;
        element = &mut element.children[position];
        element.raw = None;
    }
    Some(element)
}

/// Where in the words `needle` is, as near `skip` chars as it can be told. See the module docs.
fn find_words(words: &[&str], needle: &[&str], skip: usize) -> Option<usize> {
    if needle.is_empty() {
        return None;
    }
    let mut offset = 0;
    let mut found = None;
    for (at, window) in words.windows(needle.len()).enumerate() {
        if window == needle {
            // The last one before it: the text had at least this much whitespace.
            if offset <= skip || found.is_none() {
                found = Some(at);
            }
        }
        offset += window[0].len() + 1;
    }
    found
}

fn apply_op<'a>(root: &mut Element<'a>, op: &DocOp<'_, 'a>) -> Option<()> {
    let (_, keys) = split_path(op.path);
    let keys: Vec<&str> = keys.collect();
    let (&last, parents) = keys.split_last()?;
    let skip = op.element.attribute("skip");
    let new_elements = || op.element.children.iter().map(Element::without_raw);
    match op.kind {
        // `<old>` and `<new>`
        OpKind::Replace if skip.is_some() => {
            let [old, new] = op.element.children.as_slice() else { return None };
            let target = walk_mut(root, &keys)?;
            let at = find_words(&target.words, &old.words, skip?.parse().unwrap_or(0))?;
            target.words.splice(at..at + old.words.len(), new.words.iter().copied());
        }
        OpKind::Replace => {
            let parent = walk_mut(root, parents)?;
            let position = parent.child_position(last)?;
            parent.children.splice(position..=position, new_elements());
        }
        OpKind::Remove if skip.is_some() => {
            let target = walk_mut(root, &keys)?;
            let at = find_words(&target.words, &op.element.words, skip?.parse().unwrap_or(0))?;
            target.words.drain(at..at + op.element.words.len());
        }
        OpKind::Remove => {
            let parent = walk_mut(root, parents)?;
            let position = parent.child_position(last)?;
            parent.children.remove(position);
        }
        OpKind::Insert if op.element.children.is_empty() => {
            let target = walk_mut(root, &keys)?;
            let index: usize = op.element.attribute("index").and_then(|index| index.parse().ok()).unwrap_or(0);
            // `index` counts chars as `skip` does: see the module docs.
            let mut chars = 0;
            let at = target.words.iter().position(|word| {
                let reached = chars >= index;
                chars += word.len() + 1;
                reached
            });
            let at = at.unwrap_or(target.words.len());
            target.words.splice(at..at, op.element.words.iter().copied());
        }
        OpKind::Insert => {
            let parent = walk_mut(root, parents)?;
            let position = parent.child_position(last)?;
            parent.children.splice(position..position, new_elements());
        }
        OpKind::Append if op.element.children.is_empty() => {
            walk_mut(root, &keys)?.words.extend(op.element.words.iter().copied());
        }
        OpKind::Append => {
            walk_mut(root, &keys)?.children.extend(new_elements());
        }
    }
    Some(())
}

/// When an operation is done: see the module docs. e.g. `Remove .../Element3` before `.../Element2`.
fn order_key(op: &DocOp<'_, '_>) -> (u8, std::cmp::Reverse<usize>) {
    let moves_elements = matches!(op.kind, OpKind::Remove | OpKind::Insert)
        && op.element.attribute("skip").is_none()
        && (op.kind == OpKind::Remove || !op.element.children.is_empty());
    let stage = match op.kind {
        OpKind::Append => 2,
        _ if moves_elements => 1,
        _ => 0,
    };
    let index =
        op.path.rsplit('/').next().and_then(|last| last.strip_prefix("Element")).and_then(|index| index.parse().ok());
    (stage, std::cmp::Reverse(index.unwrap_or(0)))
}

/// Does the operations of the patch file `doc` to what `template` has.
///
/// `None`: the file isn't well formed.
pub fn apply_doc<'a>(doc: &'a str, template: impl Fn(&str) -> Option<&'a str>) -> Option<AppliedDoc> {
    /// An object, its indentation, and whether it is new.
    struct Object<'a> {
        element: Element<'a>,
        level: usize,
        touched: bool,
        is_new: bool,
    }

    let root = parse(doc)?;
    let mut ops = Vec::new();
    collect_ops(&root, &mut ops);

    let mut objects: BTreeMap<&str, Object<'a>> = BTreeMap::new();
    let (new_objects, mut pending): (Vec<_>, Vec<_>) = ops.into_iter().partition(|op| op.path.is_empty());
    for op in new_objects {
        for child in &op.element.children {
            let Some(id) = child.name() else { continue };
            objects.insert(id, Object { element: child.without_raw(), level: 0, touched: true, is_new: true });
        }
    }
    pending.sort_by_key(order_key); // stable: the rest stays in the order written

    let mut failed = Vec::new();
    for attempt in 0..2 {
        let mut again = Vec::new();
        for op in pending {
            let (id, _) = split_path(op.path);
            if !objects.contains_key(id) {
                let found = template(id).and_then(|text| {
                    let level = text.len() - text.trim_start_matches('\t').len();
                    Some(Object { element: parse(text)?, level, touched: false, is_new: false })
                });
                let Some(object) = found else {
                    failed.push(op.path.to_owned());
                    continue;
                };
                objects.insert(id, object);
            }
            let Some(object) = objects.get_mut(id) else { continue };
            if apply_op(&mut object.element, &op).is_some() {
                object.touched = true;
            } else if attempt == 0 {
                again.push(op);
            } else {
                failed.push(op.path.to_owned());
            }
        }
        pending = again;
    }

    let mut applied = AppliedDoc { failed, ..AppliedDoc::default() };
    for (id, object) in objects.iter().filter(|(_, object)| object.touched) {
        // Replaced by what was there: not an edit. Its text is the template's, as written.
        if !object.is_new && template(id).and_then(parse).is_some_and(|template| template == object.element) {
            continue;
        }
        let mut text = String::new();
        object.element.write_preserved(&mut text, object.level);
        let target = if object.is_new { &mut applied.added } else { &mut applied.edited };
        target.push(((*id).to_owned(), text));
    }
    Some(applied)
}

/// `edited` without operation `index` of [`object_ops`], for what is back as in `template`: the other way of
/// a change in the preview. `None`: there is no such operation, or the rest can't be done.
///
/// Done as a file is, so that the parts nothing touches keep the template's text.
#[must_use]
pub fn without_op(template: &str, edited: &str, index: usize) -> Option<String> {
    let mut ops = object_ops(template, edited).ok()?;
    if index >= ops.len() {
        return None;
    }
    ops.remove(index);
    if ops.is_empty() {
        return Some(template.to_owned());
    }
    let doc = patch_file("revert", &[], &ops);
    let applied = apply_doc(&doc, |_| Some(template))?;
    if !applied.failed.is_empty() {
        return None;
    }
    applied.edited.into_iter().next().map(|(_, text)| text)
}

/// The objects a patch file is about. e.g. `#0001`, `#m$0`
#[must_use]
pub fn doc_object_ids(doc: &str) -> Vec<String> {
    let Some(root) = parse(doc) else { return Vec::new() };
    let mut ops = Vec::new();
    collect_ops(&root, &mut ops);
    let mut ids: Vec<String> = Vec::new();
    for op in ops {
        if op.path.is_empty() {
            ids.extend(op.element.children.iter().filter_map(Element::name).map(str::to_owned));
        } else {
            ids.push(split_path(op.path).0.to_owned());
        }
    }
    ids.sort_unstable();
    ids.dedup();
    ids
}

/// The bytes of `part` in `text`, the text it is a part of, with the whitespace before it on its line
/// and the end of that line.
fn line_range(text: &str, part: &str) -> Option<Range<usize>> {
    let start = (part.as_ptr() as usize).checked_sub(text.as_ptr() as usize)?;
    let end = start + part.len();
    let head = &text[..start];
    let line_start = head.trim_end_matches([' ', '\t']).len();
    let tail = &text[end..];
    let after_blanks = tail.trim_start_matches([' ', '\t', '\r']);
    let own_lines = line_start == 0 || head[..line_start].ends_with('\n');
    let line_end = if own_lines && after_blanks.starts_with('\n') { text.len() - after_blanks.len() + 1 } else { end };
    Some(if own_lines { line_start..line_end } else { start..end })
}

/// Writes the operations of the object `id` anew in the file `doc`, leaving all the rest as it is.
///
/// `new_object`: its text, if it is new. `None` if the file isn't well formed.
///
/// What is in `<append><edit path="...">` stays too: such a group is rarely seen, and has no line of its
/// own to take out of.
#[must_use]
pub fn replace_object_ops(doc: &str, id: &str, ops: &[Op<'_>], new_object: Option<&Element<'_>>) -> Option<String> {
    let root = parse(doc)?;
    let mut cuts = Vec::new();
    for op in &root.children {
        let Some(path) = op.attribute("path") else { continue };
        if path.is_empty() {
            let is_ours = |child: &Element<'_>| child.name() == Some(id);
            if !op.children.is_empty() && op.children.iter().all(is_ours) {
                cuts.push(line_range(doc, op.raw?)?);
            } else {
                for child in op.children.iter().filter(|child| is_ours(child)) {
                    cuts.push(line_range(doc, child.raw?)?);
                }
            }
        } else if split_path(path).0 == id {
            cuts.push(line_range(doc, op.raw?)?);
        }
    }

    let close = doc.rfind("</")?;
    let insert_at = doc[..close].trim_end_matches([' ', '\t']).len();
    let mut out = String::with_capacity(doc.len());
    let mut done = 0;
    for cut in &cuts {
        out.push_str(&doc[done..cut.start.min(insert_at)]);
        done = cut.end.min(insert_at).max(done);
    }
    out.push_str(&doc[done..insert_at]);
    if !out.ends_with('\n') {
        out.push('\n');
    }
    if let Some(object) = new_object {
        out.push_str("\t<Insert path=\"\">\n");
        object.without_raw().write_to(&mut out, 2);
        out.push_str("\t</Insert>\n");
    }
    for op in ops {
        write_op(&mut out, op);
    }
    out.push_str(&doc[insert_at..]);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A file written with pointers pushed (`<Append>` of words) is read back as that push.
    #[test]
    fn pushed_pointers_are_read_back() {
        let template =
            "<hkobject name=\"#1\" class=\"c\">\n\t<hkparam name=\"s\" numelements=\"1\">#2</hkparam>\n</hkobject>\n";
        let edited = template.replace(">#2<", ">#2 #m$0<");
        let ops = crate::object_ops(template, &edited).unwrap();
        let doc = crate::patch_file("m", &[], &ops);
        assert!(doc.contains("<Append path=\"#1/s\">\n\t\t#m$0\n\t</Append>"), "{doc}");
        let applied = apply_doc(&doc, |_| Some(template)).unwrap();
        let (_, text) = &applied.edited[0];
        assert!(parse(text).unwrap().children[0].words == ["#2", "#m$0"], "{text}");
    }

    /// Pandora's own mod has this: ` iState_NPCSneaking`, with the space, where the template has it too.
    #[test]
    fn replacing_by_the_same_is_no_edit() {
        let template = "<hkobject name=\"#1\" class=\"c\">\n\t<hkparam name=\"l\" numelements=\"1\">\n\t\t<hkcstring> a</hkcstring>\n\t</hkparam>\n</hkobject>\n";
        let doc =
            "<patch>\n\t<Replace path=\"#1/l/Element0\">\n\t\t<hkcstring> a</hkcstring>\n\t</Replace>\n</patch>\n";
        let applied = apply_doc(doc, |_| Some(template)).unwrap();
        assert_eq!(applied, AppliedDoc::default());
    }

    #[test]
    fn an_operation_can_be_taken_back_alone() {
        let template = "<hkobject name=\"#1\" class=\"c\">\n\t<hkparam name=\"a\">1</hkparam>\n\t<hkparam name=\"b\">1</hkparam>\n</hkobject>\n";
        let edited = template.replace(">1<", ">2<");

        let only_b = without_op(template, &edited, 0).unwrap();
        assert!(only_b.contains("name=\"a\">1<"), "{only_b}");
        assert!(only_b.contains("name=\"b\">2<"), "{only_b}");
        assert_eq!(without_op(&only_b, &only_b, 0), None, "no such operation");
        assert_eq!(
            without_op(template, &template.replace("\"a\">1", "\"a\">2"), 0).as_deref(),
            Some(template),
            "the last one"
        );
    }

    const OBJECT: &str = "\t\t<hkobject name=\"#0001\" class=\"c\" signature=\"0x1\">\n\
\t\t\t<hkparam name=\"keep\">same</hkparam>\n\
\t\t\t<hkparam name=\"value\">1</hkparam>\n\
\t\t\t<hkparam name=\"pointers\" numelements=\"3\">\n\
\t\t\t\t#1 #2 #3\n\
\t\t\t</hkparam>\n\
\t\t\t<hkparam name=\"names\" numelements=\"3\">\n\
\t\t\t\t<hkcstring>a</hkcstring>\n\
\t\t\t\t<hkcstring>b</hkcstring>\n\
\t\t\t\t<hkcstring>c</hkcstring>\n\
\t\t\t</hkparam>\n\
\t\t</hkobject>";

    fn applied(ops: &str) -> AppliedDoc {
        let doc = format!("<m>{ops}</m>");
        // The result has no borrow of `doc`.
        let doc: &'static str = Box::leak(doc.into_boxed_str());
        apply_doc(doc, |id| (id == "#0001").then_some(OBJECT)).unwrap()
    }

    fn edited(ops: &str) -> String {
        let applied = applied(ops);
        assert_eq!(applied.failed, Vec::<String>::new());
        applied.edited.into_iter().next().unwrap().1
    }

    #[test]
    fn what_is_not_touched_stays_as_it_was_written() {
        let text = edited(r##"<Replace path="#0001/value"><hkparam name="value">2</hkparam></Replace>"##);
        let expected = OBJECT.replace("name=\"value\">1<", "name=\"value\">2<");
        assert_eq!(text.trim_end(), expected);
    }

    #[test]
    fn a_list_gets_the_count_of_what_it_has() {
        let text = edited(r##"<Append path="#0001/names"><hkcstring>d</hkcstring></Append>"##);
        assert!(text.contains("name=\"names\" numelements=\"4\""), "{text}");
        assert!(text.contains("<hkcstring>d</hkcstring>"), "{text}");
        let text = edited(r##"<Append path="#0001/pointers">#4</Append>"##);
        assert!(text.contains("name=\"pointers\" numelements=\"4\""), "{text}");
        assert!(text.contains("#1 #2 #3 #4"), "{text}");
    }

    #[test]
    fn paths_mean_the_elements_there_were_at_the_start() {
        let text = edited(
            r##"<Remove path="#0001/names/Element0" />
<Replace path="#0001/names/Element2"><hkcstring>x</hkcstring></Replace>
<Remove path="#0001/names/Element1" />"##,
        );
        let object = parse(&text).unwrap();
        let names: Vec<_> = object.child("names").unwrap().children.iter().map(|name| name.words[0]).collect();
        assert_eq!(names, ["x"]);
    }

    #[test]
    fn text_is_replaced_removed_and_inserted() {
        let replaced = edited(r##"<Replace path="#0001/pointers" skip="3"><old>#2</old><new>#9</new></Replace>"##);
        assert!(replaced.contains("#1 #9 #3"), "{replaced}");
        let removed = edited(r##"<Remove path="#0001/pointers" skip="0">#2</Remove>"##);
        assert!(removed.contains("#1 #3"), "{removed}");
        assert!(removed.contains("numelements=\"2\""), "{removed}");
        let inserted = edited(r##"<Insert path="#0001/pointers" index="0">#0</Insert>"##);
        assert!(inserted.contains("#0 #1 #2 #3"), "{inserted}");
    }

    #[test]
    fn new_objects_are_new_and_can_be_pointed_to() {
        let applied = applied(
            r##"<Insert path=""><hkobject name="#m$0" class="d"><hkparam name="q">1</hkparam></hkobject></Insert>
<Replace path="#m$0/q"><hkparam name="q">2</hkparam></Replace>"##,
        );
        assert_eq!(applied.added.len(), 1);
        assert_eq!(applied.added[0].0, "#m$0");
        assert!(applied.added[0].1.contains("name=\"q\">2<"), "{}", applied.added[0].1);
        assert!(applied.edited.is_empty());
    }

    #[test]
    fn a_group_of_a_kind_is_read_too() {
        let text = edited(r##"<append><edit path="#0001/names"><hkcstring>d</hkcstring></edit></append>"##);
        assert!(text.contains("<hkcstring>d</hkcstring>"), "{text}");
    }

    #[test]
    fn what_points_into_what_is_added_is_done_after_it() {
        let text = edited(
            r##"<Replace path="#0001/names/Element3"><hkcstring>y</hkcstring></Replace>
<Append path="#0001/names"><hkcstring>d</hkcstring></Append>"##,
        );
        assert!(text.contains("<hkcstring>y</hkcstring>") && !text.contains("<hkcstring>d</hkcstring>"), "{text}");
    }

    #[test]
    fn what_leads_nowhere_is_told() {
        let applied = applied(
            r##"<Replace path="#0001/none"><hkparam name="none">1</hkparam></Replace>
<Remove path="#9999/a" />"##,
        );
        assert_eq!(applied.failed, ["#9999/a", "#0001/none"]);
        assert!(applied.edited.is_empty());
        assert_eq!(apply_doc("<m><a></m>", |_| None), None);
    }

    #[test]
    fn the_objects_of_a_file_are_listed() {
        let ids = doc_object_ids(
            r##"<m><Insert path=""><hkobject name="#m$0"></hkobject></Insert><Remove path="#2/a" /><Remove path="#1/a" /><Remove path="#1/b" /></m>"##,
        );
        assert_eq!(ids, ["#1", "#2", "#m$0"]);
    }

    const DOC: &str = "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n\
<m>\n\
\t<!-- mine -->\n\
\t<Replace path=\"#0002/a\">\n\
\t\t<hkparam name=\"a\">1</hkparam>\n\
\t</Replace>\n\
\t<Replace path=\"#0001/value\">\n\
\t\t<hkparam name=\"value\">7</hkparam>\n\
\t</Replace>\n\
\t<Insert path=\"\">\n\
\t\t<hkobject name=\"#m$0\" class=\"d\"></hkobject>\n\
\t\t<hkobject name=\"#m$1\" class=\"d\"></hkobject>\n\
\t</Insert>\n\
</m>\n";

    #[test]
    fn only_the_operations_of_that_object_are_written_anew() {
        let edited_object = OBJECT.replace("name=\"value\">1<", "name=\"value\">9<");
        let ops = object_ops(OBJECT, &edited_object).unwrap();
        let doc = replace_object_ops(DOC, "#0001", &ops, None).unwrap();

        assert!(doc.contains("<!-- mine -->") && doc.contains("#0002/a"), "{doc}");
        assert!(!doc.contains(">7<"), "{doc}");
        assert!(doc.contains(">9<"), "{doc}");
        assert!(doc.trim_end().ends_with("</m>"), "{doc}");
        assert!(parse(&doc).is_some(), "{doc}");
        assert_eq!(doc.matches("path=\"#0001/value\"").count(), 1);
    }

    #[test]
    fn an_object_with_no_change_left_has_no_operation_left() {
        let doc = replace_object_ops(DOC, "#0001", &[], None).unwrap();
        assert!(!doc.contains("#0001"), "{doc}");
        assert!(doc.contains("#0002/a"), "{doc}");
        assert!(parse(&doc).is_some(), "{doc}");
    }

    #[test]
    fn a_new_object_is_written_anew_beside_the_others() {
        let object = parse("<hkobject name=\"#m$0\" class=\"d\"><hkparam name=\"q\">2</hkparam></hkobject>").unwrap();
        let doc = replace_object_ops(DOC, "#m$0", &[], Some(&object)).unwrap();
        assert!(doc.contains("name=\"#m$1\""), "{doc}");
        assert_eq!(doc.matches("name=\"#m$0\"").count(), 1, "{doc}");
        assert!(doc.contains("name=\"q\">2<"), "{doc}");
        assert!(parse(&doc).is_some(), "{doc}");

        // The last one of a group: the group goes with it.
        let doc = replace_object_ops(&doc, "#m$1", &[], None).unwrap();
        assert!(!doc.contains("#m$1"), "{doc}");
        assert!(parse(&doc).is_some(), "{doc}");
    }
}
