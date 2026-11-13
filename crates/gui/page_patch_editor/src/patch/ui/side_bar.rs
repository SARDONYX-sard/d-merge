//! The side bar, laid out as VS Code's: an activity bar of views at the far left, and the view picked
//! beside it. A click on the icon of the view shown hides the side bar, and brings it back.
//!
//! ```text
//! ┌──┬────────────────────────────────┐
//! │🔍│ EXPLORER                       │  Ctrl+Shift+E
//! │Δ³│ ▼ BEHAVIORS               [+]  │
//! │📂│   0_master (3)                 │
//! │▤ │ ▼ OBJECTS          [+] [≡] [⋯] │
//! │  │ [filter..................]     │
//! │  │   (the tree or the list)       │
//! └──┴────────────────────────────────┘
//! ```
//!
//! | Icon | View               | Key            | VS Code's        |
//! | ---- | ------------------ | -------------- | ---------------- |
//! | 🔍   | Search             | `Ctrl+Shift+F` | Search           |
//! | Δ    | Changes (+ badge)  | `Ctrl+Shift+G` | Source Control   |
//! | 📂   | Explorer           | `Ctrl+Shift+E` | Explorer         |
//! | ▤    | Events / Variables |                | Outline          |

use egui_widgets::shadcn::{Button, ComboBox, Icon};
use gui_shared::i18n::{I18nKey, I18nMap};

use crate::patch::{
    PatchTool,
    workspace::history::{ChangeKind, ObjectChange},
};

/// The view of the side bar. See the module docs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(in crate::patch) enum SideView {
    #[default]
    Explorer,
    Search,
    Changes,
    Variables,
}

/// What the side bar shows.
#[derive(Debug, Default)]
pub(in crate::patch) struct SideBar {
    pub(in crate::patch) view: SideView,
    /// Only the activity bar shows: the icon of the view shown was clicked.
    pub(in crate::patch) hidden: bool,
    /// The OBJECTS section of the explorer is folded.
    pub(in crate::patch) objects_closed: bool,
}

/// The views in the activity bar, top to bottom: `(view, icon, name, key)`.
/// The order is the user's (not VS Code's): from the bottom, events / variables, explorer, changes,
/// search.
const VIEWS: [(SideView, Icon, I18nKey, &str); 4] = [
    (SideView::Search, Icon::Search, I18nKey::SearchLabel, "Ctrl+Shift+F"),
    (SideView::Changes, Icon::Diff, I18nKey::SideChanges, "Ctrl+Shift+G"),
    (SideView::Explorer, Icon::FolderOpen, I18nKey::SideExplorer, "Ctrl+Shift+E"),
    (SideView::Variables, Icon::Layers, I18nKey::NemesisVarsTab, ""),
];

/// The width of the activity bar, and the size of its buttons.
const ACTIVITY_SIZE: f32 = 40.0;
/// The size of an icon in it.
const ACTIVITY_ICON: f32 = 22.0;

impl PatchTool {
    /// The activity bar and, unless hidden, the side bar with its view.
    pub(in crate::patch) fn side_bar(
        &mut self,
        ui: &mut egui::Ui,
        i18n: &I18nMap,
        notify: &mut egui_widgets::notify::Notification,
    ) {
        self.side_bar_keys();
        self.file_keys(ui.ctx(), notify);
        let id = if self.is_user_patch { "user_patch" } else { "nemesis" };
        egui::Panel::left((id, "activity_bar"))
            .resizable(false)
            .exact_size(ACTIVITY_SIZE)
            .show(ui, |ui| self.activity_bar(ui, i18n));
        if self.side.hidden {
            return;
        }
        egui::Panel::left((id, "side_bar")).resizable(true).default_size(320.0).show(ui, |ui| {
            let (_, _, title, _) = VIEWS.iter().find(|(view, ..)| *view == self.side.view).copied().unwrap_or(VIEWS[0]);
            match self.side.view {
                SideView::Explorer => {
                    view_title(ui, i18n.t(title));
                    self.behaviors_section(ui, i18n, notify);
                    if !self.workspace.behaviors.is_empty() {
                        self.object_list(ui, i18n, notify);
                    }
                }
                SideView::Search => {
                    view_title(ui, i18n.t(title));
                    if !self.workspace.behaviors.is_empty() {
                        self.search_panel(ui, i18n);
                    }
                }
                SideView::Changes => self.changes_view(ui, i18n, notify),
                SideView::Variables => {
                    view_title(ui, i18n.t(title));
                    // The events and variables are a behavior's: which one, here as well.
                    self.behavior_select(ui, i18n);
                    let has_vars =
                        self.workspace.behaviors.get(self.active).is_some_and(|behavior| !behavior.is_anim_data());
                    if has_vars {
                        self.vars_panel(ui, i18n);
                    } else {
                        ui.weak(i18n.t(I18nKey::SideNoVariables));
                    }
                }
            }
        });
    }

    /// `Template: [0_master ▼]`: the behavior the view is of (animation data has no events or
    /// variables). Another one goes back to where it was left in it, as a click in BEHAVIORS does.
    fn behavior_select(&mut self, ui: &mut egui::Ui, i18n: &I18nMap) {
        let current = self.workspace.behaviors.get(self.active).map_or("", |behavior| behavior.name.as_str());
        let mut picked = None;
        ui.horizontal(|ui| {
            ui.label(i18n.t(I18nKey::SideTemplate));
            egui_widgets::shadcn::ComboBox::from_id_salt("side_vars_behavior").selected_text(current).show_ui(
                ui,
                |ui| {
                    let behaviors = self.workspace.behaviors.iter().enumerate();
                    for (at, behavior) in behaviors.filter(|(_, behavior)| !behavior.is_anim_data()) {
                        if ui.selectable_label(at == self.active, &behavior.name).clicked() {
                            picked = Some(at);
                        }
                    }
                },
            );
        });
        if let Some(at) = picked.filter(|at| *at != self.active) {
            self.active = at;
            self.selected = self.tabs.last_of(at).map(std::rc::Rc::from);
        }
    }

    /// Shows `view`, or hides the side bar if it is the one shown, as a click on its icon does.
    pub(in crate::patch) fn toggle_side_view(&mut self, view: SideView) {
        if self.side.view == view && !self.side.hidden {
            self.side.hidden = true;
            return;
        }
        self.side.view = view;
        self.side.hidden = false;
        self.focus_search |= view == SideView::Search;
    }

    /// `Ctrl+Shift+E` / `F` / `G` (key bindings), as in VS Code. A key shows its view, and does not
    /// hide it.
    fn side_bar_keys(&mut self) {
        use keybinding::commands::{SIDE_BAR_CHANGES, SIDE_BAR_EXPLORER, SIDE_BAR_SEARCH};
        let keys = [
            (SIDE_BAR_EXPLORER, SideView::Explorer),
            (SIDE_BAR_SEARCH, SideView::Search),
            (SIDE_BAR_CHANGES, SideView::Changes),
        ];
        for (command, view) in keys {
            if self.key_command(command) {
                self.side.view = view;
                self.side.hidden = false;
                self.focus_search |= view == SideView::Search;
            }
        }
    }

    /// The icons of [`VIEWS`]: the one shown has a bar on its left, as in VS Code; Changes has the
    /// count of changes on it.
    fn activity_bar(&mut self, ui: &mut egui::Ui, i18n: &I18nMap) {
        self.scm.pending.refresh(&self.workspace, self.edits);
        let changes = self.scm.pending.changes.len();
        let mut clicked = None;
        for (view, icon, name, key) in VIEWS {
            let shown = view == self.side.view && !self.side.hidden;
            // As wide as the panel lets it: `ACTIVITY_SIZE` is the panel's, its margins included.
            let size = egui::vec2(ui.available_width(), ACTIVITY_SIZE);
            let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
            let visuals = ui.visuals();
            let color =
                if shown || response.hovered() { visuals.strong_text_color() } else { visuals.weak_text_color() };
            let icon_rect = egui::Rect::from_center_size(rect.center(), egui::Vec2::splat(ACTIVITY_ICON));
            egui_widgets::shadcn::icon::paint_icon(ui.painter(), icon_rect, icon, color);
            if shown {
                let bar = egui::Rect::from_min_size(rect.min, egui::vec2(2.0, rect.height()));
                ui.painter().rect_filled(bar, 0.0, visuals.selection.stroke.color);
            }
            if view == SideView::Changes && changes > 0 {
                badge(ui, rect, changes);
            }
            let hover = if key.is_empty() { i18n.t(name).to_owned() } else { format!("{} ({key})", i18n.t(name)) };
            if response.on_hover_text(hover).clicked() {
                clicked = Some(view);
            }
        }
        if let Some(view) = clicked {
            self.toggle_side_view(view);
        }
    }

    /// `CHANGES`, the message and `Commit`, the changes since the last revision, and the history:
    /// like VS Code's Source Control. See [`crate::patch::workspace::history`].
    fn changes_view(&mut self, ui: &mut egui::Ui, i18n: &I18nMap, notify: &mut egui_widgets::notify::Notification) {
        view_title(ui, i18n.t(I18nKey::SideChanges));
        self.branch_row(ui, i18n, notify);
        self.scm.pending.refresh(&self.workspace, self.edits);
        self.commit_box(ui, i18n);

        let mut picked = None;
        egui::ScrollArea::both().id_salt("side_changes").auto_shrink(false).show(ui, |ui| {
            let title = format!("{} ({})", i18n.t(I18nKey::SideChanges), self.scm.pending.changes.len());
            egui::CollapsingHeader::new(title).id_salt("side_pending").default_open(true).show(ui, |ui| {
                if self.scm.pending.changes.is_empty() {
                    ui.weak(i18n.t(I18nKey::SideNoChanges));
                }
                for change in &self.scm.pending.changes {
                    if let Some(action) = change_row(ui, i18n, &self.workspace, change, None) {
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
                        let shown = egui::CollapsingHeader::new(title)
                            .id_salt(("side_revision", revision.number))
                            .show(ui, |ui| {
                                for change in &revision.changes {
                                    let row = change_row(ui, i18n, &self.workspace, change, Some(revision.number));
                                    if let Some(action) = row {
                                        picked = Some(action);
                                    }
                                }
                            });
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
        match picked {
            Some(ChangeAction::Open(behavior, id)) => {
                self.views.shown = None; // the object itself, not a diff of it
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
                let response = ui.add(egui::Button::selectable(false, egui::RichText::new(&text).monospace()));
                // Cut off at the side bar's edge: the whole row on hover.
                let response = egui_widgets::CutHover::new().show_text(ui, response, &text);
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
fn change_row(
    ui: &mut egui::Ui,
    i18n: &I18nMap,
    workspace: &crate::patch::workspace::Workspace,
    change: &ObjectChange,
    revision: Option<u32>,
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
        let response = ui.add(egui::Button::selectable(false, egui::RichText::new(text).monospace()));
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

/// `EXPLORER`: the name of the view, as VS Code puts it at the top of the side bar.
fn view_title(ui: &mut egui::Ui, title: &str) {
    ui.label(egui::RichText::new(title.to_uppercase()).small().strong());
    ui.add_space(2.0);
}

/// The count at the bottom right of an icon, as VS Code's badges. e.g. `3`, or `99+`.
fn badge(ui: &egui::Ui, icon: egui::Rect, count: usize) {
    let text = if count > 99 { "99+".to_owned() } else { count.to_string() };
    let font = egui::FontId::proportional(9.0);
    let painter = ui.painter();
    let galley = painter.layout_no_wrap(text, font, egui::Color32::WHITE);
    let size = egui::vec2(galley.size().x.max(galley.size().y) + 4.0, galley.size().y + 2.0);
    let rect = egui::Rect::from_min_size(icon.right_bottom() - size - egui::vec2(4.0, 4.0), size);
    painter.rect_filled(rect, size.y / 2.0, ui.visuals().selection.bg_fill);
    painter.galley(rect.center() - galley.size() / 2.0, galley, egui::Color32::WHITE);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// As VS Code's icons: a click shows the view, a click on the one shown hides the side bar.
    #[test]
    fn a_view_icon_shows_or_hides() {
        let mut tool = PatchTool::default();
        tool.toggle_side_view(SideView::Search);
        assert_eq!((tool.side.view, tool.side.hidden, tool.focus_search), (SideView::Search, false, true));
        tool.toggle_side_view(SideView::Search);
        assert!(tool.side.hidden);
        tool.toggle_side_view(SideView::Changes);
        assert_eq!((tool.side.view, tool.side.hidden), (SideView::Changes, false));
    }
}
