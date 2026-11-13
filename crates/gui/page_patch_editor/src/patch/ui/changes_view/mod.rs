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
//! `keys.rs` runs the keyboard on the rows. Each part returns what was asked; [`PatchTool::changes_view`]
//! does it, in one place. The history is [`crate::patch::workspace::history`]; the diffs open in the
//! editor's place ([`super::revision_view`]).

mod header;
mod keys;
mod list;

use egui_widgets::notify::Notification;
use gui_shared::i18n::{I18nKey, I18nMap};

use self::header::BranchAsk;
use crate::patch::PatchTool;

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

impl PatchTool {
    /// Draws the view, and does what it asked for.
    pub(super) fn changes_view(&mut self, ui: &mut egui::Ui, i18n: &I18nMap, notify: &mut Notification) {
        egui_widgets::side_bar_title(ui, i18n.t(I18nKey::SideChanges));

        let branch = header::branch_row(ui, i18n, &self.workspace.history, &mut self.scm.new_branch);
        self.scm.pending.refresh(&self.workspace, self.edits);
        let has_changes = !self.scm.pending.changes.is_empty();
        let commit = header::commit_box(ui, i18n, &mut self.scm.commit_message, has_changes);

        let keys = {
            let focused = self.explorer.tree_focus;
            let taken = if focused {
                keybinding::host::take_mapped(&mut self.key_commands, keybinding::host::tree_key)
            } else {
                Vec::new()
            };
            keys::run(&mut self.scm, &self.workspace, &taken, focused)
        };

        let output = {
            let input = list::ListInput {
                workspace: &self.workspace,
                scm: &self.scm,
                selected: (self.active, self.selected.as_deref()),
                cursor: (keys.cursor, keys.moved),
            };
            list::show(ui, i18n, &input)
        };

        if let Some(ask) = branch {
            self.apply_branch(ask, notify, i18n);
        }
        if commit {
            self.commit();
        }
        if keys.leave {
            self.leave_tree();
        }
        if let Some(number) = output.toggled
            && !self.scm.open_revisions.remove(&number)
        {
            self.scm.open_revisions.insert(number);
        }
        if let Some(action) = output.picked.or(keys.picked) {
            self.apply_change(action);
        }
    }

    /// Another branch puts the work on it, unless that would lose its changes; a new one starts
    /// where the work is.
    fn apply_branch(&mut self, ask: BranchAsk, notify: &mut Notification, i18n: &I18nMap) {
        match ask {
            BranchAsk::Switch(name) => {
                if self.workspace.switch_branch(&name) {
                    self.work_restored();
                } else {
                    notify.warn(i18n.t(I18nKey::SideBranchHasChanges));
                }
            }
            BranchAsk::Create(name) => {
                if self.workspace.history.create_branch(&name) {
                    self.mark_dirty();
                }
            }
        }
    }

    /// A revision of the changes, with the message typed.
    fn commit(&mut self) {
        let time = crate::patch::workspace::history::now();
        let message = std::mem::take(&mut self.scm.commit_message);
        let mut history = std::mem::take(&mut self.workspace.history);
        history.commit(&self.workspace, &message, time);
        self.workspace.history = history;
        self.mark_dirty();
    }

    /// What a row asked for.
    fn apply_change(&mut self, action: ChangeAction) {
        match action {
            ChangeAction::Open(behavior, id) => {
                self.views.list.active = None; // the object itself, not a diff of it
                if behavior != self.active {
                    self.active = behavior;
                    self.tabs.clear();
                }
                self.selected = Some(id.into());
            }
            ChangeAction::Compare(behavior, id, from, to) => self.open_compare(behavior, &id, from, to),
            ChangeAction::Blame(behavior, id) => self.open_blame(behavior, &id),

            ChangeAction::Discard(behavior, id) => {
                self.workspace.discard_change(behavior, &id);
                self.work_restored();
            }
            ChangeAction::RestoreObject(behavior, id, number) => {
                self.workspace.restore_object(behavior, &id, number);
                self.work_restored();
            }
            ChangeAction::RestoreRevision(number) => {
                self.workspace.restore_revision(number);
                self.work_restored();
            }
        }
    }

    /// The objects changed under the editor: what shows them is made again.
    fn work_restored(&mut self) {
        self.reset_names();
        self.pane.opened = None;
        self.pane.preview = None;
        self.mark_dirty();
    }
}
