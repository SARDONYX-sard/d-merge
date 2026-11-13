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
    /// The change the keyboard is on (`j` / `k`), while the side bar has it.
    pub(super) cursor: usize,
}

/// The changes since the last revision, and what they were found for.
#[derive(Debug, Default)]
pub(super) struct Pending {
    /// `(edits counted, revisions, behaviors)` when they were found.
    found_for: Option<(u64, usize, usize)>,
    pub(super) changes: Vec<ObjectChange>,
}

impl Pending {
    /// Finds the changes again if the work was edited (`edits`: see `PatchTool::edits`), committed, or
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
