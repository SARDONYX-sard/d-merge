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
use gui_shared::{
    i18n::{I18nKey, I18nMap},
    settings::ui::editor::EditorMode,
};

use crate::patch::{
    PatchTool,
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
    /// `false`: the preview tab of the views, which the next one opened replaces (italic), as an
    /// object's. A double click on it keeps it.
    pub(in crate::patch) pinned: bool,
    body: Box<DiffView>,
}

/// A tab of a view, for the tab bar. e.g. `{ at: 0, title: "Blame Root", shown: true, pinned: false }`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::patch) struct ViewTab<'a> {
    /// Its index in [`Views::list`].
    pub(in crate::patch) at: usize,
    pub(in crate::patch) title: &'a str,
    /// Its object, e.g. `#0106`.
    pub(in crate::patch) id: &'a str,
    pub(in crate::patch) shown: bool,
    pub(in crate::patch) pinned: bool,
}

/// The views open, as tabs after the objects'.
#[derive(Debug, Default)]
pub(in crate::patch) struct Views {
    pub(in crate::patch) list: Vec<RevisionView>,
    /// The one shown in the editor's place. `None`: the object of the tab selected.
    pub(in crate::patch) shown: Option<usize>,
}

impl Views {
    /// The tabs of `behavior`, in order: see [`ViewTab`].
    pub(in crate::patch) fn tabs_of(&self, behavior: usize) -> Vec<ViewTab<'_>> {
        let of = self.list.iter().enumerate().filter(|(_, view)| view.behavior == behavior);
        of.map(|(at, view)| ViewTab {
            at,
            title: &view.title,
            id: &view.id,
            shown: self.shown == Some(at),
            pinned: view.pinned,
        })
        .collect()
    }

    /// Opens `view` in a tab: the one of the same title if open, else the preview tab of its behavior
    /// (replaced), else a new one. Returns its index.
    fn open(&mut self, view: RevisionView) -> usize {
        let same = |known: &RevisionView| known.behavior == view.behavior && known.title == view.title;
        if let Some(at) = self.list.iter().position(same) {
            return at;
        }
        let preview = self.list.iter().position(|known| known.behavior == view.behavior && !known.pinned);
        if let Some(at) = preview {
            self.list[at] = view;
            return at;
        }
        self.list.push(view);
        self.list.len() - 1
    }

    /// Shows view `at`, its editor with the keyboard.
    pub(in crate::patch) fn show(&mut self, at: usize) {
        if let Some(view) = self.list.get_mut(at) {
            view.body.focus();
            self.shown = Some(at);
        }
    }

    /// Keeps only `at` of the views of its behavior.
    pub(in crate::patch) fn close_others(&mut self, at: usize) {
        let Some(behavior) = self.list.get(at).map(|view| view.behavior) else { return };
        let mut index = 0;
        self.list.retain(|view| {
            let keep = view.behavior != behavior || index == at;
            index += 1;
            keep
        });
        let kept = self.list.iter().position(|view| view.behavior == behavior);
        self.shown = kept;
    }

    /// Closes the views of `at`'s behavior to its right.
    pub(in crate::patch) fn close_to_the_right(&mut self, at: usize) {
        let Some(behavior) = self.list.get(at).map(|view| view.behavior) else { return };
        // The last first: what is closed does not move those left to close.
        for index in (at + 1..self.list.len()).rev() {
            if self.list[index].behavior == behavior {
                self.close(index);
            }
        }
    }

    /// Closes every view of `behavior`.
    pub(in crate::patch) fn close_all(&mut self, behavior: usize) {
        for index in (0..self.list.len()).rev() {
            if self.list[index].behavior == behavior {
                self.close(index);
            }
        }
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
        self.show_in_editor(behavior, id, title, Box::new(DiffView::diff(&old, new)));
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
    fn show_in_editor(&mut self, behavior: usize, id: &str, title: String, body: Box<DiffView>) {
        if behavior != self.active {
            self.active = behavior;
            self.tabs.clear();
        }
        self.selected = Some(id.into());
        let at = self.views.open(RevisionView { behavior, id: id.to_owned(), title, pinned: false, body });
        self.views.show(at);
    }

    /// Opens the blame of object `id` of behavior `behavior`, as the work has it now.
    pub(in crate::patch) fn open_blame(&mut self, behavior: usize, id: &str) {
        let Some(of) = self.workspace.behaviors.get(behavior) else { return };
        let key: ObjectKey = (of.meshes_path.clone(), id.to_owned());
        let now = of.text(id).unwrap_or_default();
        let template = of.template_object(id).map(|object| object.text.as_str());
        let history = &self.workspace.history;
        let origins = history.blame(&key, template, now);
        let at = crate::patch::workspace::history::now();
        let label = |origin: Origin| match origin {
            Origin::Template => "template".to_owned(),
            Origin::Working => "working (not committed)".to_owned(),
            Origin::Revision(number) => history.revision(number).map_or_else(
                || format!("#{number}"),
                |revision| format!("{}  #{number} {}", ago(at, revision.time), revision.message),
            ),
        };
        let notes = origins.into_iter().map(label).collect();
        let text = now.to_owned();
        let title = format!("Blame {}", self.object_name(behavior, id));
        self.show_in_editor(behavior, id, title, Box::new(DiffView::blame(text, notes)));
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
        let mut command = None;
        egui::CentralPanel::default().show(ui, |ui| {
            let mut go_to = None;
            let view_tabs = self.views.tabs_of(self.active);
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    go_to = self.views.list[at].body.nav(ui, i18n.t(I18nKey::NemesisJumpDiffHover));
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
            if let Some(line) = go_to {
                view.body.go_to(line);
            }
            if view.body.is_unchanged() {
                ui.weak(i18n.t(I18nKey::NemesisNoChange));
            }
            let output = view.body.show(ui, ("nemesis_revision_view", view.id.as_str()), &Xml, keymap);
            command = output.command;
            // What the key bindings see (`editorFocus`): this editor, as the object's is seen.
            self.pane.had_focus = output.has_focus;
        });
        // The keys of the tabs as on an object's: the tab bar's bindings, Vim's `gt` / `Space c`.
        let close = self.key_command(keybinding::commands::TABS_CLOSE) || command == Some(EditorCommand::CloseTab);
        let next = self.key_command(keybinding::commands::TABS_NEXT) || command == Some(EditorCommand::NextTab);
        let previous =
            self.key_command(keybinding::commands::TABS_PREVIOUS) || command == Some(EditorCommand::PreviousTab);
        if close {
            self.views.close(at);
        } else if next || previous {
            let current = self.selected.as_deref().unwrap_or_default().to_owned();
            self.step_tab(&current, next);
        }
        if tab_event.is_some() {
            let current = self.selected.as_deref().unwrap_or_default().to_owned();
            self.apply_tab_event(tab_event, &current);
        }
        true
    }
}

/// How long ago `time` was at `now` (seconds since the Unix epoch), as GitLens says it.
///
/// e.g. 90 s -> `1 minute ago`, 3 days -> `3 days ago`
fn ago(now: u64, time: u64) -> String {
    const STEPS: [(u64, &str); 6] =
        [(60, "minute"), (3600, "hour"), (86_400, "day"), (2_592_000, "month"), (31_536_000, "year"), (u64::MAX, "")];
    let seconds = now.saturating_sub(time);
    if seconds < 60 {
        return "just now".to_owned();
    }
    let (unit, name) = STEPS.windows(2).find(|pair| seconds < pair[1].0).map_or(STEPS[4], |pair| pair[0]);
    let count = seconds / unit;
    format!("{count} {name}{} ago", if count == 1 { "" } else { "s" })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blank_view(title: &str) -> RevisionView {
        RevisionView { behavior: 0, id: "#1".to_owned(), title: title.to_owned(), pinned: true, body: Box::default() }
    }

    #[test]
    fn how_long_ago_as_gitlens_says_it() {
        assert_eq!(ago(100, 90), "just now");
        assert_eq!(ago(200, 100), "1 minute ago");
        assert_eq!(ago(3 * 86_400 + 5, 5), "3 days ago");
        assert_eq!(ago(2 * 2_592_000, 0), "2 months ago");
        assert_eq!(ago(31_536_000, 0), "1 year ago");
    }

    /// The tabs go around the objects' then the views', within the behavior, as the keys go.
    #[test]
    fn the_next_tab_goes_through_the_views_too() {
        let mut tool = PatchTool::default();
        for id in ["#a", "#b"] {
            tool.tabs.restore(0, id, true);
        }
        tool.tabs.restore(1, "#x", true);
        let mut view = blank_view("Blame a");
        view.id = "#a".to_owned();
        tool.views.list.push(view);
        tool.selected = Some("#b".into());

        tool.step_tab("#b", true);
        assert_eq!((tool.views.shown, tool.selected.as_deref()), (Some(0), Some("#a")), "the view after `#b`");
        tool.step_tab("#a", true);
        assert_eq!((tool.views.shown, tool.selected.as_deref()), (None, Some("#a")), "around the end");
        tool.step_tab("#a", false);
        assert_eq!(tool.views.shown, Some(0), "back around the start");
    }

    /// Closing a tab keeps the one shown, wherever it moved.
    #[test]
    fn a_view_closed_keeps_the_one_shown() {
        let view = blank_view;
        let mut views = Views { list: vec![view("a"), view("b"), view("c")], shown: Some(2) };
        views.close(0);
        assert_eq!((views.list.len(), views.shown), (2, Some(1)));
        views.close(1);
        assert_eq!(views.shown, None, "the one shown closed");

        let mut more = Views { list: vec![view("a"), view("b"), view("c")], shown: None };
        more.close_to_the_right(0);
        assert_eq!(more.list.len(), 1, "right of `a`");
        more.list.extend([view("b"), view("c")]);
        more.close_others(1);
        assert_eq!((more.list.len(), more.list[0].title.as_str(), more.shown), (1, "b", Some(0)));
        more.close_all(0);
        assert!(more.list.is_empty());
        assert_eq!(views.tabs_of(0), [ViewTab { at: 0, title: "b", id: "#1", shown: false, pinned: true }]);
        // A new view takes the place of the preview one, as an object's tab does.
        views.list[0].pinned = false;
        let at = views.open(view("d"));
        assert_eq!((at, views.list.len(), views.list[0].title.as_str()), (0, 1, "d"));
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
