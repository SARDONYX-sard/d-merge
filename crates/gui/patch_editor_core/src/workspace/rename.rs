//! Renaming the mod code of a work: the ids of its new objects and its clips carry it.
//!
//! ```text
//! #oldc$3                        -> #newc$3                       (a new object, a pointer to it)
//! DefaultMale~1/Idle~oldc$0.txt  -> DefaultMale~1/Idle~newc$0.txt (a clip of the animation data)
//! oldc$0                         -> newc$0                        (its clip id, in its text)
//! ```
//! A mod has thousands of objects: the behaviors and their objects are done in parallel, counted in a
//! [`AtomicUsize`] for the progress bar.
//!
//! The files the work exported under the old code go at the next export, as [`Workspace::owned`] do:
//! the old `info.ini`s join them.

use std::{
    borrow::Cow,
    collections::BTreeMap,
    sync::atomic::{AtomicUsize, Ordering},
};

use rayon::prelude::*;

use super::{Behavior, Workspace};

/// `text` with every id of the code `old` given the code `new`: each `old$` that does not continue a
/// longer name (e.g. `#xoldc$1` is another code's), ignoring ASCII case. Borrowed if there is none.
///
/// e.g. `<hkparam name="generator">#oldc$3</hkparam>` -> `<hkparam name="generator">#newc$3</hkparam>`
pub fn rename_ids<'a>(text: &'a str, old: &str, new: &str) -> Cow<'a, str> {
    let needle = format!("{old}$");
    let is_id_at = |at: usize| {
        let bytes = text.as_bytes();
        bytes.get(at..at + needle.len()).is_some_and(|found| found.eq_ignore_ascii_case(needle.as_bytes()))
            && at.checked_sub(1).is_none_or(|before| !(bytes[before].is_ascii_alphanumeric() || bytes[before] == b'_'))
    };
    let Some(first) = (0..text.len()).find(|&at| is_id_at(at)) else { return Cow::Borrowed(text) };
    let mut out = String::with_capacity(text.len() + 8);
    out.push_str(&text[..first]);
    let mut at = first;
    while at < text.len() {
        if is_id_at(at) {
            out.push_str(new);
            out.push('$');
            at += needle.len();
        } else {
            // `old` is ASCII, so a match starts on a char boundary: copy up to the next one.
            let len = text[at..].chars().next().map_or(1, char::len_utf8);
            out.push_str(&text[at..at + len]);
            at += len;
        }
    }
    Cow::Owned(out)
}

/// How many objects [`Workspace::rename_code`] goes through: the total of its progress bar.
pub fn object_count(workspace: &Workspace) -> usize {
    workspace.behaviors.iter().map(|behavior| behavior.edits.len() + behavior.added.len()).sum()
}

/// `text` renamed in place, if it has an id of `old`.
fn rename_in_place(text: &mut String, old: &str, new: &str) {
    if let Cow::Owned(renamed) = rename_ids(text, old, new) {
        *text = renamed;
    }
}

/// One behavior's objects. `done`: counts each one.
fn rename_behavior(behavior: &mut Behavior, old: &str, new: &str, done: &AtomicUsize) {
    behavior.edits.par_iter_mut().for_each(|(_, text)| {
        rename_in_place(text, old, new);
        done.fetch_add(1, Ordering::Relaxed);
    });
    let added = std::mem::take(&mut behavior.added);
    behavior.added = added
        .into_par_iter()
        .map(|(id, mut text)| {
            rename_in_place(&mut text, old, new);
            done.fetch_add(1, Ordering::Relaxed);
            (rename_ids(&id, old, new).into_owned(), text)
        })
        .collect::<BTreeMap<_, _>>();
    if let Some(doc) = &mut behavior.pandora_doc {
        rename_in_place(doc, old, new);
    }
}

impl Workspace {
    /// Gives the work the code `new` instead of `old`. See the module docs. `done`: the objects done.
    pub fn rename_code(&mut self, old: &str, new: &str, done: &AtomicUsize) {
        self.behaviors.par_iter_mut().for_each(|behavior| rename_behavior(behavior, old, new, done));
        if let Some(selected) = &mut self.session.selected {
            rename_in_place(selected, old, new);
        }
        for tab in &mut self.session.tabs {
            rename_in_place(&mut tab.id, old, new);
        }
        // The history names the objects as the work does.
        for change in self.history.revisions.iter_mut().flat_map(|revision| &mut revision.changes) {
            rename_in_place(&mut change.id, old, new);
            if let Some(text) = &mut change.text {
                rename_in_place(text, old, new);
            }
        }
        // The next export takes out what was exported under the old code.
        let infos = [("Nemesis_Engine", "info.ini"), ("Nemesis_EngineExt", "info.ini"), ("Pandora_Engine", "info.xml")];
        self.owned.extend(infos.map(|(engine, file)| format!("{engine}/mod/{old}/{file}")));
        // Its cache is written anew, under the new code.
        self.saved = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_ids_of_the_code_are_renamed() {
        let text = "#oldc$3 #OLDC$4 #xoldc$1 Idle~oldc$0.txt\noldc$0 #0106 oldc";
        assert_eq!(rename_ids(text, "oldc", "newc"), "#newc$3 #newc$4 #xoldc$1 Idle~newc$0.txt\nnewc$0 #0106 oldc");
        assert!(matches!(rename_ids("#0106 é", "oldc", "newc"), Cow::Borrowed(_)));
        assert_eq!(rename_ids("é#oldc$1é", "oldc", "newc"), "é#newc$1é");
    }

    #[test]
    fn a_work_gets_the_new_code() {
        let mut workspace = Workspace::default();
        let mut behavior =
            Behavior::open(&std::path::Path::new("meshes").join(format!("{}.bin", super::super::ADSF))).unwrap();
        behavior.added.insert("M~1/Idle~oldc$0.txt".to_owned(), "Idle\noldc$0\n1\n0\n0\n0\n".to_owned());
        workspace.behaviors.push(behavior);
        workspace.session.selected = Some("M~1/Idle~oldc$0.txt".to_owned());

        let done = AtomicUsize::new(0);
        assert_eq!(object_count(&workspace), 1);
        workspace.rename_code("oldc", "newc", &done);

        assert_eq!(done.load(Ordering::Relaxed), 1);
        let added = &workspace.behaviors[0].added;
        assert_eq!(added.get("M~1/Idle~newc$0.txt").map(String::as_str), Some("Idle\nnewc$0\n1\n0\n0\n0\n"));
        assert_eq!(workspace.session.selected.as_deref(), Some("M~1/Idle~newc$0.txt"));
        assert!(workspace.owned.contains("Nemesis_Engine/mod/oldc/info.ini"));
    }
}
