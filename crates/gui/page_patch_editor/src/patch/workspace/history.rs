//! A small revision history of a work, like git's: which change came when.
//!
//! ```text
//! #1  add dodge      2026-11-12 18:00   A 0_master #m$0, M 0_master #0106
//! #2  fix the speed  2026-11-12 19:30   M 0_master #m$0
//! (working)                             M 0_master #0106   <- what the Changes view lists
//! ```
//! - A revision keeps only what changed since the one before it: each object's whole text, or that
//!   it went (`Removed`). What the work was at a revision is all of them replayed ([`History::head`]).
//! - The objects are named by their behavior's template (`meshes_path`) and id, as the work keeps them.
//! - It is in the work's cache (`history.json`), not in the mod: Nemesis knows nothing of it.

use std::collections::BTreeMap;

use super::Workspace;

/// `(behavior's meshes_path, object id)`, e.g. `("meshes/actors/character/behaviors/0_master.bin", "#0106")`.
pub(crate) type ObjectKey = (String, String);

/// What happened to an object in a revision, as git's status letters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) enum ChangeKind {
    /// `A`: not in the revision before.
    Added,
    /// `M`: another text.
    Modified,
    /// `D`: gone, back to the template or deleted.
    Removed,
}

impl ChangeKind {
    pub(crate) const fn mark(self) -> &'static str {
        match self {
            Self::Added => "A",
            Self::Modified => "M",
            Self::Removed => "D",
        }
    }
}

/// One object of a revision.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct ObjectChange {
    /// e.g. `meshes/actors/character/behaviors/0_master.bin`
    pub(crate) behavior: String,
    /// e.g. `#0106`
    pub(crate) id: String,
    pub(crate) kind: ChangeKind,
    /// Its text from then on. `None` for `Removed`.
    pub(crate) text: Option<String>,
}

/// One revision. See the module docs.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct Revision {
    /// 1, 2, ...
    pub(crate) number: u32,
    /// Seconds since the Unix epoch.
    pub(crate) time: u64,
    pub(crate) message: String,
    pub(crate) changes: Vec<ObjectChange>,
}

/// See the module docs.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub(crate) struct History {
    pub(crate) revisions: Vec<Revision>,
}

/// The changed objects of the work now, by key: what a revision compares to.
fn current(workspace: &Workspace) -> BTreeMap<ObjectKey, &str> {
    let mut found = BTreeMap::new();
    for behavior in &workspace.behaviors {
        for (id, text) in behavior.edits.iter().chain(&behavior.added) {
            found.insert((behavior.meshes_path.clone(), id.clone()), text.as_str());
        }
    }
    found
}

impl History {
    /// What the work was at the last revision: every revision replayed. Empty without one.
    pub(crate) fn head(&self) -> BTreeMap<ObjectKey, &str> {
        self.state_at(u32::MAX)
    }

    /// What the work was at revision `number`: the revisions up to it replayed. `0`: before the first,
    /// the template as it is.
    pub(crate) fn state_at(&self, number: u32) -> BTreeMap<ObjectKey, &str> {
        let mut state = BTreeMap::new();
        let upto = self.revisions.iter().take_while(|revision| revision.number <= number);
        for change in upto.flat_map(|revision| &revision.changes) {
            let key = (change.behavior.clone(), change.id.clone());
            match &change.text {
                Some(text) => state.insert(key, text.as_str()),
                None => state.remove(&key),
            };
        }
        state
    }

    /// What the work changes since the last revision, by behavior and id. The `text`s are not kept:
    /// they are the work's.
    pub(crate) fn pending(&self, workspace: &Workspace) -> Vec<ObjectChange> {
        let (head, now) = (self.head(), current(workspace));
        let mut changes = Vec::new();
        for (key, text) in &now {
            let kind = match head.get(key) {
                None => ChangeKind::Added,
                Some(before) if before != text => ChangeKind::Modified,
                Some(_) => continue,
            };
            changes.push(ObjectChange { behavior: key.0.clone(), id: key.1.clone(), kind, text: None });
        }
        for key in head.keys().filter(|key| !now.contains_key(*key)) {
            changes.push(ObjectChange {
                behavior: key.0.clone(),
                id: key.1.clone(),
                kind: ChangeKind::Removed,
                text: None,
            });
        }
        changes.sort_by(|a, b| (&a.behavior, &a.id).cmp(&(&b.behavior, &b.id)));
        changes
    }

    /// Makes a revision of what changed since the last one. `None`: nothing changed.
    pub(crate) fn commit(&mut self, workspace: &Workspace, message: &str, time: u64) -> Option<u32> {
        let now = current(workspace);
        let mut changes = self.pending(workspace);
        if changes.is_empty() {
            return None;
        }
        for change in &mut changes {
            let key = (change.behavior.clone(), change.id.clone());
            change.text = now.get(&key).map(|text| (*text).to_owned());
        }
        let number = self.revisions.last().map_or(1, |last| last.number + 1);
        self.revisions.push(Revision { number, time, message: message.trim().to_owned(), changes });
        Some(number)
    }

    /// The revisions that changed `key`, newest first: GitLens' "File History" of an object.
    pub(crate) fn of_object<'a>(
        &'a self,
        key: &'a ObjectKey,
    ) -> impl Iterator<Item = (&'a Revision, &'a ObjectChange)> {
        self.revisions.iter().rev().filter_map(move |revision| {
            let change = revision.changes.iter().find(|change| change.behavior == key.0 && change.id == key.1)?;
            Some((revision, change))
        })
    }
}

/// Sets object `id` of `behavior` to `text`: an edit of the template, a new object, or (`None`) gone.
fn set_object(behavior: &mut super::Behavior, id: &str, text: Option<String>) {
    match text {
        Some(text) if behavior.template_object(id).is_some() => behavior.set_text(id, &text),
        Some(text) => {
            behavior.added.insert(id.to_owned(), text);
        }
        None => {
            behavior.edits.remove(id);
            behavior.added.remove(id);
        }
    }
}

impl Workspace {
    /// Puts object `id` of the behavior at `behavior` back to what it was at the last revision: as
    /// git's "discard changes". Gone if it was not there.
    pub(crate) fn discard_change(&mut self, behavior: usize, id: &str) {
        self.restore_object(behavior, id, u32::MAX);
    }

    /// Puts object `id` of the behavior at `behavior` as it was at revision `number`: as git's
    /// `restore --source <rev> <file>`. The work's history stays: it is a change to commit.
    pub(crate) fn restore_object(&mut self, behavior: usize, id: &str, number: u32) {
        let Some(of) = self.behaviors.get(behavior) else { return };
        let key = (of.meshes_path.clone(), id.to_owned());
        let text = self.history.state_at(number).get(&key).map(|text| (*text).to_owned());
        set_object(&mut self.behaviors[behavior], id, text);
    }

    /// Puts the whole work as it was at revision `number`: as git's `restore --source <rev> .`. Its
    /// history stays, and what differs is a change to commit. Returns the objects of behaviors the
    /// work no longer has, which can not be put back.
    pub(crate) fn restore_revision(&mut self, number: u32) -> usize {
        let then: BTreeMap<ObjectKey, String> =
            self.history.state_at(number).into_iter().map(|(key, text)| (key, text.to_owned())).collect();
        let mut missing = 0;
        for of in &mut self.behaviors {
            let now: Vec<String> = of.edits.keys().chain(of.added.keys()).cloned().collect();
            for id in now {
                let key = (of.meshes_path.clone(), id);
                if !then.contains_key(&key) {
                    set_object(of, &key.1, None);
                }
            }
        }
        for ((behavior, id), text) in then {
            match self.behaviors.iter_mut().find(|of| of.meshes_path == behavior) {
                Some(of) => set_object(of, &id, Some(text)),
                None => missing += 1,
            }
        }
        missing
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::patch::workspace::{ADSF, Behavior};

    fn work(files: &[(&str, &str)]) -> Workspace {
        let mut workspace = Workspace::default();
        let mut behavior = Behavior::open(&std::path::Path::new("meshes").join(format!("{ADSF}.bin"))).unwrap();
        for (id, text) in files {
            behavior.added.insert((*id).to_owned(), (*text).to_owned());
        }
        workspace.behaviors.push(behavior);
        workspace
    }

    fn marks(changes: &[ObjectChange]) -> Vec<(&str, &str)> {
        changes.iter().map(|change| (change.id.as_str(), change.kind.mark())).collect()
    }

    #[test]
    fn a_revision_keeps_what_changed_since_the_last_one() {
        let mut history = History::default();
        let first = work(&[("a", "1"), ("b", "1")]);
        assert_eq!(marks(&history.pending(&first)), [("a", "A"), ("b", "A")]);
        assert_eq!(history.commit(&first, " first ", 10), Some(1));
        assert!(history.pending(&first).is_empty());
        assert_eq!(history.commit(&first, "again", 11), None, "nothing changed");

        let second = work(&[("a", "2"), ("c", "1")]);
        assert_eq!(marks(&history.pending(&second)), [("a", "M"), ("b", "D"), ("c", "A")]);
        assert_eq!(history.commit(&second, "second", 12), Some(2));

        let revision = &history.revisions[1];
        assert_eq!((revision.message.as_str(), revision.changes.len()), ("second", 3));
        assert_eq!(history.revisions[0].message, "first");
        let head = history.head();
        assert_eq!(head.values().copied().collect::<Vec<_>>(), ["2", "1"], "a and c, b gone");
    }

    #[test]
    fn the_work_goes_back_to_a_revision() {
        let mut workspace = work(&[("a", "1")]);
        let mut history = History::default();
        history.commit(&workspace, "first", 10);
        let mut second = work(&[("a", "2"), ("b", "1")]);
        history.commit(&second, "second", 11);
        second.history = history;

        let key = (second.behaviors[0].meshes_path.clone(), "a".to_owned());
        let numbers: Vec<u32> = second.history.of_object(&key).map(|(revision, _)| revision.number).collect();
        assert_eq!(numbers, [2, 1], "newest first");

        second.restore_object(0, "a", 1);
        assert_eq!(second.behaviors[0].added.get("a").map(String::as_str), Some("1"));
        assert_eq!(second.restore_revision(1), 0);
        let ids: Vec<&String> = second.behaviors[0].added.keys().collect();
        assert_eq!(ids, ["a"], "b came later");
        assert_eq!(second.history.revisions.len(), 2, "the history stays");

        workspace.history = second.history.clone();
        workspace.restore_revision(0);
        assert!(workspace.behaviors[0].added.is_empty(), "before the first: the template");
    }

    #[test]
    fn a_change_is_discarded_to_the_last_revision() {
        let mut history = History::default();
        let mut workspace = work(&[("a", "1")]);
        history.commit(&workspace, "first", 10);
        workspace.history = history;
        workspace.behaviors[0].added.insert("a".to_owned(), "2".to_owned());
        workspace.behaviors[0].added.insert("new".to_owned(), "x".to_owned());

        workspace.discard_change(0, "a");
        workspace.discard_change(0, "new");
        let added = &workspace.behaviors[0].added;
        assert_eq!(added.get("a").map(String::as_str), Some("1"));
        assert!(!added.contains_key("new"));
    }
}
