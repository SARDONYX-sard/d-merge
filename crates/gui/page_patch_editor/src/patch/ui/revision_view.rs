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
//! The diff is the code editor, read only: the newer text with the older lines in it, as the
//! editor's inline diff colors them; `▲ ▼` and Vim's `]c` / `[c` go from change to change, Vim's
//! keys move in it, `Space c` / `:q` close it.

use egui_code_editor::{CodeEditor, CodeEditorState, EditorCommand, InlineDiff, Keymap, languages::Xml};
use gui_shared::{
    i18n::{I18nKey, I18nMap},
    settings::ui::editor::EditorMode,
};

use crate::patch::{
    PatchTool,
    editor::inline_diff,
    workspace::history::{ObjectKey, Origin},
};

/// A diff or a blame of an object, in a tab of the editor as VS Code opens a diff:
/// `GetUpModList (#2) ↔ GetUpModList (working) 🔒 ✖`. Read only.
#[derive(Debug)]
pub(in crate::patch) struct RevisionView {
    /// Its behavior: its tab is with that behavior's.
    pub(in crate::patch) behavior: usize,
    /// The object, e.g. `#0106`.
    pub(in crate::patch) id: String,
    /// What its tab says. e.g. `GetUpModList (#2) ↔ GetUpModList (working)`
    pub(in crate::patch) title: String,
    body: Body,
}

/// The views open, as tabs after the objects'.
#[derive(Debug, Default)]
pub(in crate::patch) struct Views {
    pub(in crate::patch) list: Vec<RevisionView>,
    /// The one shown in the editor's place. `None`: the object of the tab selected.
    pub(in crate::patch) shown: Option<usize>,
}

impl Views {
    /// `(title, shown)` of the tabs of `behavior`, in order, with their indices in [`Self::list`].
    pub(in crate::patch) fn tabs_of(&self, behavior: usize) -> Vec<(usize, &str, bool)> {
        let of = self.list.iter().enumerate().filter(|(_, view)| view.behavior == behavior);
        of.map(|(at, view)| (at, view.title.as_str(), self.shown == Some(at))).collect()
    }

    /// Closes view `at`; the one shown stays the one shown.
    pub(in crate::patch) fn close(&mut self, at: usize) {
        if at >= self.list.len() {
            return;
        }
        self.list.remove(at);
        self.shown = match self.shown {
            Some(shown) if shown == at => None,
            Some(shown) if shown > at => Some(shown - 1),
            shown => shown,
        };
    }
}

#[derive(Debug)]
enum Body {
    Compare(Box<Compare>),
    /// Each line with what last changed it.
    Blame(Vec<(String, String)>),
}

/// The newer text, read only, with the changes from the older one in it.
#[derive(Debug)]
struct Compare {
    text: String,
    diff: InlineDiff,
    state: CodeEditorState,
    /// Where the cursor is: `▲ ▼` go from there.
    cursor_line: usize,
    /// Just opened: the cursor goes to the first change, as VS Code's diff editor opens.
    to_first: bool,
}

impl PatchTool {
    /// The text of object `id` of behavior `behavior` at revision `number` (`None`: in the work now).
    /// An object no revision changed is the template's.
    pub(in crate::patch) fn object_text_at(&self, behavior: usize, id: &str, number: Option<u32>) -> String {
        let Some(of) = self.workspace.behaviors.get(behavior) else { return String::new() };
        let template = || of.template_object(id).map(|object| object.text.clone()).unwrap_or_default();
        match number {
            None => of.text(id).map_or_else(template, str::to_owned),
            Some(number) => {
                let key: ObjectKey = (of.meshes_path.clone(), id.to_owned());
                self.workspace.history.state_at(number).get(&key).map_or_else(template, |text| (*text).to_owned())
            }
        }
    }

    /// Opens the diff of object `id` from revision `from` to `to` (`None`: the work now).
    pub(in crate::patch) fn open_compare(&mut self, behavior: usize, id: &str, from: u32, to: Option<u32>) {
        let (old, new) = (self.object_text_at(behavior, id, Some(from)), self.object_text_at(behavior, id, to));
        let to_name = to.map_or_else(|| "working".to_owned(), |number| format!("#{number}"));
        let name = self.object_name(behavior, id);
        let title = format!("{name} (#{from}) ↔ {name} ({to_name})");
        let diff = inline_diff::changes(&old, &new);
        let compare = Compare { text: new, diff, state: CodeEditorState::default(), cursor_line: 0, to_first: true };
        self.show_in_editor(behavior, id, title, Body::Compare(Box::new(compare)));
    }

    /// The object's name as its tab says it. e.g. `GetUpModList`; its id if it has none.
    fn object_name(&mut self, behavior: usize, id: &str) -> String {
        if behavior != self.active {
            return id.to_owned();
        }
        let names = self.names();
        let name = names.graph.node_of(id).map_or("", |node| names.graph.short_name(node));
        if name.is_empty() { id.to_owned() } else { name.to_owned() }
    }

    /// Opens a tab of object `id` with `body`, or shows the one open with that title.
    fn show_in_editor(&mut self, behavior: usize, id: &str, title: String, body: Body) {
        if behavior != self.active {
            self.active = behavior;
            self.tabs.clear();
        }
        self.selected = Some(id.into());
        let views = &mut self.views;
        let open = views.list.iter().position(|view| view.behavior == behavior && view.title == title);
        let at = open.unwrap_or_else(|| {
            views.list.push(RevisionView { behavior, id: id.to_owned(), title, body });
            views.list.len() - 1
        });
        views.shown = Some(at);
    }

    /// Opens the blame of object `id` of behavior `behavior`, as the work has it now.
    pub(in crate::patch) fn open_blame(&mut self, behavior: usize, id: &str) {
        let Some(of) = self.workspace.behaviors.get(behavior) else { return };
        let key: ObjectKey = (of.meshes_path.clone(), id.to_owned());
        let now = of.text(id).unwrap_or_default();
        let template = of.template_object(id).map(|object| object.text.as_str());
        let history = &self.workspace.history;
        let origins = history.blame(&key, template, now);
        let label = |origin: Origin| match origin {
            Origin::Template => "template".to_owned(),
            Origin::Working => "working".to_owned(),
            Origin::Revision(number) => {
                let message = history.revision(number).map_or("", |revision| revision.message.as_str());
                format!("#{number} {message}")
            }
        };
        let lines = now.lines().zip(origins).map(|(line, origin)| (label(origin), line.to_owned())).collect();
        let title = format!("Blame {}", self.object_name(behavior, id));
        self.show_in_editor(behavior, id, title, Body::Blame(lines));
    }

    /// Draws the view shown, with the tabs, in the editor's place. Returns whether one is shown.
    pub(in crate::patch) fn views_ui(&mut self, ui: &mut egui::Ui, i18n: &I18nMap) -> bool {
        // Another object selected (the tree, the list): its editor, the view's tab stays.
        let selected = self.selected.as_deref();
        let shown = self.views.shown.filter(|&at| {
            self.views.list.get(at).is_some_and(|v| v.behavior == self.active && selected == Some(v.id.as_str()))
        });
        self.views.shown = shown;
        let Some(at) = shown else { return false };
        let keymap = if self.pane.mode == EditorMode::Vim { Keymap::Vim } else { Keymap::Normal };
        let names = self.names();
        let mut tab_event = None;
        let mut close = false;
        egui::CentralPanel::default().show(ui, |ui| {
            let mut go_to = None;
            let view_tabs = self.views.tabs_of(self.active);
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Body::Compare(compare) = &self.views.list[at].body {
                        go_to = super::object_editor::inline_diff_nav(ui, i18n, &compare.diff, compare.cursor_line);
                    }
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        let marks = super::object_editor::TabMarks { focused: true, problems: None };
                        let current = (self.active, "");
                        tab_event = super::object_editor::tab_bar(
                            ui,
                            i18n,
                            &self.tabs,
                            current,
                            &names.graph,
                            marks,
                            &view_tabs,
                        );
                    });
                });
            });
            let view = &mut self.views.list[at];
            let id = view.id.clone();
            match &mut view.body {
                Body::Compare(compare) => close |= compare_ui(ui, i18n, &id, compare, (keymap, go_to)),
                Body::Blame(lines) => {
                    egui::ScrollArea::both().id_salt(("nemesis_revision_view", &id)).auto_shrink(false).show(
                        ui,
                        |ui| {
                            blame_lines(ui, lines);
                        },
                    );
                }
            }
        });
        if close {
            self.views.close(at);
        }
        if tab_event.is_some() {
            let current = self.selected.as_deref().unwrap_or_default().to_owned();
            self.apply_tab_event(tab_event, &current);
        }
        true
    }
}

/// What the read only editor of a diff hosts: its changes, and closing it.
const COMPARE_COMMANDS: [EditorCommand; 3] =
    [EditorCommand::NextChange, EditorCommand::PreviousChange, EditorCommand::CloseTab];

/// The diff as the code editor: see the module docs. `go_to`: a line `▲ ▼` asked for. Returns
/// whether it is to be closed (Vim's `Space c`, `:q`).
fn compare_ui(
    ui: &mut egui::Ui,
    i18n: &I18nMap,
    id: &str,
    compare: &mut Compare,
    (keymap, go_to): (Keymap, Option<usize>),
) -> bool {
    let editor_id = egui_code_editor::editor_id(("nemesis_revision_view", id));
    if compare.diff.hunks.is_empty() {
        ui.weak(i18n.t(I18nKey::NemesisNoChange));
    }
    egui::Panel::bottom(("nemesis_revision_status", id)).show(ui, |ui| {
        egui_code_editor::status_line(ui, &compare.state, keymap, &compare.text);
    });
    let output = egui::ScrollArea::both()
        .id_salt(("nemesis_revision_scroll", id))
        .auto_shrink(false)
        .show(ui, |ui| {
            CodeEditor::new(("nemesis_revision_view", id), &mut compare.text)
                .language(&Xml)
                .read_only(true)
                .keymap(keymap)
                .inline_diff(Some(&compare.diff))
                .host_commands(&COMPARE_COMMANDS)
                .show(ui, &mut compare.state)
        })
        .inner;
    // Taken out before the text is read again: the output borrows it.
    let (cursor, command) = (output.cursor, output.command);
    if let Some(cursor) = cursor {
        compare.cursor_line = cursor.line;
    }
    let step = match command {
        Some(EditorCommand::NextChange) => inline_diff::step(&compare.diff, compare.cursor_line, true),
        Some(EditorCommand::PreviousChange) => inline_diff::step(&compare.diff, compare.cursor_line, false),
        _ => None,
    };
    let first = std::mem::take(&mut compare.to_first).then(|| inline_diff::first(&compare.diff)).flatten();
    if let Some(line) = go_to.or(step).or(first) {
        let at = compare.text.split('\n').take(line).map(|line| line.chars().count() + 1).sum();
        egui_code_editor::set_cursor(ui.ctx(), editor_id, at);
        ui.memory_mut(|m| m.request_focus(editor_id));
    }
    command == Some(EditorCommand::CloseTab)
}

/// `#2 fix speed   <hkparam ...>`: the revision in a column of its own, as GitLens' blame.
fn blame_lines(ui: &mut egui::Ui, lines: &[(String, String)]) {
    egui::Grid::new("nemesis_blame").striped(true).show(ui, |ui| {
        let mut last = "";
        for (origin, line) in lines {
            // Only where it changes: a run of lines of one revision reads as a block.
            let shown = if origin == last { String::new() } else { origin.clone() };
            ui.label(egui::RichText::new(shown).small().weak());
            ui.label(egui::RichText::new(line).monospace());
            ui.end_row();
            last = origin;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Closing a tab keeps the one shown, wherever it moved.
    #[test]
    fn a_view_closed_keeps_the_one_shown() {
        let view = |title: &str| RevisionView {
            behavior: 0,
            id: "#1".to_owned(),
            title: title.to_owned(),
            body: Body::Blame(Vec::new()),
        };
        let mut views = Views { list: vec![view("a"), view("b"), view("c")], shown: Some(2) };
        views.close(0);
        assert_eq!((views.list.len(), views.shown), (2, Some(1)));
        views.close(1);
        assert_eq!(views.shown, None, "the one shown closed");
        assert_eq!(views.tabs_of(0), [(0, "b", false)]);
    }

    /// The diff is the newer text with the older lines in it, as the editor's inline diff has them.
    #[test]
    fn a_compare_is_the_newer_text_with_its_changes() {
        let diff = inline_diff::changes("a\nb\nd\n", "a\nc\nd\ne\n");
        assert_eq!(diff.hunks.len(), 2);
        assert_eq!(diff.hunks[0].old_lines, ["b"]);
        assert_eq!(diff.hunks[0].new_lines, 1..2);
        assert_eq!(inline_diff::step(&diff, 1, true), Some(3), "`▼` / `]c`: to the next change");
        assert_eq!(inline_diff::first(&diff), Some(1), "where it opens");
    }
}
