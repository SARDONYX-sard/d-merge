//! The editor groups, as VS Code's: each an editor with its own tabs, docked side by side, one above
//! the other, or in a window of its own (see `egui_dock_area`). A tab dragged from one group's bar
//! goes onto another's, or beside a group (a new one), or out of the window.
//!
//! ```text
//! ┌ group 1 (active) ─────────────────┬ group 2 ─────────────────┐
//! │ [Root] [Idle ●]                   │ [GetUpModList] [Preview] │
//! │ the editor of Idle                │ the patch preview        │
//! └───────────────────────────────────┴──────────────────────────┘
//! ```
//!
//! The active group (the one last clicked) is [`EditorArea`]'s own state, as when there was one: its
//! pane, tabs, views and the [`Selection`]. The others are parked here, each with its own; one is
//! swapped in to be drawn (without the keys: they are the active group's), and swapped back.
//!
//! The side tab (the patch preview, or a conflict's values while merged) is in one group's bar; it
//! shows the active group's object.

use fast_hash::HashMap;

use super::{EditorArea, pane::EditorPane, revision_view::Views};
use crate::patch::{editor::tabs::EditorTabs, tool::selection::Selection};

/// A group's number in the dock.
pub(in crate::patch) type GroupId = u64;

/// What a dragged tab takes along.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::patch) enum DragItem {
    /// An object's tab: `(behavior, id)`.
    Object(usize, String),
    /// A diff's / blame's tab, by its index in its group's views.
    View(usize),
    /// The side tab: the patch preview, or a conflict's values.
    Side,
}

/// A group while another is active: what [`EditorArea`] holds of the active one.
#[derive(Debug, Default)]
pub(super) struct Parked {
    pub(super) pane: EditorPane,
    pub(super) tabs: EditorTabs,
    pub(super) views: Views,
    pub(super) sel: Selection,
}

impl Parked {
    /// Has no tab: neither an object's, a view's, nor the side tab (`side_here`).
    fn is_empty(&self, side_here: bool) -> bool {
        !side_here && self.tabs.all().next().is_none() && self.views.list.is_empty()
    }
}

/// See the module docs.
#[derive(Debug)]
pub(in crate::patch) struct Groups {
    pub(super) dock: egui_dock_area::Dock<GroupId, DragItem>,
    pub(super) active: GroupId,
    pub(super) parked: HashMap<GroupId, Parked>,
    /// The group whose bar has the side tab, and whether it is the tab shown there.
    pub(super) side: Option<(GroupId, bool)>,
    /// A group's `✖` was clicked: it closes once all are drawn.
    pub(super) to_close: Option<GroupId>,
    next: GroupId,
}

impl Default for Groups {
    #[inline]
    fn default() -> Self {
        let dock = egui_dock_area::Dock::new(0);
        Self { dock, active: 0, parked: HashMap::default(), side: None, to_close: None, next: 1 }
    }
}

impl Groups {
    /// A new group's number.
    pub(super) const fn new_id(&mut self) -> GroupId {
        let id = self.next;
        self.next += 1;
        id
    }

    /// Group `group` has the side tab (shown or not).
    pub(super) fn has_side(&self, group: GroupId) -> bool {
        self.side.is_some_and(|(at, _)| at == group)
    }

    /// A press in `group` makes it the active one: not while it shows the side tab, as VS Code's
    /// preview: that keeps showing the object of the group last active (its `▲ ▼` work on it).
    pub(super) fn takes_focus(&self, group: GroupId) -> bool {
        !self.side_shown(group)
    }

    /// The side tab is the one shown in `group`.
    pub(super) fn side_shown(&self, group: GroupId) -> bool {
        self.side == Some((group, true))
    }
}

/// What a group's tab bar draws besides its objects' and views' tabs, and tells the dock: see
/// `super::tab_bar`.
#[derive(Debug, Default)]
pub(super) struct TabStrip {
    /// The side tab, if it is in this bar: `(title, shown)`.
    pub(super) side: Option<(String, bool)>,
    /// The bar: a tab dropped on it goes among its tabs.
    pub(super) bar: Option<egui::Rect>,
    /// Each tab drawn: what dragging it takes along.
    pub(super) tabs: Vec<(egui::Response, String, DragItem)>,
}

impl TabStrip {
    /// Tells the dock the bar and its tabs, for them to be dragged and dropped on.
    pub(super) fn report(self, drags: &mut egui_dock_area::TabDrags<'_, DragItem>) {
        if let Some(bar) = self.bar {
            drags.bar(bar);
        }
        for (response, title, item) in self.tabs {
            drags.tab(&response, &title, true, || item);
        }
    }
}

impl EditorPane {
    /// What is the user's, not the group's: the keymap, the language options, what is shown beside
    /// the text. A group's pane takes them from the active one's.
    pub(super) fn take_settings_from(&mut self, active: &Self) {
        self.mode = active.mode;
        self.lsp.clone_from(&active.lsp);
        self.show_inline_diff = active.show_inline_diff;
        self.follow_preview = active.follow_preview;
        self.show_preview = active.show_preview;
    }
}

impl EditorArea {
    /// Group `group` is the active one now: the active one is parked, and it is unparked.
    pub(in crate::patch) fn activate(&mut self, sel: &mut Selection, group: GroupId) {
        if group == self.groups.active {
            return;
        }
        let Some(mut next) = self.groups.parked.remove(&group) else { return };
        next.pane.take_settings_from(&self.pane);
        let left = self.swap(sel, next);
        self.groups.parked.insert(self.groups.active, left);
        self.groups.active = group;
    }

    /// Puts `parked` in the active group's place. Returns what was there.
    pub(super) const fn swap(&mut self, sel: &mut Selection, mut parked: Parked) -> Parked {
        std::mem::swap(&mut self.pane, &mut parked.pane);
        std::mem::swap(&mut self.tabs, &mut parked.tabs);
        std::mem::swap(&mut self.views, &mut parked.views);
        std::mem::swap(sel, &mut parked.sel);
        parked
    }

    /// Group `group` closes, as VS Code's `Close Group`: its tabs with it (the side tab turns the
    /// preview off). The last group stays.
    pub(super) fn close_group(&mut self, sel: &mut Selection, group: GroupId) {
        if self.groups.dock.groups().len() < 2 {
            return;
        }
        if group == self.groups.active {
            let Some(&other) = self.groups.parked.keys().next() else { return };
            self.activate(sel, other);
        }
        if self.groups.has_side(group) {
            self.groups.side = None;
            self.pane.show_preview = false;
        }
        self.groups.parked.remove(&group);
        self.groups.dock.retain(|&kept| kept != group);
    }

    /// Groups with no tab go, but one: the active one if it is empty and another is not.
    pub(super) fn drop_empty_groups(&mut self, sel: &mut Selection) {
        let active_empty =
            self.tabs.all().next().is_none() && self.views.list.is_empty() && !self.groups.has_side(self.groups.active);
        if active_empty && let Some(&other) = self.groups.parked.keys().next() {
            let empty = self.groups.active;
            self.activate(sel, other);
            self.groups.parked.remove(&empty);
            self.groups.dock.retain(|&group| group != empty);
        }

        let side = self.groups.side.map(|(group, _)| group);
        let empty: Vec<GroupId> = (self.groups.parked.iter())
            .filter(|(id, parked)| parked.is_empty(side == Some(**id)))
            .map(|(id, _)| *id)
            .collect();
        for group in empty {
            self.groups.parked.remove(&group);
            self.groups.dock.retain(|&kept| kept != group);
        }
    }
}
