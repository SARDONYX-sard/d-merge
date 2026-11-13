//! The objects open in the editor's tabs, like VS Code:
//!
//! ```text
//! [ #0106 GetUpModList ✖ ] [ ● #0200 Root ✖ ] [ #0300 hkbClipGenerator ✖ ]
//!                                               ^ italic: a preview tab
//! ```
//! A single click in the tree or list opens an object in the **preview tab**, which the next one
//! replaces, so going through the tree does not open hundreds of tabs. A tab stays once it is kept
//! open: edited, double-clicked (in the tree, the list or on the tab), or by a right-click menu.
//! A new tab goes right after the current one.
//!
//! Each tab keeps its editor state (undo history, Vim mode and registers) while another is shown.
//! Tabs belong to a behavior: only the active behavior's are shown.

use egui_code_editor::CodeEditorState;

/// One open object.
#[derive(Debug)]
pub(in crate::patch) struct EditorTab {
    /// Index of its behavior in the workspace.
    pub(in crate::patch) behavior: usize,
    /// e.g. `#0106`
    pub(in crate::patch) id: String,
    /// `false`: the preview tab, which the next opened object replaces.
    pub(in crate::patch) pinned: bool,
    /// The editor state while the tab is not shown. `None`: it is in use, or never was.
    editor: Option<CodeEditorState>,
}

/// See the module docs.
#[derive(Debug, Default)]
pub(in crate::patch) struct EditorTabs {
    tabs: Vec<EditorTab>,
}

impl EditorTabs {
    fn position(&self, behavior: usize, id: &str) -> Option<usize> {
        self.tabs.iter().position(|tab| tab.behavior == behavior && tab.id == id)
    }

    /// Every tab, to write with the work.
    pub(in crate::patch) fn all(&self) -> impl Iterator<Item = &EditorTab> {
        self.tabs.iter()
    }

    /// A tab from when the work was last open. Its editor state starts anew.
    pub(in crate::patch) fn restore(&mut self, behavior: usize, id: &str, pinned: bool) {
        if self.position(behavior, id).is_none() {
            self.tabs.push(EditorTab { behavior, id: id.to_owned(), pinned, editor: None });
        }
    }

    /// The tabs of `behavior`, in order.
    pub(in crate::patch) fn of(&self, behavior: usize) -> impl Iterator<Item = &EditorTab> {
        self.tabs.iter().filter(move |tab| tab.behavior == behavior)
    }

    /// The last tab of `behavior`: what to show when going back to it.
    pub(in crate::patch) fn last_of(&self, behavior: usize) -> Option<&str> {
        self.of(behavior).last().map(|tab| tab.id.as_str())
    }

    /// Shows `id` in a tab: its own if it has one, else the preview tab, else a new one right after
    /// `current` (the tab being left), or at the end.
    pub(in crate::patch) fn open(&mut self, behavior: usize, id: &str, current: Option<&str>) {
        if self.position(behavior, id).is_some() {
            return;
        }
        let tab = EditorTab { behavior, id: id.to_owned(), pinned: false, editor: None };
        if let Some(preview) = self.tabs.iter().position(|tab| tab.behavior == behavior && !tab.pinned) {
            self.tabs[preview] = tab;
            return;
        }
        let after = current.and_then(|current| self.position(behavior, current));
        self.tabs.insert(after.map_or(self.tabs.len(), |at| at + 1), tab);
    }

    /// The tab stays open from now on.
    pub(in crate::patch) fn pin(&mut self, behavior: usize, id: &str) {
        if let Some(at) = self.position(behavior, id) {
            self.tabs[at].pinned = true;
        }
    }

    /// Closes a tab. Returns the tab that takes its place: the next one, or else the previous one.
    pub(in crate::patch) fn close(&mut self, behavior: usize, id: &str) -> Option<String> {
        let at = self.position(behavior, id)?;
        self.tabs.remove(at);
        let next = self.tabs[at..].iter().find(|tab| tab.behavior == behavior);
        let neighbor = next.or_else(|| self.tabs[..at].iter().rfind(|tab| tab.behavior == behavior));
        neighbor.map(|tab| tab.id.clone())
    }

    /// Closes every tab of `behavior` but `id`.
    pub(in crate::patch) fn close_others(&mut self, behavior: usize, id: &str) {
        self.tabs.retain(|tab| tab.behavior != behavior || tab.id == id);
    }

    /// Closes the tabs of `behavior` right of `id`.
    pub(in crate::patch) fn close_to_the_right(&mut self, behavior: usize, id: &str) {
        let Some(at) = self.position(behavior, id) else { return };
        let mut index = 0;
        self.tabs.retain(|tab| {
            let keep = tab.behavior != behavior || index <= at;
            index += 1;
            keep
        });
    }

    /// Closes every tab of `behavior`.
    pub(in crate::patch) fn close_all(&mut self, behavior: usize) {
        self.tabs.retain(|tab| tab.behavior != behavior);
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

    #[test]
    fn the_preview_tab_is_replaced_until_pinned() {
        let mut tabs = EditorTabs::default();
        tabs.open(0, "#a", None);
        tabs.open(0, "#b", None);
        assert_eq!(ids(&tabs, 0), ["#b"]); // going through the tree opens only one tab

        tabs.pin(0, "#b");
        tabs.open(0, "#c", None);
        assert_eq!(ids(&tabs, 0), ["#b", "#c"]);
        tabs.open(0, "#b", None); // already open: nothing changes
        assert_eq!(ids(&tabs, 0), ["#b", "#c"]);
    }

    #[test]
    fn a_new_tab_goes_after_the_current_one() {
        let mut tabs = EditorTabs::default();
        for id in ["#a", "#b"] {
            tabs.open(0, id, None);
            tabs.pin(0, id);
        }
        tabs.open(0, "#c", Some("#a"));
        assert_eq!(ids(&tabs, 0), ["#a", "#c", "#b"]);
        // The preview tab is replaced where it is.
        tabs.open(0, "#d", Some("#b"));
        assert_eq!(ids(&tabs, 0), ["#a", "#d", "#b"]);
    }

    #[test]
    fn closes_to_the_right_and_all() {
        let mut tabs = EditorTabs::default();
        for id in ["#a", "#b", "#c"] {
            tabs.open(0, id, None);
            tabs.pin(0, id);
        }
        tabs.open(1, "#x", None);
        tabs.close_to_the_right(0, "#a");
        assert_eq!(ids(&tabs, 0), ["#a"]);
        assert_eq!(ids(&tabs, 1), ["#x"]); // another behavior keeps its tabs
        tabs.close_all(0);
        assert!(ids(&tabs, 0).is_empty());
        assert_eq!(ids(&tabs, 1), ["#x"]);
    }

    #[test]
    fn closing_shows_a_neighbor() {
        let mut tabs = EditorTabs::default();
        for id in ["#a", "#b", "#c"] {
            tabs.open(0, id, None);
            tabs.pin(0, id);
        }
        assert_eq!(tabs.close(0, "#b").as_deref(), Some("#c"));
        assert_eq!(tabs.close(0, "#c").as_deref(), Some("#a"));
        assert_eq!(tabs.close(0, "#a"), None);
        assert_eq!(tabs.close(0, "#gone"), None);
    }

    #[test]
    fn behaviors_have_their_own_tabs() {
        let mut tabs = EditorTabs::default();
        tabs.open(0, "#a", None);
        tabs.open(1, "#a", None);
        tabs.pin(1, "#a");
        tabs.open(1, "#b", None);
        assert_eq!(ids(&tabs, 0), ["#a"]);
        assert_eq!(ids(&tabs, 1), ["#a", "#b"]);
        assert_eq!(tabs.last_of(1), Some("#b"));

        // A neighbor is looked for in the same behavior only.
        assert_eq!(tabs.close(0, "#a"), None);

        tabs.close_others(1, "#b");
        assert_eq!(ids(&tabs, 1), ["#b"]);
    }

    #[test]
    fn a_tab_gets_its_editor_state_back() {
        let mut tabs = EditorTabs::default();
        tabs.open(0, "#a", None);
        tabs.store_state(0, "#a", CodeEditorState::default());
        assert!(tabs.tabs[0].editor.is_some());
        tabs.take_state(0, "#a");
        assert!(tabs.tabs[0].editor.is_none());
    }
}
