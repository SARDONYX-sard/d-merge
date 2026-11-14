//! The objects open in the editor's tabs, as VS Code keeps them ([`egui_widgets::preview_tabs`]): a
//! click in the tree or the list opens the preview tab, a tab stays once kept open (edited,
//! double-clicked, a right-click menu), and a new one goes right after the current one.
//!
//! Each tab keeps its editor state (undo history, Vim mode and registers) while another is shown.
//! Tabs belong to a behavior (their group): only the active behavior's are shown.

use egui_code_editor::CodeEditorState;
use egui_widgets::preview_tabs::{PreviewTabs, Tab};

/// One open object.
#[derive(Debug)]
pub(in crate::patch) struct ObjectTab {
    /// e.g. `#0106`
    pub(in crate::patch) id: String,
    /// The editor state while the tab is not shown. `None`: it is in use, or never was.
    editor: Option<CodeEditorState>,
}

/// An object's tab: its group is its behavior's index in the workspace.
type EditorTab = Tab<ObjectTab>;

/// See the module docs. Tabs are named by behavior and object id.
#[derive(Debug, Default)]
pub(in crate::patch) struct EditorTabs {
    tabs: PreviewTabs<ObjectTab>,
}

impl EditorTabs {
    fn position(&self, behavior: usize, id: &str) -> Option<usize> {
        self.tabs.find(behavior, |tab| tab.id == id)
    }

    /// Every tab, to write with the work.
    pub(in crate::patch) fn all(&self) -> impl Iterator<Item = &EditorTab> {
        self.tabs.iter()
    }

    /// A tab from when the work was last open. Its editor state starts anew.
    pub(in crate::patch) fn restore(&mut self, behavior: usize, id: &str, pinned: bool) {
        if self.position(behavior, id).is_none() {
            let item = ObjectTab { id: id.to_owned(), editor: None };
            self.tabs.push(Tab { group: behavior, pinned, item });
        }
    }

    /// The tabs of `behavior`, in order.
    pub(in crate::patch) fn of(&self, behavior: usize) -> impl DoubleEndedIterator<Item = &EditorTab> {
        self.tabs.of(behavior).map(|(_, tab)| tab)
    }

    /// The last tab of `behavior`: what to show when going back to it.
    pub(in crate::patch) fn last_of(&self, behavior: usize) -> Option<&str> {
        self.of(behavior).next_back().map(|tab| tab.id.as_str())
    }

    /// Shows `id` in a tab: its own if it has one, else the preview tab, else a new one right after
    /// `current` (the tab being left), or at the end.
    pub(in crate::patch) fn open(&mut self, behavior: usize, id: &str, current: Option<&str>) {
        if self.position(behavior, id).is_some() {
            return;
        }
        let after = current.and_then(|current| self.position(behavior, current));
        let item = ObjectTab { id: id.to_owned(), editor: None };
        self.tabs.open(behavior, item, |_| false, after);
    }

    /// The tab stays open from now on.
    pub(in crate::patch) fn pin(&mut self, behavior: usize, id: &str) {
        if let Some(at) = self.position(behavior, id) {
            self.tabs.pin(at);
        }
    }

    /// Closes a tab. Returns the tab that takes its place: the next one, or else the previous one.
    pub(in crate::patch) fn close(&mut self, behavior: usize, id: &str) -> Option<String> {
        let at = self.position(behavior, id)?;
        self.tabs.close(at).map(|at| self.tabs[at].id.clone())
    }

    /// Closes every tab of `behavior` but `id`.
    pub(in crate::patch) fn close_others(&mut self, behavior: usize, id: &str) {
        match self.position(behavior, id) {
            Some(at) => _ = self.tabs.close_others(at),
            None => self.tabs.close_all(behavior),
        }
    }

    /// Closes the tabs of `behavior` right of `id`.
    pub(in crate::patch) fn close_to_the_right(&mut self, behavior: usize, id: &str) {
        if let Some(at) = self.position(behavior, id) {
            self.tabs.close_to_the_right(at);
        }
    }

    /// Closes every tab of `behavior`.
    pub(in crate::patch) fn close_all(&mut self, behavior: usize) {
        self.tabs.close_all(behavior);
    }

    /// The workspace was replaced, or a behavior removed: the indices no longer hold.
    pub(in crate::patch) fn clear(&mut self) {
        self.tabs.clear();
    }

    /// Keeps the editor state of a tab that is no longer shown. Gone with the tab, if it has been
    /// replaced or closed.
    pub(in crate::patch) fn store_state(&mut self, behavior: usize, id: &str, editor: CodeEditorState) {
        if let Some(at) = self.position(behavior, id) {
            self.tabs[at].editor = Some(editor);
        }
    }

    /// The editor state of the tab about to be shown: as it was left, or a new one.
    pub(in crate::patch) fn take_state(&mut self, behavior: usize, id: &str) -> CodeEditorState {
        self.position(behavior, id).and_then(|at| self.tabs[at].editor.take()).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(tabs: &EditorTabs, behavior: usize) -> Vec<&str> {
        tabs.of(behavior).map(|tab| tab.id.as_str()).collect()
    }

    /// By id, as the editor names them; the rest is `PreviewTabs`'.
    #[test]
    fn tabs_are_named_by_behavior_and_id() {
        let mut tabs = EditorTabs::default();
        tabs.open(0, "#a", None);
        tabs.pin(0, "#a");
        tabs.open(0, "#b", Some("#a"));
        tabs.open(1, "#a", None);
        assert_eq!((ids(&tabs, 0), ids(&tabs, 1)), (vec!["#a", "#b"], vec!["#a"]));
        assert_eq!(tabs.last_of(0), Some("#b"));
        assert_eq!(tabs.close(0, "#b").as_deref(), Some("#a"));
        assert_eq!(tabs.close(0, "#gone"), None);

        tabs.store_state(0, "#a", CodeEditorState::default());
        assert!(tabs.tabs[0].editor.is_some());
        tabs.take_state(0, "#a");
        assert!(tabs.tabs[0].editor.is_none());
    }
}
