//! The views of the history, in the editor's place: an object compared between two revisions (as git's `diff`), and its
//! lines with the revision that last changed each (as GitLens' blame). The history is
//! [`crate::patch::workspace::history`]; the Changes view, [`super::side_bar`], opens these.
//!
//! ```text
//! ┌ #0106: #2 -> working        ▲ ▼ 1/2  ✖ ┐   ┌ Blame #0106 ─────────────────────────────┐
//! │   <hkobject name="#0106" ...>          │   │ template       <hkobject name="#0106"..>  │
//! │ - <hkparam name="enable">true<..>      │   │ #2 fix speed   <hkparam name="enable">..  │
//! │ + <hkparam name="enable">false<..>     │   │ working        <hkparam name="userData".. │
//! │ NORMAL  Ln 2, Col 1        Vim ▾       │   └───────────────────────────────────────────┘
//! └────────────────────────────────────────┘
//! ```
//! Each is an [`egui_code_editor::DiffView`] (the code editor, read only) in a tab after the
//! objects'; `Space c` / `:q` close it.

use egui_code_editor::{DiffView, EditorCommand, Keymap, languages::Xml};
use egui_widgets::preview_tabs::PreviewTabs;
use gui_shared::i18n::{I18nKey, I18nMap};

use crate::patch::{
    editor_area::EditorArea,
    tool::{file::FileAction, keys::Keys, selection::Selection, work_state::Work},
    workspace::{
        Workspace,
        history::{ObjectKey, Origin},
    },
};

/// A diff or a blame of an object, in a tab of the editor as VS Code opens a diff:
/// `GetUpModList (#2) ↔ GetUpModList (working) 🔒 ✖`. Read only. Its tab's group is its behavior.
#[derive(Debug)]
pub(super) struct RevisionView {
    /// The object, e.g. `#0106`.
    pub(super) id: String,
    /// What its tab says. e.g. `GetUpModList (#2) ↔ GetUpModList (working)`
    pub(super) title: String,
    body: Box<DiffView>,
}

/// A tab of a view, for the tab bar. e.g. `{ at: 0, title: "Blame Root", shown: true, pinned: false }`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ViewTab<'a> {
    /// Its index in [`Views::list`].
    pub(super) at: usize,
    pub(super) title: &'a str,
    /// Its object, e.g. `#0106`.
    pub(super) id: &'a str,
    pub(super) shown: bool,
    pub(super) pinned: bool,
}

/// The views open, as tabs after the objects'. The one shown in the editor's place is
/// `list.active`; `None`: the object of the tab selected.
#[derive(Debug, Default)]
pub(super) struct Views {
    pub(super) list: PreviewTabs<RevisionView>,
}

impl Views {
    /// The tabs of `behavior`, in order: see [`ViewTab`].
    pub(super) fn tabs_of(&self, behavior: usize) -> Vec<ViewTab<'_>> {
        let of = self.list.of(behavior);
        of.map(|(at, view)| ViewTab {
            at,
            title: &view.title,
            id: &view.id,
            shown: self.list.active == Some(at),
            pinned: view.pinned,
        })
        .collect()
    }

    /// Shows view `at`, its editor with the keyboard.
    pub(super) fn show(&mut self, at: usize) {
        if let Some(view) = self.list.get_mut(at) {
            view.body.focus();
            self.list.active = Some(at);
        }
    }

    /// Keeps only `at` of the views of its behavior, and shows it.
    pub(super) fn close_others(&mut self, at: usize) {
        self.list.active = self.list.close_others(at);
    }
}

/// The text of object `id` of behavior `behavior` at revision `number` (`None`: in the work now).
/// An object no revision changed is the template's.
fn object_text_at(workspace: &Workspace, behavior: usize, id: &str, number: Option<u32>) -> String {
    let Some(of) = workspace.behaviors.get(behavior) else { return String::new() };
    let template = || of.template_object(id).map(|object| object.text.clone()).unwrap_or_default();
    match number {
        None => of.text(id).map_or_else(template, str::to_owned),
        Some(number) => {
            let key: ObjectKey = (of.meshes_path.clone(), id.to_owned());
            workspace.history.state_at(number).get(&key).map_or_else(template, |text| (*text).to_owned())
        }
    }
}

/// Object `id` of behavior `behavior` as the work has it now, and the revision that last changed
/// each of its lines, as GitLens says it: `3 days ago  #2 fix the speed`.
fn blame_of(workspace: &Workspace, behavior: usize, id: &str) -> Option<(String, Vec<String>)> {
    let of = workspace.behaviors.get(behavior)?;
    let key: ObjectKey = (of.meshes_path.clone(), id.to_owned());
    let now = of.text(id).unwrap_or_default();
    let template = of.template_object(id).map(|object| object.text.as_str());

    let history = &workspace.history;
    let at = crate::patch::workspace::history::now();
    let label = |origin: Origin| match origin {
        Origin::Template => "template".to_owned(),
        Origin::Working => "working (not committed)".to_owned(),
        Origin::Revision(number) => history.revision(number).map_or_else(
            || format!("#{number}"),
            |revision| format!("{}  #{number} {}", stdx::time::ago(at, revision.time), revision.message),
        ),
    };
    let notes = history.blame(&key, template, now).into_iter().map(label).collect();
    Some((now.to_owned(), notes))
}

/// What the view shown asked for.
#[derive(Debug, Default)]
struct ShownOutput {
    tab_event: Option<crate::patch::TabEvent>,
    /// Its editor's command (Vim's `Space c`, `gt`, ...).
    command: Option<EditorCommand>,
    /// Its editor has the keyboard.
    has_focus: bool,
}

impl Views {
    /// Draws view `at` in the editor's place, the tab bar above it (the object tabs of `active`,
    /// `tabs`, with their names from `graph`).
    fn show_at(
        &mut self,
        ui: &mut egui::Ui,
        i18n: &I18nMap,
        at: usize,
        (tabs, graph, active): (&crate::patch::editor::tabs::EditorTabs, &crate::patch::tree::Graph, usize),
        keymap: Keymap,
    ) -> ShownOutput {
        let mut output = ShownOutput::default();
        egui::CentralPanel::default().show(ui, |ui| {
            let mut go_to = None;
            let view_tabs = self.tabs_of(active);
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    go_to = self.list[at].body.nav(ui, i18n.t(I18nKey::NemesisJumpDiffHover));
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        let marks = super::tab_bar::TabMarks { focused: true, problems: None };
                        output.tab_event =
                            super::tab_bar::tab_bar(ui, i18n, tabs, (active, ""), graph, marks, &view_tabs);
                    });
                });
            });

            let view = &mut *self.list[at];
            if let Some(line) = go_to {
                view.body.go_to(line);
            }
            if view.body.is_unchanged() {
                ui.weak(i18n.t(I18nKey::NemesisNoChange));
            }
            let shown = view.body.show(ui, ("nemesis_revision_view", view.id.as_str()), &Xml, keymap);
            output.command = shown.command;
            output.has_focus = shown.has_focus;
        });
        output
    }
}

impl EditorArea {
    /// Opens the diff of object `id` from revision `from` to `to` (`None`: the work now).
    pub(in crate::patch) fn open_compare(
        &mut self,
        (work, sel): (&mut Work, &mut Selection),
        (behavior, id): (usize, &str),
        (from, to): (u32, Option<u32>),
    ) {
        let old = object_text_at(&work.workspace, behavior, id, Some(from));
        let new = object_text_at(&work.workspace, behavior, id, to);
        let to_name = to.map_or_else(|| "working".to_owned(), |number| format!("#{number}"));
        let name = object_name(work, sel.active, (behavior, id));
        let title = format!("{name} (#{from}) ↔ {name} ({to_name})");
        self.show_view(sel, (behavior, id), title, Box::new(DiffView::diff(&old, new)));
    }

    /// Opens the blame of object `id` of behavior `behavior`, as the work has it now.
    pub(in crate::patch) fn open_blame(
        &mut self,
        (work, sel): (&mut Work, &mut Selection),
        (behavior, id): (usize, &str),
    ) {
        let Some((text, notes)) = blame_of(&work.workspace, behavior, id) else { return };
        let title = format!("Blame {}", object_name(work, sel.active, (behavior, id)));
        self.show_view(sel, (behavior, id), title, Box::new(DiffView::blame(text, notes)));
    }

    /// The object `(behavior, id)` itself, not a diff of it.
    pub(in crate::patch) fn open_object(&mut self, sel: &mut Selection, (behavior, id): (usize, &str)) {
        self.views.list.active = None;
        self.select_object(sel, (behavior, id));
    }

    /// Shows object `id` of `behavior`. Another behavior's: the tabs of this one close.
    fn select_object(&mut self, sel: &mut Selection, (behavior, id): (usize, &str)) {
        if behavior != sel.active {
            sel.active = behavior;
            self.tabs.clear();
        }
        sel.selected = Some(id.into());
    }

    /// Opens a tab of object `id` with `body`, or shows the one open with that title.
    fn show_view(&mut self, sel: &mut Selection, (behavior, id): (usize, &str), title: String, body: Box<DiffView>) {
        self.select_object(sel, (behavior, id));
        let at = match self.views.list.find(behavior, |known| known.title == title) {
            Some(at) => at,
            None => self.views.list.open(behavior, RevisionView { id: id.to_owned(), title, body }, |_| false, None),
        };
        self.views.show(at);
    }

    /// Draws the view shown, with the tabs, in the editor's place. Returns `None` if none is shown.
    pub(super) fn views_ui(
        &mut self,
        ui: &mut egui::Ui,
        i18n: &I18nMap,
        (work, sel, keys): (&mut Work, &mut Selection, &mut Keys),
    ) -> Option<ViewShown> {
        // Another object selected (the tree, the list): its editor, the view's tab stays.
        let selected = sel.selected.as_deref();
        let active = sel.active;
        let shown = self.views.list.active.filter(|&at| {
            self.views.list.get(at).is_some_and(|view| view.group == active && selected == Some(view.id.as_str()))
        });
        self.views.list.active = shown;
        let at = shown?;

        let keymap = Keymap::from_vim(self.pane.mode.is_vim());
        let names = work.names(active);
        let output = self.views.show_at(ui, i18n, at, (&self.tabs, &names.graph, active), keymap);
        // What the key bindings see (`editorFocus`): this editor, as the object's is seen.
        self.pane.had_focus = output.has_focus;

        // The keys of the tabs as on an object's: the tab bar's bindings, Vim's `gt` / `Space c`.
        let command = output.command;
        let close = keys.take(keybinding::commands::TABS_CLOSE) || command == Some(EditorCommand::CloseTab);
        let next = keys.take(keybinding::commands::TABS_NEXT) || command == Some(EditorCommand::NextTab);
        let previous = keys.take(keybinding::commands::TABS_PREVIOUS) || command == Some(EditorCommand::PreviousTab);
        let current = sel.selected.as_deref().unwrap_or_default().to_owned();
        if close {
            self.views.list.close(at);
        } else if next || previous {
            self.step_tab(sel, &current, next);
        }
        Some(ViewShown { file: self.apply_tab_event(sel, output.tab_event, &current) })
    }
}

/// A diff or a blame was shown, see [`EditorArea::views_ui`].
#[derive(Debug)]
pub(super) struct ViewShown {
    /// What a tab asked of an object's file.
    pub(super) file: Option<(FileAction, String)>,
}

/// The name of object `(behavior, id)` as its tab says it, e.g. `GetUpModList`; its id if it has none
/// (or is not of the behavior `active`, whose names are known).
fn object_name(work: &mut Work, active: usize, (behavior, id): (usize, &str)) -> String {
    if behavior != active {
        return id.to_owned();
    }
    let names = work.names(active);
    let name = names.graph.node_of(id).map_or("", |node| names.graph.short_name(node));
    if name.is_empty() { id.to_owned() } else { name.to_owned() }
}

#[cfg(test)]
mod tests {
    use egui_widgets::preview_tabs::Tab;

    use super::*;

    fn blank_view(title: &str) -> Tab<RevisionView> {
        let item = RevisionView { id: "#1".to_owned(), title: title.to_owned(), body: Box::default() };
        Tab { group: 0, pinned: true, item }
    }

    #[test]
    fn the_next_tab_goes_through_the_views_too() {
        let (mut area, mut sel) = (EditorArea::default(), Selection::default());
        for id in ["#a", "#b"] {
            area.tabs.restore(0, id, true);
        }
        area.tabs.restore(1, "#x", true);
        let mut view = blank_view("Blame a");
        view.id = "#a".to_owned();
        area.views.list.push(view);
        sel.selected = Some("#b".into());

        area.step_tab(&mut sel, "#b", true);
        assert_eq!((area.views.list.active, sel.selected.as_deref()), (Some(0), Some("#a")), "the view after `#b`");
        area.step_tab(&mut sel, "#a", true);
        assert_eq!((area.views.list.active, sel.selected.as_deref()), (None, Some("#a")), "around the end");
        area.step_tab(&mut sel, "#a", false);
        assert_eq!(area.views.list.active, Some(0), "back around the start");
    }

    /// The views' tabs are `PreviewTabs`: `Close Others` shows the one kept.
    #[test]
    fn closing_the_others_shows_the_one_kept() {
        let mut views = Views::default();
        for title in ["a", "b", "c"] {
            views.list.push(blank_view(title));
        }
        views.close_others(1);
        assert_eq!((views.list.len(), views.list[0].title.as_str(), views.list.active), (1, "b", Some(0)));
        assert_eq!(views.tabs_of(0), [ViewTab { at: 0, title: "b", id: "#1", shown: true, pinned: true }]);
    }

    /// The diff is the newer text with the older lines in it, as the editor's inline diff has them.
    #[test]
    fn a_compare_is_the_newer_text_with_its_changes() {
        let view = DiffView::diff("a\nb\nd\n", "a\nc\nd\ne\n".to_owned());
        let diff = view.changes();
        assert_eq!(diff.hunks.len(), 2);
        assert_eq!(diff.hunks[0].old_lines, ["b"]);
        assert_eq!(diff.hunks[0].new_lines, 1..2);
        assert_eq!(diff.step(1, true), Some(3), "`▼` / `]c`: to the next change");
        assert_eq!(diff.first_line(), Some(1), "where it opens");
    }
}
