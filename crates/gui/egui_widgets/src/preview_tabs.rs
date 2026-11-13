//! The tabs of an editor as VS Code keeps them, apart from drawing them ([`crate::EditorTab`]):
//!
//! ```text
//! [ #0106 GetUpModList ✖ ] [ #0200 Root ✖ ] [ #0300 hkbClipGenerator ✖ ]
//!                                             ^ italic: the preview tab
//! ```
//! - Opening something shows it in the **preview tab**, which the next one opened replaces, so going
//!   through a tree does not open hundreds of tabs. A tab stays once **pinned** (edited, double-clicked).
//! - Tabs are in **groups** (e.g. a project, a behavior): only one group's are shown, and closing
//!   others / to the right / all stays in the group.
//! - The **active** tab (the one shown), if the host keeps it here, follows the closes.
//!
//! What a tab holds is the host's (`T`); a [`Tab`] reads as its `T` (`tab.id`).
//!
//! ```text
//! let mut tabs = PreviewTabs::default();
//! let at = tabs.open(0, "#a", |id| *id == "#a", None);   // the preview tab
//! tabs.pin(at);                                          // kept open
//! tabs.open(0, "#b", |id| *id == "#b", Some(at));        // a new preview tab after it
//! ```

use std::ops::{Deref, DerefMut};

/// One tab: the host's `item` in group `group`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tab<T> {
    /// e.g. the index of a project.
    pub group: usize,
    /// `false`: the preview tab of its group, which the next one opened replaces (italic).
    pub pinned: bool,
    pub item: T,
}

impl<T> Deref for Tab<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &T {
        &self.item
    }
}

impl<T> DerefMut for Tab<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut T {
        &mut self.item
    }
}

/// See the module docs. Reads as the slice of its tabs (`tabs[at]`, `tabs.len()`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewTabs<T> {
    tabs: Vec<Tab<T>>,
    /// The tab shown, if the host keeps it here. Moved by the closes; `None` when it is closed.
    pub active: Option<usize>,
}

impl<T> Default for PreviewTabs<T> {
    #[inline]
    fn default() -> Self {
        Self { tabs: Vec::new(), active: None }
    }
}

impl<T> Deref for PreviewTabs<T> {
    type Target = [Tab<T>];

    #[inline]
    fn deref(&self) -> &[Tab<T>] {
        &self.tabs
    }
}

impl<T> DerefMut for PreviewTabs<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut [Tab<T>] {
        &mut self.tabs
    }
}

impl<T> PreviewTabs<T> {
    /// The tabs of `group` in order, with their indices.
    #[inline]
    pub fn of(&self, group: usize) -> impl DoubleEndedIterator<Item = (usize, &Tab<T>)> {
        self.tabs.iter().enumerate().filter(move |(_, tab)| tab.group == group)
    }

    /// The tab of `group` that `is` the one looked for.
    #[inline]
    pub fn find(&self, group: usize, mut is: impl FnMut(&T) -> bool) -> Option<usize> {
        self.tabs.iter().position(|tab| tab.group == group && is(&tab.item))
    }

    /// Adds a tab at the end as it is: e.g. one from the last session.
    #[inline]
    pub fn push(&mut self, tab: Tab<T>) {
        self.tabs.push(tab);
    }

    /// Shows `item` in a tab of `group`: the one that `is_same` if open (`item` is dropped), else the
    /// preview tab (replaced where it is), else a new preview tab after `after`, or at the end.
    /// Returns its index.
    pub fn open(&mut self, group: usize, item: T, is_same: impl FnMut(&T) -> bool, after: Option<usize>) -> usize {
        if let Some(at) = self.find(group, is_same) {
            return at;
        }
        let tab = Tab { group, pinned: false, item };
        if let Some(preview) = self.tabs.iter().position(|tab| tab.group == group && !tab.pinned) {
            self.tabs[preview] = tab;
            return preview;
        }
        let at = after.map_or(self.tabs.len(), |at| (at + 1).min(self.tabs.len()));
        self.tabs.insert(at, tab);
        if let Some(active) = &mut self.active
            && *active >= at
        {
            *active += 1;
        }
        at
    }

    /// Tab `at` stays open from now on.
    #[inline]
    pub fn pin(&mut self, at: usize) {
        if let Some(tab) = self.tabs.get_mut(at) {
            tab.pinned = true;
        }
    }

    /// Closes tab `at`. Returns the tab of its group that takes its place: the next one, or else the
    /// previous one (its index after the close).
    pub fn close(&mut self, at: usize) -> Option<usize> {
        if at >= self.tabs.len() {
            return None;
        }
        let group = self.tabs.remove(at).group;
        self.active = match self.active {
            Some(active) if active == at => None,
            Some(active) if active > at => Some(active - 1),
            active => active,
        };
        let next = (at..self.tabs.len()).find(|&i| self.tabs[i].group == group);
        next.or_else(|| (0..at).rev().find(|&i| self.tabs[i].group == group))
    }

    /// Closes every tab of `at`'s group but it. Returns its index after.
    pub fn close_others(&mut self, at: usize) -> Option<usize> {
        let group = self.tabs.get(at)?.group;
        self.close_where(|i, tab| tab.group == group && i != at);
        self.of(group).next().map(|(at, _)| at)
    }

    /// Closes the tabs of `at`'s group right of it.
    pub fn close_to_the_right(&mut self, at: usize) {
        let Some(group) = self.tabs.get(at).map(|tab| tab.group) else { return };
        self.close_where(|i, tab| tab.group == group && i > at);
    }

    /// Closes every tab of `group`.
    #[inline]
    pub fn close_all(&mut self, group: usize) {
        self.close_where(|_, tab| tab.group == group);
    }

    /// Closes every tab: e.g. the groups were replaced, so their indices no longer hold.
    #[inline]
    pub fn clear(&mut self) {
        self.tabs.clear();
        self.active = None;
    }

    /// Closes the tabs `(index, tab)` that `close` says, the last first: what is closed does not move
    /// those left to close.
    fn close_where(&mut self, mut close: impl FnMut(usize, &Tab<T>) -> bool) {
        for at in (0..self.tabs.len()).rev() {
            if close(at, &self.tabs[at]) {
                self.close(at);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn items(tabs: &PreviewTabs<&'static str>, group: usize) -> Vec<&'static str> {
        tabs.of(group).map(|(_, tab)| tab.item).collect()
    }

    fn open(tabs: &mut PreviewTabs<&'static str>, group: usize, item: &'static str, after: Option<usize>) -> usize {
        tabs.open(group, item, |known| *known == item, after)
    }

    #[test]
    fn the_preview_tab_is_replaced_until_pinned() {
        let mut tabs = PreviewTabs::default();
        open(&mut tabs, 0, "a", None);
        let b = open(&mut tabs, 0, "b", None);
        assert_eq!(items(&tabs, 0), ["b"], "going through a tree opens only one tab");
        tabs.pin(b);
        open(&mut tabs, 0, "c", None);
        assert_eq!(items(&tabs, 0), ["b", "c"]);
        assert_eq!(open(&mut tabs, 0, "b", None), 0, "already open: nothing changes");
        assert_eq!(items(&tabs, 0), ["b", "c"]);
    }

    #[test]
    fn a_new_tab_goes_after_the_current_one() {
        let mut tabs = PreviewTabs::default();
        for item in ["a", "b"] {
            let at = open(&mut tabs, 0, item, None);
            tabs.pin(at);
        }
        tabs.active = Some(1);
        open(&mut tabs, 0, "c", Some(0));
        assert_eq!((items(&tabs, 0), tabs.active), (vec!["a", "c", "b"], Some(2)), "the active one moves");
        open(&mut tabs, 0, "d", Some(1));
        assert_eq!(items(&tabs, 0), ["a", "d", "b"], "the preview tab is replaced where it is");
    }

    #[test]
    fn closes_stay_in_the_group_and_keep_the_active_one() {
        let mut tabs = PreviewTabs::default();
        for item in ["a", "b", "c"] {
            let at = open(&mut tabs, 0, item, None);
            tabs.pin(at);
        }
        open(&mut tabs, 1, "x", None);
        tabs.active = Some(3);
        tabs.close_to_the_right(0);
        assert_eq!((items(&tabs, 0), items(&tabs, 1), tabs.active), (vec!["a"], vec!["x"], Some(1)));
        tabs.close_all(0);
        assert_eq!((items(&tabs, 0).len(), tabs.active), (0, Some(0)));
        tabs.close(0);
        assert_eq!(tabs.active, None, "the active one closed");
    }

    #[test]
    fn closing_shows_a_neighbor_of_the_group() {
        let mut tabs = PreviewTabs::default();
        for (group, item) in [(0, "a"), (1, "x"), (0, "b"), (0, "c")] {
            let at = open(&mut tabs, group, item, None);
            tabs.pin(at);
        }
        assert_eq!(tabs.close(2), Some(2), "`b`: the next one, `c`");
        assert_eq!(tabs.close(2), Some(0), "`c`: the previous one, `a`, not `x`");
        assert_eq!(tabs.close(0), None);
        assert_eq!(tabs.close(9), None);
        let mut others = PreviewTabs::default();
        for item in ["a", "b", "c"] {
            let at = open(&mut others, 0, item, None);
            others.pin(at);
        }
        assert_eq!((others.close_others(1), items(&others, 0)), (Some(0), vec!["b"]));
    }
}
