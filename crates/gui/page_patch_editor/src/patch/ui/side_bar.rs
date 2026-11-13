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

use egui_widgets::shadcn::Icon;
use gui_shared::i18n::{I18nKey, I18nMap};

use crate::patch::PatchTool;

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

impl PatchTool {
    /// The activity bar and, unless hidden, the side bar with its view.
    pub(in crate::patch) fn side_bar(
        &mut self,
        ui: &mut egui::Ui,
        i18n: &I18nMap,
        notify: &mut egui_widgets::notify::Notification,
    ) {
        self.side_bar_keys(ui);
        self.file_keys(ui.ctx(), notify);
        let id = if self.is_user_patch { "user_patch" } else { "nemesis" };
        egui::Panel::left((id, "activity_bar"))
            .resizable(false)
            .exact_size(egui_widgets::ActivityBar::SIZE)
            .show(ui, |ui| self.activity_bar(ui, i18n));
        if self.side.hidden {
            return;
        }
        egui::Panel::left((id, "side_bar")).resizable(true).default_size(320.0).show(ui, |ui| {
            let (_, _, title, _) = VIEWS.iter().find(|(view, ..)| *view == self.side.view).copied().unwrap_or(VIEWS[0]);
            match self.side.view {
                SideView::Explorer => {
                    egui_widgets::side_bar_title(ui, i18n.t(title));
                    self.behaviors_section(ui, i18n, notify);
                    if !self.workspace.behaviors.is_empty() {
                        self.object_list(ui, i18n, notify);
                    }
                }
                SideView::Search => {
                    egui_widgets::side_bar_title(ui, i18n.t(title));
                    if !self.workspace.behaviors.is_empty() {
                        self.search_panel(ui, i18n);
                    }
                }
                SideView::Changes => self.changes_view(ui, i18n, notify),
                SideView::Variables => {
                    egui_widgets::side_bar_title(ui, i18n.t(title));
                    // The events and variables are a behavior's: which one, here as well.
                    self.behavior_select(ui, i18n);
                    let has_vars =
                        self.workspace.behaviors.get(self.active).is_some_and(|behavior| !behavior.is_anim_data());
                    if has_vars {
                        let names = self.names();
                        let behavior = &mut self.workspace.behaviors[self.active];
                        let changed = self.vars.show(ui, i18n, behavior, &names, &self.explorer.filter);
                        if !changed.is_empty() {
                            self.pane.opened = None; // the open object may be one of them
                        }
                        for id in changed {
                            self.object_changed(&id);
                        }
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
    ///
    /// As VS Code's, the view shown has the keyboard then (the search, its field).
    fn side_bar_keys(&mut self, ui: &egui::Ui) {
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
                if view != SideView::Search {
                    self.explorer.tree_focus = true;
                    ui.memory_mut(|m| {
                        if let Some(focused) = m.focused() {
                            m.surrender_focus(focused);
                        }
                    });
                }
            }
        }
    }

    /// The icons of [`VIEWS`] ([`egui_widgets::ActivityBar`]); Changes has the count of changes on it.
    fn activity_bar(&mut self, ui: &mut egui::Ui, i18n: &I18nMap) {
        self.scm.pending.refresh(&self.workspace, self.edits);
        let changes = self.scm.pending.changes.len();
        let hovers =
            VIEWS.map(
                |(_, _, name, key)| {
                    if key.is_empty() { i18n.t(name).to_owned() } else { format!("{} ({key})", i18n.t(name)) }
                },
            );
        let items = VIEWS.iter().zip(&hovers).map(|(&(view, icon, ..), hover)| egui_widgets::ActivityItem {
            icon,
            hover,
            shown: view == self.side.view && !self.side.hidden,
            badge: if view == SideView::Changes { changes } else { 0 },
        });
        if let Some(at) = egui_widgets::ActivityBar::new().show(ui, items) {
            self.toggle_side_view(VIEWS[at].0);
        }
    }
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
