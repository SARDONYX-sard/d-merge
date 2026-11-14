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

mod anim_tree;
mod behaviors;
mod changes;
mod changes_view;
mod explorer;
mod object_list;
mod search;
mod tree;
mod vars_panel;

use egui_widgets::shadcn::Icon;
use gui_shared::{
    i18n::{I18nKey, I18nMap},
    settings::pages::patch_editor::PatchEditorSettings,
};

use crate::patch::{
    tool::{file::FileAction, keys::Keys, selection::Selection, work_state::Work},
    tree::TreeState,
    ui::windows::WindowAsk,
};

/// The view of the side bar. See the module docs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum SideView {
    #[default]
    Explorer,
    Search,
    Changes,
    Variables,
}

/// The side bar and the state of its views, as one owner: what is drawn there sees this, not the
/// whole tool.
///
/// ```text
/// bar       which view, hidden or not          (the activity bar)
/// explorer  the objects: filters, tree / list   (EXPLORER)
/// scm       the changes, the commit message     (CHANGES)
/// search    the search of every object          (SEARCH)
/// vars      the events / variables panel        (EVENTS / VARIABLES)
/// ```
#[derive(Debug, Default)]
pub(in crate::patch) struct SideArea {
    bar: SideBar,
    explorer: explorer::Explorer,
    scm: changes::Scm,
    search: egui_code_editor::SearchView,
    vars: vars_panel::VarsPanel,
}

impl SideArea {
    /// The user patch's: its behaviors listed, and only what it changes.
    pub(in crate::patch) fn for_user_patch() -> Self {
        let mut side = Self::default();
        side.bar.behaviors_open = true;
        side.explorer.changed_only = true;
        side
    }

    /// The first frame: what the settings kept. The behaviors are listed.
    pub(in crate::patch) fn restore(&mut self, settings: &PatchEditorSettings) {
        self.bar.behaviors_open = true;
        self.explorer.trees = settings
            .tree_open
            .iter()
            .map(|(path, open)| {
                let open = open.iter().map(|(id, open)| (id.as_str(), *open));
                (path.clone(), TreeState::restored(open))
            })
            .collect();
    }

    /// A tree or a list, as the settings have it.
    pub(in crate::patch) const fn read_view(&mut self, settings: &PatchEditorSettings) {
        self.explorer.tree_view = settings.tree_view;
    }

    pub(in crate::patch) const fn write_view(&self, settings: &mut PatchEditorSettings) {
        settings.tree_view = self.explorer.tree_view;
    }

    /// Only the changed objects, as the settings have it.
    pub(in crate::patch) const fn read_filter(&mut self, settings: &PatchEditorSettings) {
        self.explorer.changed_only = settings.changed_only;
    }

    /// The filter and what was opened or closed of the trees, into the settings.
    pub(in crate::patch) fn write_settings(&mut self, settings: &mut PatchEditorSettings) {
        settings.changed_only = self.explorer.changed_only;
        for (path, tree) in &mut self.explorer.trees {
            if let Some(open) = tree.take_open_changes() {
                let open = open.map(|(id, open)| (id.to_owned(), open)).collect();
                settings.tree_open.insert(path.clone(), open);
            }
        }
    }

    /// What the key bindings see of the side bar: `(it has the keyboard, "tree" or "list")`.
    pub(in crate::patch) const fn key_state(&self) -> (bool, &'static str) {
        (self.explorer.tree_focus, if self.explorer.tree_view { "tree" } else { "list" })
    }

    /// The editor got the keyboard: the selected object is shown again, as VS Code's explorer
    /// follows the file being edited.
    pub(in crate::patch) const fn reveal(&mut self) {
        self.explorer.reveal = true;
    }

    /// Something else took the keyboard (the Problems panel).
    pub(in crate::patch) const fn leave_tree(&mut self) {
        self.explorer.tree_focus = false;
    }

    /// Files went to project `folder` of the animation data: it opens to show them.
    pub(in crate::patch) fn show_project(&mut self, folder: String) {
        self.explorer.anim_tree.show_project(folder);
    }

    /// The tree of the behavior at `path`, for the tests.
    #[cfg(test)]
    pub(in crate::patch) fn tree(&self, path: &str) -> Option<&TreeState> {
        self.explorer.trees.get(path)
    }
}

/// What the side bar shows.
#[derive(Debug, Default)]
struct SideBar {
    view: SideView,
    /// Only the activity bar shows: the icon of the view shown was clicked.
    hidden: bool,
    /// The OBJECTS section of the explorer is folded.
    pub(super) objects_closed: bool,
    /// The BEHAVIORS section of the explorer is open.
    behaviors_open: bool,
}

impl SideBar {
    /// A click on `view`'s icon: shows it, or hides the side bar if it is the one shown. Returns
    /// whether `view` is shown now.
    fn toggle(&mut self, view: SideView) -> bool {
        if self.view == view && !self.hidden {
            self.hidden = true;
            return false;
        }
        self.show(view);
        true
    }

    /// Shows `view` (a key of it: it never hides).
    const fn show(&mut self, view: SideView) {
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

/// What the side bar asks of the rest: the editor shows something, the keyboard goes to it, a file.
/// The side bar itself only changes the work and the selection.
#[derive(Debug)]
pub(in crate::patch) enum SideAsk {
    /// Behavior `.0` is shown, at the object it showed last.
    ShowBehavior(usize),
    /// A behavior was removed: the numbers of the others shift, so the tabs close.
    BehaviorRemoved,
    /// Shows the object in a tab that stays.
    KeepOpen(String),
    /// The object at a line of it (the search).
    OpenLocation(String, usize),
    /// The object `(behavior, id)` itself, not a diff of it (the changes).
    OpenChange(usize, String),
    /// The object `(behavior, id)` from revision `.2` to `.3` (`None`: the work now).
    Compare(usize, String, u32, Option<u32>),
    /// Each line of the object `(behavior, id)` with the revision that last changed it.
    Blame(usize, String),
    /// Deletes a new object, or drops the edits of a template object.
    RevertOrDelete(String),
    /// The Pandora patch file of the active behavior.
    OpenPandoraDoc,
    /// Copy the path of / reveal the file of an object.
    File(FileAction, String),
    /// The keyboard goes back to the editor.
    FocusEditor,
    /// Objects changed under the editor: it loads the open one again.
    Reload,
    /// A window to open.
    Window(WindowAsk),
}

impl SideArea {
    /// The activity bar and, unless hidden, the side bar with its view. Returns what it asked of the
    /// rest, see [`SideAsk`].
    pub(in crate::patch) fn show(
        &mut self,
        ui: &mut egui::Ui,
        i18n: &I18nMap,
        (work, sel, keys): (&mut Work, &mut Selection, &mut Keys),
        notify: &mut egui_widgets::notify::Notification,
    ) -> Vec<SideAsk> {
        let mut asks = Vec::new();
        self.view_keys(ui, keys);
        asks.extend(file_keys(keys, sel));

        let id = if work.user_patch { "user_patch" } else { "nemesis" };
        let clicked = egui::Panel::left((id, "activity_bar"))
            .resizable(false)
            .exact_size(egui_widgets::ActivityBar::SIZE)
            .show(ui, |ui| {
                self.scm.pending.refresh(&work.workspace, work.edits);
                activity_bar(ui, i18n, &self.bar, self.scm.pending.changes.len())
            })
            .inner;
        if let Some(view) = clicked {
            self.toggle_view(view);
        }
        if self.bar.hidden {
            return asks;
        }

        egui::Panel::left((id, "side_bar")).resizable(true).default_size(320.0).show(ui, |ui| {
            let (_, _, title, _) = VIEWS.iter().find(|(view, ..)| *view == self.bar.view).copied().unwrap_or(VIEWS[0]);
            if self.bar.view != SideView::Changes {
                egui_widgets::side_bar_title(ui, i18n.t(title));
            }
            let has_behaviors = !work.workspace.behaviors.is_empty();
            match self.bar.view {
                SideView::Explorer => {
                    asks.extend(self.behaviors_section(ui, i18n, (work, sel), notify));
                    if has_behaviors {
                        asks.extend(self.object_list(ui, i18n, (work, sel, keys)));
                    }
                }
                SideView::Search if has_behaviors => asks.extend(self.search_panel(ui, i18n, work, sel)),
                SideView::Search => {}
                SideView::Changes => asks.extend(self.changes_view(ui, i18n, (work, sel, keys), notify)),
                SideView::Variables => asks.extend(self.variables(ui, i18n, work, sel.active)),
            }
        });
        asks
    }

    /// The EVENTS / VARIABLES view: of a behavior to pick here as well.
    fn variables(&mut self, ui: &mut egui::Ui, i18n: &I18nMap, work: &mut Work, active: usize) -> Vec<SideAsk> {
        let mut asks = Vec::new();
        if let Some(at) = behavior_select(ui, i18n, &work.workspace.behaviors, active) {
            asks.push(SideAsk::ShowBehavior(at));
        }

        let has_vars = work.workspace.behaviors.get(active).is_some_and(|behavior| !behavior.is_anim_data());
        if !has_vars {
            ui.weak(i18n.t(I18nKey::SideNoVariables));
            return asks;
        }

        let names = work.names(active);
        let behavior = &mut work.workspace.behaviors[active];
        let changed = self.vars.show(ui, i18n, behavior, &names, &self.explorer.filter);
        if !changed.is_empty() {
            asks.push(SideAsk::Reload); // the open object may be one of them
        }
        for id in changed {
            work.object_changed(&id);
        }
        asks
    }

    /// Shows `view`, or hides the side bar if it is the one shown, as a click on its icon does.
    fn toggle_view(&mut self, view: SideView) {
        if self.bar.toggle(view) && view == SideView::Search {
            self.search.focus();
        }
    }

    /// The keyboard goes to the object tree or list, as it shows, its filter kept (Vim `Space e`, or
    /// the palette). `editor`: the editor that had it.
    pub(in crate::patch) fn enter_tree(&mut self, ui: &egui::Ui, editor: egui::Id) {
        // The view shown, as VS Code's "Focus Side Bar": the tree or the list, or the changes. The
        // views without keys of their own (the search, the variables) give way to the explorer.
        self.explorer.tree_focus = true;
        if !matches!(self.bar.view, SideView::Explorer | SideView::Changes) || self.bar.hidden {
            self.bar.view = SideView::Explorer;
        }
        self.bar.hidden = false;
        self.bar.objects_closed = false;
        ui.memory_mut(|m| m.surrender_focus(editor));
    }

    /// `Ctrl+Shift+E` / `F` / `G` (key bindings), as in VS Code. A key shows its view, and does not
    /// hide it.
    ///
    /// As VS Code's, the view shown has the keyboard then (the search, its field).
    fn view_keys(&mut self, ui: &egui::Ui, keys: &mut Keys) {
        use keybinding::commands::{SIDE_BAR_CHANGES, SIDE_BAR_EXPLORER, SIDE_BAR_SEARCH};
        let views = [
            (SIDE_BAR_EXPLORER, SideView::Explorer),
            (SIDE_BAR_SEARCH, SideView::Search),
            (SIDE_BAR_CHANGES, SideView::Changes),
        ];
        for (command, view) in views {
            if !keys.take(command) {
                continue;
            }
            self.bar.show(view);
            if view == SideView::Search {
                self.search.focus();
                continue;
            }
            self.explorer.tree_focus = true;
            ui.memory_mut(|m| {
                if let Some(focused) = m.focused() {
                    m.surrender_focus(focused);
                }
            });
        }
    }
}

/// `Shift+Alt+C` / `Shift+Alt+R` (key bindings), as VS Code: of the selected object.
fn file_keys(keys: &mut Keys, sel: &Selection) -> Vec<SideAsk> {
    use keybinding::commands::{FILE_COPY_PATH, FILE_REVEAL};
    let (copy, reveal) = (keys.take(FILE_COPY_PATH), keys.take(FILE_REVEAL));
    let Some(selected) = sel.selected.as_deref() else { return Vec::new() };

    [(copy, FileAction::CopyPath), (reveal, FileAction::Reveal)]
        .into_iter()
        .filter(|(on, _)| *on)
        .map(|(_, action)| SideAsk::File(action, selected.to_owned()))
        .collect()
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
