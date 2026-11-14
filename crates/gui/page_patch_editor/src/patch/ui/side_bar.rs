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

impl SideBar {
    /// A click on `view`'s icon: shows it, or hides the side bar if it is the one shown. Returns
    /// whether `view` is shown now.
    pub(in crate::patch) fn toggle(&mut self, view: SideView) -> bool {
        if self.view == view && !self.hidden {
            self.hidden = true;
            return false;
        }
        self.show(view);
        true
    }

    /// Shows `view` (a key of it: it never hides).
    pub(in crate::patch) const fn show(&mut self, view: SideView) {
        self.view = view;
        self.hidden = false;
    }
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
        let clicked = egui::Panel::left((id, "activity_bar"))
            .resizable(false)
            .exact_size(egui_widgets::ActivityBar::SIZE)
            .show(ui, |ui| {
                self.scm.pending.refresh(&self.work.workspace, self.work.edits);
                activity_bar(ui, i18n, &self.side, self.scm.pending.changes.len())
            })
            .inner;
        if let Some(view) = clicked {
            self.toggle_side_view(view);
        }
        if self.side.hidden {
            return;
        }
        egui::Panel::left((id, "side_bar")).resizable(true).default_size(320.0).show(ui, |ui| {
            let (_, _, title, _) = VIEWS.iter().find(|(view, ..)| *view == self.side.view).copied().unwrap_or(VIEWS[0]);
            match self.side.view {
                SideView::Explorer => {
                    egui_widgets::side_bar_title(ui, i18n.t(title));
                    self.behaviors_section(ui, i18n, notify);
                    if !self.work.workspace.behaviors.is_empty() {
                        self.object_list(ui, i18n, notify);
                    }
                }
                SideView::Search => {
                    egui_widgets::side_bar_title(ui, i18n.t(title));
                    if !self.work.workspace.behaviors.is_empty() {
                        self.search_panel(ui, i18n);
                    }
                }
                SideView::Changes => self.changes_view(ui, i18n, notify),
                SideView::Variables => {
                    egui_widgets::side_bar_title(ui, i18n.t(title));
                    // The events and variables are a behavior's: which one, here as well.
                    let behaviors = &self.work.workspace.behaviors;
                    if let Some(at) = behavior_select(ui, i18n, behaviors, self.active) {
                        self.active = at;
                        self.selected = self.tabs.last_of(at).map(std::rc::Rc::from);
                    }
                    let has_vars =
                        self.work.workspace.behaviors.get(self.active).is_some_and(|behavior| !behavior.is_anim_data());
                    if has_vars {
                        let names = self.names();
                        let behavior = &mut self.work.workspace.behaviors[self.active];
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

    /// Shows `view`, or hides the side bar if it is the one shown, as a click on its icon does.
    pub(in crate::patch) fn toggle_side_view(&mut self, view: SideView) {
        if self.side.toggle(view) && view == SideView::Search {
            self.search.focus();
        }
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
            if self.keys.take(command) {
                self.side.show(view);
                if view == SideView::Search {
                    self.search.focus();
                } else {
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
}

/// The icons of [`VIEWS`] ([`egui_widgets::ActivityBar`]); Changes has the count of `changes` on it.
/// Returns the view clicked.
fn activity_bar(ui: &mut egui::Ui, i18n: &I18nMap, side: &SideBar, changes: usize) -> Option<SideView> {
    let hovers = VIEWS.map(
        |(_, _, name, key)| {
            if key.is_empty() { i18n.t(name).to_owned() } else { format!("{} ({key})", i18n.t(name)) }
        },
    );
    let items = VIEWS.iter().zip(&hovers).map(|(&(view, icon, ..), hover)| egui_widgets::ActivityItem {
        icon,
        hover,
        shown: view == side.view && !side.hidden,
        badge: if view == SideView::Changes { changes } else { 0 },
    });
    egui_widgets::ActivityBar::new().show(ui, items).map(|at| VIEWS[at].0)
}

/// `Template: [0_master ▼]`: the behavior the EVENTS / VARIABLES view is of (animation data has
/// none). Returns another one picked.
fn behavior_select(
    ui: &mut egui::Ui,
    i18n: &I18nMap,
    behaviors: &[crate::patch::workspace::Behavior],
    active: usize,
) -> Option<usize> {
    let current = behaviors.get(active).map_or("", |behavior| behavior.name.as_str());
    let mut picked = None;
    ui.horizontal(|ui| {
        ui.label(i18n.t(I18nKey::SideTemplate));
        let combo = egui_widgets::shadcn::ComboBox::from_id_salt("side_vars_behavior").selected_text(current);
        combo.show_ui(ui, |ui| {
            let shown = behaviors.iter().enumerate().filter(|(_, behavior)| !behavior.is_anim_data());
            for (at, behavior) in shown {
                if ui.selectable_label(at == active, &behavior.name).clicked() {
                    picked = Some(at);
                }
            }
        });
    });
    picked.filter(|at| *at != active)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// As VS Code's icons: a click shows the view, a click on the one shown hides the side bar.
    #[test]
    fn a_view_icon_shows_or_hides() {
        let mut side = SideBar::default();
        assert!(side.toggle(SideView::Search));
        assert_eq!((side.view, side.hidden), (SideView::Search, false));
        assert!(!side.toggle(SideView::Search));
        assert!(side.hidden);
        assert!(side.toggle(SideView::Changes));
        assert_eq!((side.view, side.hidden), (SideView::Changes, false));
    }
}
