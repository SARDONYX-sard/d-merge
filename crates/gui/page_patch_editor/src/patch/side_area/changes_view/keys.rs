//! The keys of the Changes view, as in a tree ([`egui_tree::navigate`]): the changes, then the
//! revisions, each open one with its changes under it (see [`ScmRow`]). What the cursor gets to
//! opens, as a click does.

use egui_tree::{Nav, TreeKey};

use super::ChangeAction;
use crate::patch::{
    side_area::changes::{Scm, ScmRow, scm_rows},
    workspace::Workspace,
};

/// What the keys did.
#[derive(Debug, Default)]
pub(super) struct ScmKeys {
    /// The row the keyboard is on (`None`: the side bar does not have it).
    pub(super) cursor: Option<ScmRow>,
    /// It moved this frame (scrolled to).
    pub(super) moved: bool,
    /// What the row moved to opens.
    pub(super) picked: Option<ChangeAction>,
    /// `Enter` / `Esc`: back to the editor.
    pub(super) leave: bool,
}

/// Runs `keys` on the view's rows. `focused`: the side bar has the keyboard.
pub(super) fn run(scm: &mut Scm, workspace: &Workspace, keys: &[TreeKey], focused: bool) -> ScmKeys {
    let log: Vec<(u32, usize)> =
        workspace.history.log().iter().map(|revision| (revision.number, revision.changes.len())).collect();
    let mut rows = scm_rows(scm.pending.changes.len(), &log, &scm.open_revisions);
    scm.cursor = scm.cursor.min(rows.len().saturating_sub(1));
    if !focused {
        return ScmKeys::default();
    }

    let mut done = ScmKeys::default();
    for &key in keys {
        let nav: Vec<_> = rows.iter().map(|row| row.nav(&scm.open_revisions)).collect();
        match egui_tree::navigate(key, &nav, scm.cursor) {
            Nav::Cursor(at) => {
                done.moved |= at != scm.cursor;
                scm.cursor = at;
            }
            Nav::SetOpen(at, open) => {
                if let ScmRow::Revision(number) = rows[at] {
                    set_open(scm, number, open);
                }
                rows = scm_rows(scm.pending.changes.len(), &log, &scm.open_revisions);
            }
            Nav::Leave => done.leave = true,
            Nav::None => {}
        }
    }

    done.cursor = rows.get(scm.cursor).copied();
    if done.moved {
        done.picked = done.cursor.and_then(|row| opened_by(row, scm, workspace));
    }
    done
}

/// Revision `number`'s changes are shown (`open`) or not.
fn set_open(scm: &mut Scm, number: u32, open: bool) {
    if open {
        scm.open_revisions.insert(number);
    } else {
        scm.open_revisions.remove(&number);
    }
}

/// What getting to `row` opens: a change's object, or what a revision changed of one.
fn opened_by(row: ScmRow, scm: &Scm, workspace: &Workspace) -> Option<ChangeAction> {
    let behavior_of = |path: &str| workspace.behaviors.iter().position(|behavior| behavior.meshes_path == path);
    match row {
        ScmRow::Change(at) => {
            let change = scm.pending.changes.get(at)?;
            Some(ChangeAction::Open(behavior_of(&change.behavior)?, change.id.clone()))
        }
        ScmRow::RevisionChange(number, at) => {
            let history = &workspace.history;
            let change = history.revision(number)?.changes.get(at)?;
            let parent = history.parent_number(number);
            Some(ChangeAction::Compare(behavior_of(&change.behavior)?, change.id.clone(), parent, Some(number)))
        }
        ScmRow::Revision(_) => None,
    }
}
