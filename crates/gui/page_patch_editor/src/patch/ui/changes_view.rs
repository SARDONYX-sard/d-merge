//! The side bar's Changes view, as VS Code's Source Control: the branch, the message and `Commit`,
//! the changes since the last revision, and the history of the work.
//!
//! ```text
//! CHANGES
//! [main ▼] [+]
//! [message...................] [Commit]
//! M 0_master #0106 GetUpModList          <- a click opens it, the keys go through the rows
//! ▼ #3 fix the speed  (2)
//!     M 0_master #0106 ...               <- its diff opens
//! ```
//! The history is [`crate::patch::workspace::history`]; the diffs open in the editor's place
//! ([`super::revision_view`]).

use egui_widgets::shadcn::{Button, ComboBox, Icon};
use gui_shared::i18n::{I18nKey, I18nMap};

use crate::patch::{
    PatchTool,
    changes::ScmRow,
    workspace::history::{ChangeKind, ObjectChange},
};

impl PatchTool {
    /// `CHANGES`, the message and `Commit`, the changes since the last revision, and the history:
    /// like VS Code's Source Control. See [`crate::patch::workspace::history`].
    pub(super) fn changes_view(
        &mut self,
        ui: &mut egui::Ui,
        i18n: &I18nMap,
        notify: &mut egui_widgets::notify::Notification,
    ) {
        egui_widgets::side_bar_title(ui, i18n.t(I18nKey::SideChanges));
        self.branch_row(ui, i18n, notify);
        self.scm.pending.refresh(&self.workspace, self.edits);
        self.commit_box(ui, i18n);

        let mut picked = None;
        // The keys of the explorer go through the changes and the history while the side bar has the
        // keyboard (see `ScmRow`): what the cursor gets to opens, as a click does.
        let (cursor, moved) = self.scm_keys(&mut picked);
        let mut toggled = None;
        // Up and down only: the rows are cut with `…`, as VS Code's.
        egui::ScrollArea::vertical().id_salt("side_changes").auto_shrink(false).show(ui, |ui| {
            let title = format!("{} ({})", i18n.t(I18nKey::SideChanges), self.scm.pending.changes.len());
            egui::CollapsingHeader::new(title).id_salt("side_pending").default_open(true).show(ui, |ui| {
                if self.scm.pending.changes.is_empty() {
                    ui.weak(i18n.t(I18nKey::SideNoChanges));
                }
                for (at, change) in self.scm.pending.changes.iter().enumerate() {
                    let keyboard = (cursor == Some(ScmRow::Change(at)), moved);
                    if let Some(action) = change_row(ui, i18n, (&self.workspace, change), None, keyboard) {
                        picked = Some(action);
                    }
                }
            });
            if let Some(action) = self.object_history(ui, i18n) {
                picked = Some(action);
            }
            egui::CollapsingHeader::new(i18n.t(I18nKey::SideHistory)).id_salt("side_history").default_open(true).show(
                ui,
                |ui| {
                    for revision in self.workspace.history.log() {
                        let title = format!(
                            "#{} {}  {}  ({})",
                            revision.number,
                            revision.message,
                            local_time(revision.time),
                            revision.changes.len()
                        );
                        let number = revision.number;
                        let shown = egui::CollapsingHeader::new(title)
                            .id_salt(("side_revision", number))
                            .open(Some(self.scm.open_revisions.contains(&number)))
                            .show(ui, |ui| {
                                for (at, change) in revision.changes.iter().enumerate() {
                                    let keyboard = (cursor == Some(ScmRow::RevisionChange(number, at)), moved);
                                    let row = change_row(ui, i18n, (&self.workspace, change), Some(number), keyboard);
                                    if let Some(action) = row {
                                        picked = Some(action);
                                    }
                                }
                            });
                        if shown.header_response.clicked() {
                            toggled = Some(number);
                        }
                        if cursor == Some(ScmRow::Revision(number)) {
                            egui_tree::list::cursor_frame(ui, shown.header_response.rect);
                            if moved {
                                shown.header_response.scroll_to_me(None);
                            }
                        }
                        // As GitLens' "Restore": the whole work as it was then.
                        shown.header_response.context_menu(|ui| {
                            let restore = Button::new(i18n.t(I18nKey::SideRestoreRevision)).destructive();
                            if ui.add(restore).on_hover_text(i18n.t(I18nKey::SideRestoreRevisionHover)).clicked() {
                                picked = Some(ChangeAction::RestoreRevision(revision.number));
                                ui.close();
                            }
                        });
                    }
                },
            );
        });
        if let Some(number) = toggled
            && !self.scm.open_revisions.remove(&number)
        {
            self.scm.open_revisions.insert(number);
        }
        match picked {
            Some(ChangeAction::Open(behavior, id)) => {
                self.views.list.active = None; // the object itself, not a diff of it
                if behavior != self.active {
                    self.active = behavior;
                    self.tabs.clear();
                }
                self.selected = Some(id.into());
            }
            Some(ChangeAction::Discard(behavior, id)) => {
                self.workspace.discard_change(behavior, &id);
                self.work_restored();
            }
            Some(ChangeAction::RestoreObject(behavior, id, number)) => {
                self.workspace.restore_object(behavior, &id, number);
                self.work_restored();
            }
            Some(ChangeAction::RestoreRevision(number)) => {
                self.workspace.restore_revision(number);
                self.work_restored();
            }
            Some(ChangeAction::Compare(behavior, id, from, to)) => self.open_compare(behavior, &id, from, to),
            Some(ChangeAction::Blame(behavior, id)) => self.open_blame(behavior, &id),
            None => {}
        }
    }

    /// `⎇ [main ▼] [+]`: the branch the work is on, as VS Code's status of git. Another one puts the
    /// work on it, unless that would lose its changes; `+` starts one where the work is.
    fn branch_row(&mut self, ui: &mut egui::Ui, i18n: &I18nMap, notify: &mut egui_widgets::notify::Notification) {
        let names: Vec<String> = self.workspace.history.branch_names().into_iter().map(str::to_owned).collect();
        let current = self.workspace.history.branch_name().to_owned();
        let mut switch = None;
        ui.horizontal(|ui| {
            ui.label("⎇").on_hover_text(i18n.t(I18nKey::SideBranchHover));
            ComboBox::from_id_salt("side_branch").selected_text(&current).show_ui(ui, |ui| {
                for name in &names {
                    if ui.selectable_label(*name == current, name).clicked() && *name != current {
                        switch = Some(name.clone());
                    }
                }
            });
            match &mut self.scm.new_branch {
                None => {
                    if ui
                        .add(Button::new("").icon(Icon::Plus).small())
                        .on_hover_text(i18n.t(I18nKey::SideNewBranch))
                        .clicked()
                    {
                        self.scm.new_branch = Some(String::new());
                    }
                }
                Some(name) => {
                    let field = ui.add(
                        egui::TextEdit::singleline(name).hint_text(i18n.t(I18nKey::SideNewBranch)).desired_width(120.0),
                    );
                    if field.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)) {
                        let name = std::mem::take(name);
                        self.scm.new_branch = None;
                        if self.workspace.history.create_branch(&name) {
                            self.mark_dirty();
                        }
                    } else if field.lost_focus() {
                        self.scm.new_branch = None;
                    } else {
                        field.request_focus();
                    }
                }
            }
        });
        if let Some(name) = switch {
            if self.workspace.switch_branch(&name) {
                self.work_restored();
            } else {
                notify.warn(i18n.t(I18nKey::SideBranchHasChanges));
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

    /// `OBJECT HISTORY  #0106`: the revisions that changed the selected object, newest first, as
    /// GitLens' File History. Each one's menu puts the object as it was then.
    fn object_history(&self, ui: &mut egui::Ui, i18n: &I18nMap) -> Option<ChangeAction> {
        let id = self.selected.as_deref()?;
        let behavior = self.workspace.behaviors.get(self.active)?;
        let key = (behavior.meshes_path.clone(), id.to_owned());
        let mut picked = None;
        let title = format!("{}  {id}", i18n.t(I18nKey::SideObjectHistory));
        egui::CollapsingHeader::new(title).id_salt("side_object_history").default_open(true).show(ui, |ui| {
            if ui.small_button(i18n.t(I18nKey::SideBlame)).on_hover_text(i18n.t(I18nKey::SideBlameHover)).clicked() {
                picked = Some(ChangeAction::Blame(self.active, id.to_owned()));
            }
            let mut any = false;
            for (revision, change) in self.workspace.history.of_object(&key) {
                any = true;
                let text = format!(
                    "{} #{} {}  {}",
                    change.kind.mark(),
                    revision.number,
                    revision.message,
                    local_time(revision.time)
                );
                // Cut with `…` to the width left, the whole on hover when cut, as VS Code's.
                let response = egui_widgets::fitted_button(ui, false, &text);
                if response.clicked() {
                    let parent = self.workspace.history.parent_number(revision.number);
                    picked = Some(ChangeAction::Compare(self.active, id.to_owned(), parent, Some(revision.number)));
                }
                response.context_menu(|ui| {
                    let parent = self.workspace.history.parent_number(revision.number);
                    if ui.button(i18n.t(I18nKey::SideComparePrevious)).clicked() {
                        picked = Some(ChangeAction::Compare(self.active, id.to_owned(), parent, Some(revision.number)));
                        ui.close();
                    }
                    if ui.button(i18n.t(I18nKey::SideCompareWorking)).clicked() {
                        picked = Some(ChangeAction::Compare(self.active, id.to_owned(), revision.number, None));
                        ui.close();
                    }
                    ui.separator();
                    if ui.add(Button::new(i18n.t(I18nKey::SideRestoreObject)).destructive()).clicked() {
                        picked = Some(ChangeAction::RestoreObject(self.active, id.to_owned(), revision.number));
                        ui.close();
                    }
                });
            }
            if !any {
                ui.weak(i18n.t(I18nKey::SideNoObjectHistory));
            }
        });
        picked
    }

    /// The keys of the Changes view, as in a tree (see [`egui_tree::navigate`]). Returns the row the
    /// keyboard is on (`None`: the side bar does not have it) and whether it moved this frame.
    fn scm_keys(&mut self, picked: &mut Option<ChangeAction>) -> (Option<ScmRow>, bool) {
        use crate::patch::changes::scm_rows;
        let log: Vec<(u32, usize)> =
            self.workspace.history.log().iter().map(|revision| (revision.number, revision.changes.len())).collect();
        let mut rows = scm_rows(self.scm.pending.changes.len(), &log, &self.scm.open_revisions);
        self.scm.cursor = self.scm.cursor.min(rows.len().saturating_sub(1));
        if !self.explorer.tree_focus {
            return (None, false);
        }
        let mut moved = false;
        for input in keybinding::host::take_mapped(&mut self.key_commands, keybinding::host::tree_key) {
            let nav: Vec<_> = rows.iter().map(|row| row.nav(&self.scm.open_revisions)).collect();
            match egui_tree::navigate(input, &nav, self.scm.cursor) {
                egui_tree::Nav::Cursor(at) => {
                    moved |= at != self.scm.cursor;
                    self.scm.cursor = at;
                }
                egui_tree::Nav::SetOpen(at, open) => {
                    if let ScmRow::Revision(number) = rows[at] {
                        if open {
                            self.scm.open_revisions.insert(number);
                        } else {
                            self.scm.open_revisions.remove(&number);
                        }
                    }
                    rows = scm_rows(self.scm.pending.changes.len(), &log, &self.scm.open_revisions);
                }
                egui_tree::Nav::Leave => self.leave_tree(),
                egui_tree::Nav::None => {}
            }
        }
        let row = rows.get(self.scm.cursor).copied();
        if moved {
            *picked = match row {
                Some(ScmRow::Change(at)) => self.scm.pending.changes.get(at).and_then(|change| {
                    let behavior = self.workspace.behaviors.iter().position(|b| b.meshes_path == change.behavior)?;
                    Some(ChangeAction::Open(behavior, change.id.clone()))
                }),
                Some(ScmRow::RevisionChange(number, at)) => {
                    let history = &self.workspace.history;
                    history.revision(number).and_then(|revision| revision.changes.get(at)).and_then(|change| {
                        let behavior =
                            self.workspace.behaviors.iter().position(|b| b.meshes_path == change.behavior)?;
                        let parent = history.parent_number(number);
                        Some(ChangeAction::Compare(behavior, change.id.clone(), parent, Some(number)))
                    })
                }
                _ => None,
            };
        }
        (row, moved)
    }

    /// `[message............] [Commit]`: a revision of the changes. `Ctrl+Enter` in the field too.
    fn commit_box(&mut self, ui: &mut egui::Ui, i18n: &I18nMap) {
        let can_commit = !self.scm.pending.changes.is_empty() && !self.scm.commit_message.trim().is_empty();
        let mut commit = false;
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let button = Button::new(i18n.t(I18nKey::SideCommit)).small().primary();
                commit |= ui.add_enabled(can_commit, button).on_hover_text(i18n.t(I18nKey::SideCommitHover)).clicked();
                let field = egui::TextEdit::singleline(&mut self.scm.commit_message)
                    .hint_text(i18n.t(I18nKey::SideCommitMessageHint))
                    .desired_width(ui.available_width());
                let response = ui.add(field);
                let ctrl_enter = ui.input(|input| input.modifiers.command && input.key_pressed(egui::Key::Enter));
                commit |= can_commit && response.lost_focus() && ctrl_enter;
            });
        });
        if commit {
            let time = crate::patch::workspace::history::now();
            let message = std::mem::take(&mut self.scm.commit_message);
            let workspace = &mut self.workspace;
            let mut history = std::mem::take(&mut workspace.history);
            history.commit(workspace, &message, time);
            workspace.history = history;
            self.mark_dirty();
        }
    }
}

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

/// `M  0_master  #0106  hkbModifierList  GetUpModList`. `revision`: `None` for a change since the last
/// revision, which can be discarded; else the revision of the history it is of, which the object can
/// be put back to. A removed one can not be opened.
///
/// `(under_cursor, moved)`: the keyboard is on it (a frame), and just went there (scrolled to).
fn change_row(
    ui: &mut egui::Ui,
    i18n: &I18nMap,
    (workspace, change): (&crate::patch::workspace::Workspace, &ObjectChange),
    revision: Option<u32>,
    (under_cursor, moved): (bool, bool),
) -> Option<ChangeAction> {
    let behavior = workspace.behaviors.iter().position(|behavior| behavior.meshes_path == change.behavior);
    let name = behavior.map_or_else(|| file_stem(&change.behavior), |at| workspace.behaviors[at].name.as_str());
    let object = behavior.and_then(|at| workspace.behaviors[at].object_text(&change.id));
    let mut picked = None;
    ui.horizontal(|ui| {
        // Source Control's colors: added green, modified yellow, removed red.
        let color = match change.kind {
            ChangeKind::Added => egui::Color32::from_rgb(0x73, 0xc9, 0x91),
            ChangeKind::Modified => egui::Color32::from_rgb(0xe2, 0xc0, 0x8d),
            ChangeKind::Removed => ui.visuals().error_fg_color,
        };
        ui.label(egui::RichText::new(change.kind.mark()).monospace().strong().color(color));
        let (class, label) = object.map_or(("", ""), |object| (object.class, object.name));
        let text = format!("{name}  {}  {class}  {label}", change.id);
        // Cut with `…` to the width left, the whole on hover when cut, as VS Code's.
        let response = egui_widgets::fitted_button(ui, false, &text);
        if under_cursor {
            egui_tree::list::cursor_frame(ui, response.rect);
            if moved {
                response.scroll_to_me(None);
            }
        }
        // As VS Code's Source Control: a change opens its object; a file of a revision, what that
        // revision changed of it.
        match (response.clicked(), behavior, revision) {
            (true, Some(at), Some(number)) => {
                let parent = workspace.history.parent_number(number);
                picked = Some(ChangeAction::Compare(at, change.id.clone(), parent, Some(number)));
            }
            (true, Some(at), None) if object.is_some() => picked = Some(ChangeAction::Open(at, change.id.clone())),
            _ => {}
        }
        if let Some(at) = behavior {
            response.context_menu(|ui| {
                let id = change.id.clone();
                // Of the history: what it changed, and against the work now.
                if let Some((number, parent)) = revision.map(|number| (number, workspace.history.parent_number(number)))
                {
                    if ui.button(i18n.t(I18nKey::SideComparePrevious)).clicked() {
                        picked = Some(ChangeAction::Compare(at, id.clone(), parent, Some(number)));
                        ui.close();
                    }
                    if ui.button(i18n.t(I18nKey::SideCompareWorking)).clicked() {
                        picked = Some(ChangeAction::Compare(at, id.clone(), number, None));
                        ui.close();
                    }
                    ui.separator();
                } else if ui.button(i18n.t(I18nKey::SideCompareWorking)).clicked() {
                    // A change: against the last revision.
                    picked = Some(ChangeAction::Compare(at, id.clone(), workspace.history.head_number(), None));
                    ui.close();
                }
                let (label, action) = match revision {
                    None => (I18nKey::SideDiscard, ChangeAction::Discard(at, id)),
                    Some(number) => (I18nKey::SideRestoreObject, ChangeAction::RestoreObject(at, id, number)),
                };
                if ui.add(Button::new(i18n.t(label)).destructive()).clicked() {
                    picked = Some(action);
                    ui.close();
                }
            });
        }
    });
    picked
}

/// `meshes/actors/character/behaviors/0_master.bin` -> `0_master`: a behavior the work no longer has.
fn file_stem(meshes_path: &str) -> &str {
    let file = meshes_path.rsplit('/').next().unwrap_or(meshes_path);
    file.rsplit_once('.').map_or(file, |(stem, _)| stem)
}

/// Seconds since the Unix epoch as the local time, e.g. `2026-11-12 18:00`.
fn local_time(seconds: u64) -> String {
    let seconds = i64::try_from(seconds).unwrap_or_default();
    chrono::DateTime::from_timestamp(seconds, 0)
        .map(|time| time.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_default()
}
