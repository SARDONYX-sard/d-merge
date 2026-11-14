//! What the work changes since its last revision, for the side bar's Changes view: like VS Code's
//! Source Control, the objects marked `A` / `M` / `D` under their behavior. Drawn by
//! [`super::ui::side_bar`]; the revisions are [`super::workspace::history`].
//!
//! Comparing the work with the last revision goes through every changed object, so it is done again
//! only after an edit or a commit, not each frame: see [`Pending::refresh`].

use super::workspace::{Workspace, history::ObjectChange};

/// The state of the Changes view, as VS Code's Source Control.
#[derive(Debug, Default)]
pub(super) struct Scm {
    pub(super) pending: Pending,
    /// The message of the next revision, as it is typed.
    pub(super) commit_message: String,
    /// The name of a new branch, while it is typed.
    pub(super) new_branch: Option<String>,
    /// The row the keyboard is on (`j` / `k`), while the side bar has it: see [`ScmRow`].
    pub(super) cursor: usize,
    /// The revisions of the history open, to see what they changed.
    pub(super) open_revisions: std::collections::BTreeSet<u32>,
}

/// A row of the Changes view the keyboard goes through: the changes, then the history.
///
/// ```text
/// M 0_master #0106 ...        Change(0)
/// #3 fix the speed  (2)       Revision(3)
///   M 0_master #0106 ...      RevisionChange(3, 0)   <- while #3 is open
/// #2 add dodge  (1)           Revision(2)
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ScmRow {
    Change(usize),
    Revision(u32),
    RevisionChange(u32, usize),
}

/// The rows: `changes` of the work, then each revision of `history` (`(number, changes)`, newest
/// first), with its changes if it is in `open`.
pub(super) fn scm_rows(
    changes: usize,
    history: &[(u32, usize)],
    open: &std::collections::BTreeSet<u32>,
) -> Vec<ScmRow> {
    let mut rows: Vec<ScmRow> = (0..changes).map(ScmRow::Change).collect();
    for &(number, count) in history {
        rows.push(ScmRow::Revision(number));
        if open.contains(&number) {
            rows.extend((0..count).map(|at| ScmRow::RevisionChange(number, at)));
        }
    }
    rows
}

impl ScmRow {
    /// What the keys need of it: a revision opens (if it is in `open`), its changes are in it.
    pub(super) fn nav(self, open: &std::collections::BTreeSet<u32>) -> egui_tree::NavRow {
        let (depth, open) = match self {
            Self::Change(_) => (0, None),
            Self::Revision(number) => (0, Some(open.contains(&number))),
            Self::RevisionChange(..) => (1, None),
        };
        egui_tree::NavRow { depth, open }
    }
}

/// The changes since the last revision, and what they were found for.
#[derive(Debug, Default)]
pub(super) struct Pending {
    /// `(edits counted, revisions, behaviors)` when they were found.
    found_for: Option<(u64, usize, usize)>,
    pub(super) changes: Vec<ObjectChange>,
}

impl Pending {
    /// Finds the changes again if the work was edited (`edits`: see `Work::edits`), committed, or
    /// got or lost a behavior since.
    pub(super) fn refresh(&mut self, workspace: &Workspace, edits: u64) {
        let now = (edits, workspace.history.revisions.len(), workspace.behaviors.len());
        if self.found_for != Some(now) {
            self.changes = workspace.history.pending(workspace);
            self.found_for = Some(now);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The changes, then the history, with the changes of a revision open under it.
    #[test]
    fn the_rows_are_the_changes_then_the_history() {
        let open = std::collections::BTreeSet::from([3]);
        let rows = scm_rows(1, &[(3, 2), (2, 1)], &open);
        let expected = [
            ScmRow::Change(0),
            ScmRow::Revision(3),
            ScmRow::RevisionChange(3, 0),
            ScmRow::RevisionChange(3, 1),
            ScmRow::Revision(2),
        ];
        assert_eq!(rows, expected);
    }

    use crate::patch::workspace::{ADSF, Behavior};

    #[test]
    fn the_changes_are_found_again_only_after_an_edit() {
        let mut workspace = Workspace::default();
        let mut behavior = Behavior::open(&std::path::Path::new("meshes").join(format!("{ADSF}.bin"))).unwrap();
        behavior.added.insert("M~1/a.txt".to_owned(), "1".to_owned());
        workspace.behaviors.push(behavior);
        let mut pending = Pending::default();
        pending.refresh(&workspace, 0);
        assert_eq!(pending.changes.len(), 1);

        workspace.behaviors[0].added.insert("M~1/b.txt".to_owned(), "1".to_owned());
        pending.refresh(&workspace, 0);
        assert_eq!(pending.changes.len(), 1, "no edit counted: as it was");
        pending.refresh(&workspace, 1);
        assert_eq!(pending.changes.len(), 2);
    }
}
