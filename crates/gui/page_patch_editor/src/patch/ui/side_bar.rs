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

use egui_widgets::shadcn::{Button, Icon};
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
        self.side_bar_keys(ui);
        let id = if self.is_user_patch { "user_patch" } else { "nemesis" };
        egui::Panel::left((id, "activity_bar"))
            .resizable(false)
            .exact_size(ACTIVITY_SIZE)
            .show(ui, |ui| self.activity_bar(ui, i18n));
        if self.side_bar_hidden {
            return;
        }
        egui::Panel::left((id, "side_bar")).resizable(true).default_size(320.0).show(ui, |ui| {
            let (_, _, title, _) = VIEWS.iter().find(|(view, ..)| *view == self.side_view).copied().unwrap_or(VIEWS[0]);
            match self.side_view {
                SideView::Explorer => {
                    view_title(ui, i18n.t(title));
                    self.behaviors_section(ui, i18n, notify);
                    if !self.workspace.behaviors.is_empty() {
                        self.object_list(ui, i18n);
                    }
                }
                SideView::Search => {
                    view_title(ui, i18n.t(title));
                    if !self.workspace.behaviors.is_empty() {
                        self.search_panel(ui, i18n);
                    }
                }
                SideView::Changes => self.changes_view(ui, i18n),
                SideView::Variables => {
                    view_title(ui, i18n.t(title));
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

    /// Shows `view`, or hides the side bar if it is the one shown, as a click on its icon does.
    pub(in crate::patch) fn toggle_side_view(&mut self, view: SideView) {
        if self.side_view == view && !self.side_bar_hidden {
            self.side_bar_hidden = true;
            return;
        }
        self.side_view = view;
        self.side_bar_hidden = false;
        self.focus_search |= view == SideView::Search;
    }

    /// `Ctrl+Shift+E` / `F` / `G`, as in VS Code. A key shows its view, and does not hide it.
    fn side_bar_keys(&mut self, ui: &egui::Ui) {
        use egui::{Key, Modifiers};
        let keys = [(Key::E, SideView::Explorer), (Key::F, SideView::Search), (Key::G, SideView::Changes)];
        for (key, view) in keys {
            if ui.input_mut(|input| input.consume_key(Modifiers::COMMAND | Modifiers::SHIFT, key)) {
                self.side_view = view;
                self.side_bar_hidden = false;
                self.focus_search |= view == SideView::Search;
            }
        }
    }

    /// The icons of [`VIEWS`]: the one shown has a bar on its left, as in VS Code; Changes has the
    /// count of changes on it.
    fn activity_bar(&mut self, ui: &mut egui::Ui, i18n: &I18nMap) {
        self.pending.refresh(&self.workspace, self.edits);
        let changes = self.pending.changes.len();
        let mut clicked = None;
        for (view, icon, name, key) in VIEWS {
            let shown = view == self.side_view && !self.side_bar_hidden;
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

    /// `CHANGES  [✓]`, the message and `Commit`, the changes since the last revision, and the history:
    /// like VS Code's Source Control. See [`crate::patch::workspace::history`].
    fn changes_view(&mut self, ui: &mut egui::Ui, i18n: &I18nMap) {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(i18n.t(I18nKey::SideChanges).to_uppercase()).small().strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // As Source Control's title actions: on all that changed.
                let check = Button::new("").icon(Icon::ScanSearch).small();
                let hover = |ui: &mut egui::Ui| {
                    ui.strong(i18n.t(I18nKey::NemesisCheckWork));
                    ui.label(i18n.t(I18nKey::NemesisCheckWorkHover));
                };
                if ui.add(check).on_hover_ui(hover).clicked() {
                    self.start_whole_work_check();
                }
            });
        });
        self.pending.refresh(&self.workspace, self.edits);
        self.commit_box(ui, i18n);

        let mut picked = None;
        egui::ScrollArea::both().id_salt("side_changes").auto_shrink(false).show(ui, |ui| {
            let title = format!("{} ({})", i18n.t(I18nKey::SideChanges), self.pending.changes.len());
            egui::CollapsingHeader::new(title).id_salt("side_pending").default_open(true).show(ui, |ui| {
                if self.pending.changes.is_empty() {
                    ui.weak(i18n.t(I18nKey::SideNoChanges));
                }
                for change in &self.pending.changes {
                    if let Some(action) = change_row(ui, i18n, &self.workspace, change, true) {
                        picked = Some(action);
                    }
                }
            });
            egui::CollapsingHeader::new(i18n.t(I18nKey::SideHistory)).id_salt("side_history").default_open(true).show(
                ui,
                |ui| {
                    for revision in self.workspace.history.revisions.iter().rev() {
                        let title = format!(
                            "#{} {}  {}  ({})",
                            revision.number,
                            revision.message,
                            local_time(revision.time),
                            revision.changes.len()
                        );
                        egui::CollapsingHeader::new(title).id_salt(("side_revision", revision.number)).show(ui, |ui| {
                            for change in &revision.changes {
                                if let Some(action) = change_row(ui, i18n, &self.workspace, change, false) {
                                    picked = Some(action);
                                }
                            }
                        });
                    }
                },
            );
        });
        match picked {
            Some(ChangeAction::Open(behavior, id)) => {
                if behavior != self.active {
                    self.active = behavior;
                    self.tabs.clear();
                }
                self.selected = Some(id.into());
            }
            Some(ChangeAction::Discard(behavior, id)) => {
                self.workspace.discard_change(behavior, &id);
                self.reset_names();
                self.editor_for = None;
                self.preview = None;
                self.mark_dirty();
            }
            None => {}
        }
    }

    /// `[message............] [Commit]`: a revision of the changes. `Ctrl+Enter` in the field too.
    fn commit_box(&mut self, ui: &mut egui::Ui, i18n: &I18nMap) {
        let can_commit = !self.pending.changes.is_empty() && !self.commit_message.trim().is_empty();
        let mut commit = false;
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let button = Button::new(i18n.t(I18nKey::SideCommit)).small().primary();
                commit |= ui.add_enabled(can_commit, button).on_hover_text(i18n.t(I18nKey::SideCommitHover)).clicked();
                let field = egui::TextEdit::singleline(&mut self.commit_message)
                    .hint_text(i18n.t(I18nKey::SideCommitMessageHint))
                    .desired_width(ui.available_width());
                let response = ui.add(field);
                let ctrl_enter = ui.input(|input| input.modifiers.command && input.key_pressed(egui::Key::Enter));
                commit |= can_commit && response.lost_focus() && ctrl_enter;
            });
        });
        if commit {
            let time =
                std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |since| since.as_secs());
            let message = std::mem::take(&mut self.commit_message);
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
}

/// `M  0_master  #0106  hkbModifierList  GetUpModList`. `pending`: a change since the last revision,
/// which can be discarded; else one of the history. A removed one can not be opened.
fn change_row(
    ui: &mut egui::Ui,
    i18n: &I18nMap,
    workspace: &crate::patch::workspace::Workspace,
    change: &ObjectChange,
    pending: bool,
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
        if let (true, Some(at)) = (response.clicked(), behavior.filter(|_| object.is_some())) {
            picked = Some(ChangeAction::Open(at, change.id.clone()));
        }
        if let (true, Some(at)) = (pending, behavior) {
            response.context_menu(|ui| {
                if ui.add(Button::new(i18n.t(I18nKey::SideDiscard)).destructive()).clicked() {
                    picked = Some(ChangeAction::Discard(at, change.id.clone()));
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
        assert_eq!((tool.side_view, tool.side_bar_hidden, tool.focus_search), (SideView::Search, false, true));
        tool.toggle_side_view(SideView::Search);
        assert!(tool.side_bar_hidden);
        tool.toggle_side_view(SideView::Changes);
        assert_eq!((tool.side_view, tool.side_bar_hidden), (SideView::Changes, false));
    }
}
