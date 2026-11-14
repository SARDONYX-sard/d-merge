//! The side bar's Changes view, as VS Code's Source Control, one part per file:
//!
//! ```text
//! CHANGES
//! ⎇ [main ▼] [+]                         header.rs   the branch
//! [message...................] [Commit]  header.rs   the commit box
//! ▼ CHANGES (2)                          list.rs     the changes, the object's history, the history
//! ▼ OBJECT HISTORY  #0106
//! ▼ HISTORY
//! ```
//! `keys.rs` runs the keyboard on the rows. Each part returns what was asked; [`SideArea::changes_view`]
//! does it, in one place. The history is [`crate::patch::workspace::history`]; the diffs open in the
//! editor's place ([`super::revision_view`]).

mod header;
mod keys;
mod list;

use egui_widgets::notify::Notification;
use gui_shared::i18n::{I18nKey, I18nMap};

use self::header::BranchAsk;
use crate::patch::{
    side_area::{SideArea, SideAsk},
    tool::{keys::Keys, selection::Selection, work_state::Work},
};

/// What a row of the Changes view asked for: `(behavior, id)`.
#[derive(Debug)]
enum ChangeAction {
    Open(usize, String),
    /// Back to the last revision: as git's "Discard changes".
    Discard(usize, String),
    /// The object as it was at revision `.2`: as git's `restore --source <rev> <file>`.
    RestoreObject(usize, String, u32),
    /// The whole work as it was at that revision: as git's `restore --source <rev> .`.
    RestoreRevision(u32),
    /// The object from revision `.2` to `.3` (`None`: the work now): as git's `diff`.
    Compare(usize, String, u32, Option<u32>),
    /// Each line of the object with the revision that last changed it.
    Blame(usize, String),
}

impl SideArea {
    /// Draws the view, and does what it asked for of the work. Returns what it asked of the rest.
    pub(super) fn changes_view(
        &mut self,
        ui: &mut egui::Ui,
        i18n: &I18nMap,
        (work, sel, keys): (&mut Work, &Selection, &mut Keys),
        notify: &mut Notification,
    ) -> Vec<SideAsk> {
        egui_widgets::side_bar_title(ui, i18n.t(I18nKey::SideChanges));

        let branch = header::branch_row(ui, i18n, &work.workspace.history, &mut self.scm.new_branch);
        self.scm.pending.refresh(&work.workspace, work.edits());
        let has_changes = !self.scm.pending.changes.is_empty();
        let commit = header::commit_box(ui, i18n, &mut self.scm.commit_message, has_changes);

        let keys = {
            let focused = self.explorer.tree_focus;
            let taken = if focused { keys.take_mapped(keybinding::host::tree_key) } else { Vec::new() };
            keys::run(&mut self.scm, &work.workspace, &taken, focused)
        };

        let output = {
            let input = list::ListInput {
                workspace: &work.workspace,
                scm: &self.scm,
                selected: (sel.active, sel.selected.as_deref()),
                cursor: (keys.cursor, keys.moved),
            };
            list::show(ui, i18n, &input)
        };

        let mut asks = Vec::new();
        if let Some(ask) = branch {
            asks.extend(apply_branch(work, ask, notify, i18n));
        }
        if commit {
            commit_changes(work, std::mem::take(&mut self.scm.commit_message));
        }
        if keys.leave {
            self.explorer.tree_focus = false;
            asks.push(SideAsk::FocusEditor);
        }
        if let Some(number) = output.toggled
            && !self.scm.open_revisions.remove(&number)
        {
            self.scm.open_revisions.insert(number);
        }
        if let Some(action) = output.picked.or(keys.picked) {
            asks.push(apply_change(work, action));
        }
        asks
    }
}

/// Another branch puts the work on it, unless that would lose its changes; a new one starts where
/// the work is.
fn apply_branch(work: &mut Work, ask: BranchAsk, notify: &mut Notification, i18n: &I18nMap) -> Option<SideAsk> {
    match ask {
        BranchAsk::Switch(name) => {
            if work.workspace.switch_branch(&name) {
                return Some(work_restored(work));
            }
            notify.warn(i18n.t(I18nKey::SideBranchHasChanges));
        }
        BranchAsk::Create(name) => {
            if work.workspace.history.create_branch(&name) {
                work.mark_dirty();
            }
        }
    }
    None
}

/// A revision of the changes, with `message`.
fn commit_changes(work: &mut Work, message: String) {
    let time = crate::patch::workspace::history::now();
    let mut history = std::mem::take(&mut work.workspace.history);
    history.commit(&work.workspace, &message, time);
    work.workspace.history = history;
    work.mark_dirty();
}

/// What a row asked for: what changes the work is done, what is shown is asked for.
fn apply_change(work: &mut Work, action: ChangeAction) -> SideAsk {
    match action {
        ChangeAction::Open(behavior, id) => SideAsk::OpenChange(behavior, id),
        ChangeAction::Compare(behavior, id, from, to) => SideAsk::Compare(behavior, id, from, to),
        ChangeAction::Blame(behavior, id) => SideAsk::Blame(behavior, id),

        ChangeAction::Discard(behavior, id) => {
            work.workspace.discard_change(behavior, &id);
            work_restored(work)
        }
        ChangeAction::RestoreObject(behavior, id, number) => {
            work.workspace.restore_object(behavior, &id, number);
            work_restored(work)
        }
        ChangeAction::RestoreRevision(number) => {
            work.workspace.restore_revision(number);
            work_restored(work)
        }
    }
}

/// The objects changed under the editor: what shows them is made again.
fn work_restored(work: &mut Work) -> SideAsk {
    work.reset_names();
    work.mark_dirty();
    SideAsk::Reload
}
