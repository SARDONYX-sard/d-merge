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
//!
//! # Branches
//! Each revision has its parent, and a branch is a name for its newest revision, as git's:
//! ```text
//! main:   #1 -- #2 -- #4
//!                  \
//! dodge:            #3
//! ```
//! A commit goes on the branch the work is on. A history kept before branches had none: it is one
//! line, each revision's parent the one before ([`Revision::parent`] `None`), named `main`.
//!
//! # Blame
//! [`History::blame`]: which revision last changed each line of an object, as GitLens' blame: the
//! versions of the object along the branch are compared one after the other.

use std::collections::BTreeMap;

use super::{Behavior, Workspace};

/// The branch of a history kept before branches, and of a new one.
pub(crate) const MAIN: &str = "main";

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
    /// The revision it was made on: `Some(0)` for the first one of the work. `None`: kept before there
    /// were branches, where it is the one before (`number - 1`).
    #[serde(default)]
    pub(crate) parent: Option<u32>,
    /// Seconds since the Unix epoch.
    pub(crate) time: u64,
    pub(crate) message: String,
    pub(crate) changes: Vec<ObjectChange>,
}

/// See the module docs.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub(crate) struct History {
    /// Every revision of every branch, by number.
    pub(crate) revisions: Vec<Revision>,
    /// Branch name -> its newest revision (`0`: none yet). Empty: only `main`, at the last revision.
    pub(crate) branches: BTreeMap<String, u32>,
    /// The branch the work is on. Empty: [`MAIN`].
    pub(crate) branch: String,
}

/// Where a line of an object comes from. See [`History::blame`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Origin {
    /// As the template has it.
    Template,
    /// Last changed by this revision.
    Revision(u32),
    /// Changed since the last revision.
    Working,
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
    /// The branch the work is on.
    pub(crate) fn branch_name(&self) -> &str {
        if self.branch.is_empty() { MAIN } else { &self.branch }
    }

    /// The branches, by name. `main` also before there were branches.
    pub(crate) fn branch_names(&self) -> Vec<&str> {
        if self.branches.is_empty() { vec![MAIN] } else { self.branches.keys().map(String::as_str).collect() }
    }

    /// The newest revision of the branch the work is on (`0`: none).
    pub(crate) fn head_number(&self) -> u32 {
        match self.branches.get(self.branch_name()) {
            Some(number) => *number,
            None => self.revisions.last().map_or(0, |last| last.number),
        }
    }

    pub(crate) fn revision(&self, number: u32) -> Option<&Revision> {
        self.revisions.iter().find(|revision| revision.number == number)
    }

    /// The parent of revision `number` (`0`: it is the first of its line, made on the template).
    pub(crate) fn parent_number(&self, number: u32) -> u32 {
        self.revision(number).and_then(Self::parent_of).unwrap_or(0)
    }

    /// The parent of `revision` (`None`: it is the first of its line).
    fn parent_of(revision: &Revision) -> Option<u32> {
        Some(revision.parent.unwrap_or_else(|| revision.number.saturating_sub(1))).filter(|parent| *parent > 0)
    }

    /// Revision `number` and its parents, newest first: git's log from it.
    pub(crate) fn log_from(&self, number: u32) -> Vec<&Revision> {
        let mut log = Vec::new();
        let mut at = Some(number).filter(|number| *number > 0);
        while let Some(revision) = at.and_then(|number| self.revision(number)) {
            log.push(revision);
            at = Self::parent_of(revision);
        }
        log
    }

    /// The revisions of the branch the work is on, newest first.
    pub(crate) fn log(&self) -> Vec<&Revision> {
        self.log_from(self.head_number())
    }

    /// What the work was at the last revision of its branch. Empty without one.
    pub(crate) fn head(&self) -> BTreeMap<ObjectKey, &str> {
        self.state_at(self.head_number())
    }

    /// What the work was at revision `number`: it and its parents replayed. `0`: before the first, the
    /// template as it is.
    pub(crate) fn state_at(&self, number: u32) -> BTreeMap<ObjectKey, &str> {
        let mut state = BTreeMap::new();
        let log = self.log_from(number);
        for change in log.iter().rev().flat_map(|revision| &revision.changes) {
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
        let parent = self.head_number();
        let number = self.revisions.iter().map(|revision| revision.number).max().unwrap_or(0) + 1;
        let message = message.trim().to_owned();
        self.revisions.push(Revision { number, parent: Some(parent), time, message, changes });
        let branch = self.branch_name().to_owned();
        self.branches.insert(branch, number);
        Some(number)
    }

    /// The revisions of the branch that changed `key`, newest first: GitLens' "File History".
    pub(crate) fn of_object<'a>(&'a self, key: &'a ObjectKey) -> Vec<(&'a Revision, &'a ObjectChange)> {
        let log = self.log();
        log.into_iter()
            .filter_map(|revision| {
                let change = revision.changes.iter().find(|change| change.behavior == key.0 && change.id == key.1)?;
                Some((revision, change))
            })
            .collect()
    }

    /// Starts branch `name` at the revision the work is on, and puts the work on it. `false`: there is
    /// one of that name already.
    pub(crate) fn create_branch(&mut self, name: &str) -> bool {
        let name = name.trim();
        if name.is_empty() || self.branch_names().contains(&name) {
            return false;
        }
        let head = self.head_number();
        if self.branches.is_empty() {
            self.branches.insert(MAIN.to_owned(), head);
        }
        self.branches.insert(name.to_owned(), head);
        name.clone_into(&mut self.branch);
        true
    }

    /// Which revision last changed each line of `now`, the text of object `key` in the work (`template`:
    /// the template's, if it has the object). See the module docs.
    ///
    /// e.g. template `a`, `b`; #1 makes `b` `B`; the work adds `c` -> `Template`, `Revision(1)`, `Working`
    pub(crate) fn blame(&self, key: &ObjectKey, template: Option<&str>, now: &str) -> Vec<Origin> {
        let mut text = template.unwrap_or_default().to_owned();
        let mut origins = vec![Origin::Template; text.lines().count()];
        let mut versions: Vec<(Origin, String)> = self
            .of_object(key)
            .into_iter()
            .rev()
            .map(|(revision, change)| (Origin::Revision(revision.number), change.text.clone().unwrap_or_default()))
            .collect();
        versions.push((Origin::Working, now.to_owned()));
        for (origin, next) in versions {
            origins = carry_origins(&text, &next, &origins, origin);
            text = next;
        }
        origins
    }
}

/// The origins of the lines of `new`: those `old` has as they were (`old_origins`), the others `origin`.
fn carry_origins(old: &str, new: &str, old_origins: &[Origin], origin: Origin) -> Vec<Origin> {
    let mut origins = Vec::with_capacity(new.lines().count());
    let mut at_old = 0;
    for hunk in nemesis_diff::line_changes(old, new) {
        let same = hunk.old.start.saturating_sub(at_old);
        origins.extend((0..same).map(|i| old_origins.get(at_old + i).copied().unwrap_or(origin)));
        origins.extend(hunk.new.clone().map(|_| origin));
        at_old = hunk.old.end;
    }
    let rest = new.lines().count().saturating_sub(origins.len());
    origins.extend((0..rest).map(|i| old_origins.get(at_old + i).copied().unwrap_or(origin)));
    origins
}

/// Sets object `id` of `behavior` to `text`: an edit of the template, a new object, or (`None`) gone.
fn set_object(behavior: &mut Behavior, id: &str, text: Option<String>) {
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
        let head = self.history.head_number();
        self.restore_object(behavior, id, head);
    }

    /// Puts the work on branch `name`, as it is at its newest revision: git's `switch`. `false`: the work
    /// has changes since its last revision, which would be lost (commit or discard them first).
    pub(crate) fn switch_branch(&mut self, name: &str) -> bool {
        if !self.history.pending(self).is_empty() {
            return false;
        }
        if self.history.branches.is_empty() {
            let head = self.history.head_number();
            self.history.branches.insert(MAIN.to_owned(), head);
        }
        let Some(head) = self.history.branches.get(name).copied() else { return false };
        name.clone_into(&mut self.history.branch);
        self.restore_revision(head);
        true
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
        let numbers: Vec<u32> =
            second.history.of_object(&key).into_iter().map(|(revision, _)| revision.number).collect();
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

    /// `main: #1 -- #2`, `dodge: #1 -- #3`: each branch has its own line, and the work goes with it.
    #[test]
    fn branches_have_their_own_line() {
        let mut workspace = work(&[("a", "1")]);
        let mut history = std::mem::take(&mut workspace.history);
        history.commit(&workspace, "first", 10);
        workspace.history = history;
        assert!(workspace.history.create_branch("dodge"));
        assert!(!workspace.history.create_branch("main"), "there is one");

        workspace.behaviors[0].added.insert("b".to_owned(), "1".to_owned());
        let mut history = std::mem::take(&mut workspace.history);
        assert_eq!(history.commit(&workspace, "on dodge", 11), Some(2));
        workspace.history = history;

        assert!(workspace.switch_branch("main"));
        assert!(!workspace.behaviors[0].added.contains_key("b"), "main has no b");
        workspace.behaviors[0].added.insert("a".to_owned(), "2".to_owned());
        assert!(!workspace.switch_branch("dodge"), "a change would be lost");
        let mut history = std::mem::take(&mut workspace.history);
        assert_eq!(history.commit(&workspace, "on main", 12), Some(3));
        workspace.history = history;

        let log = |history: &History| history.log().iter().map(|revision| revision.number).collect::<Vec<_>>();
        assert_eq!(log(&workspace.history), [3, 1]);
        assert!(workspace.switch_branch("dodge"));
        assert_eq!(log(&workspace.history), [2, 1]);
        assert_eq!(workspace.behaviors[0].added.get("a").map(String::as_str), Some("1"));
    }

    /// A history kept before branches: one line, `main`.
    #[test]
    fn an_old_history_is_one_line() {
        let old = |number| Revision { number, parent: None, time: 0, message: String::new(), changes: Vec::new() };
        let history = History { revisions: vec![old(1), old(2)], ..History::default() };
        assert_eq!(history.branch_names(), [MAIN]);
        assert_eq!(history.log().iter().map(|revision| revision.number).collect::<Vec<_>>(), [2, 1]);
    }

    #[test]
    fn blame_tells_the_revision_of_each_line() {
        let mut workspace = work(&[("a", "x\nB\n")]);
        let key = (workspace.behaviors[0].meshes_path.clone(), "a".to_owned());
        let mut history = std::mem::take(&mut workspace.history);
        history.commit(&workspace, "b", 10);
        let origins = history.blame(&key, Some("x\nb\n"), "x\nB\nc\n");
        assert_eq!(origins, [Origin::Template, Origin::Revision(1), Origin::Working]);
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
