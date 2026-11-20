//! The editor groups drawn by the dock (see `crate::patch::editor_area::groups` and
//! `egui_dock_area`), and what a dropped tab does.
//!
//! ```text
//! drawn       the active group as the editor area always was; another one swapped in for it,
//!             without the keys; a press in it makes it the active one (next frame)
//! dropped     an object's / a view's tab moves to the group it went onto, or to a new one (beside a
//!             group, or in a window of its own), with its editor state; the side tab moves too.
//!             A group left with no tab goes.
//! closed      a window closed by the user: its tabs come back to the active group
//! ```

use std::rc::Rc;

use egui_dock_area::{DockArea, Dropped, GroupViewer, Node, TabDrags, Target};
use gui_shared::i18n::{I18nKey, I18nMap};

use super::{EditorOut, preview::PreviewEvent};
use crate::patch::{
    TabEvent,
    editor_area::{
        EditorArea, EditorPane,
        groups::{DragItem, GroupId, Parked},
    },
    problems::Problems,
    tool::{keys::Keys, selection::Selection, work_state::Work},
    workspace::history::ObjectChange,
};

/// Draws each group, with what they need.
struct GroupsViewer<'a> {
    area: &'a mut EditorArea,
    i18n: &'a I18nMap,
    work: &'a mut Work,
    sel: &'a mut Selection,
    keys: &'a mut Keys,
    problems: (&'a mut Problems, &'a [ObjectChange]),
    /// What the active group asked of the rest; of the others, their files and edits.
    out: EditorOut,
    /// A group was pressed: it becomes the active one.
    pressed: Option<GroupId>,
}

impl GroupViewer for GroupsViewer<'_> {
    type Group = GroupId;
    type Item = DragItem;

    fn ui(&mut self, ui: &mut egui::Ui, group: &mut GroupId, drags: &mut TabDrags<'_, DragItem>) {
        let group = *group;
        let active = group == self.area.groups.active;
        let out = if active {
            self.draw(ui, group)
        } else {
            let Some(mut parked) = self.area.groups.parked.remove(&group) else { return };
            parked.pane.take_settings_from(&self.area.pane);
            let held = self.keys.set_aside();
            self.area.away = Some(self.area.swap(self.sel, parked));
            let out = self.draw(ui, group);
            let active_state = self.area.away.take().unwrap_or_default();
            let drawn = self.area.swap(self.sel, active_state);
            self.area.groups.parked.insert(group, drawn);
            self.keys.restore(held);
            out
        };
        std::mem::take(&mut self.area.strip).report(drags);

        if active {
            self.out = out;
        } else {
            self.out.file = self.out.file.take().or(out.file);
            self.out.edited = self.out.edited.take().or(out.edited);
            let pressed = ui.input(|input| input.pointer.any_pressed());
            let in_group = pressed && ui.rect_contains_pointer(ui.max_rect()) && !drags.dragging();
            if in_group && self.area.groups.takes_focus(group) {
                self.pressed = Some(group);
            }
        }
    }

    fn title(&mut self, group: &GroupId) -> String {
        let selected = if *group == self.area.groups.active {
            self.sel.selected.clone()
        } else {
            self.area.groups.parked.get(group).and_then(|parked| parked.sel.selected.clone())
        };
        selected.map_or_else(|| self.i18n.t(I18nKey::PatchPreviewTitle).to_owned(), |id| id.to_string())
    }
}

impl GroupsViewer<'_> {
    /// One group, as the editor area draws it.
    fn draw(&mut self, ui: &mut egui::Ui, group: GroupId) -> EditorOut {
        self.area.drawing = group;
        let (work, sel, keys) = (&mut *self.work, &mut *self.sel, &mut *self.keys);
        self.area.group_ui(ui, self.i18n, (work, sel, keys), (&mut *self.problems.0, self.problems.1))
    }
}

impl EditorArea {
    /// Every group, docked; then what was dropped, pressed or closed. Returns what the active group
    /// asked of the rest.
    pub(super) fn groups_ui(
        &mut self,
        ui: &mut egui::Ui,
        i18n: &I18nMap,
        (work, sel, keys): (&mut Work, &mut Selection, &mut Keys),
        problems: (&mut Problems, &[ObjectChange]),
    ) -> EditorOut {
        self.sync_side();
        self.go_where_open(sel);
        let user_patch = work.user_patch;
        let mut dock = std::mem::take(&mut self.groups.dock);
        let mut viewer =
            GroupsViewer { area: self, i18n, work, sel, keys, problems, out: EditorOut::default(), pressed: None };
        let output = DockArea::new(("editor_groups", user_patch), &mut dock).show_inside(ui, &mut viewer);
        let (out, pressed) = (viewer.out, viewer.pressed);
        self.groups.dock = dock;

        for node in output.closed {
            self.take_closed(node);
        }
        if let Some(group) = self.groups.to_close.take() {
            self.close_group(sel, group);
        }
        if let Some(dropped) = output.dropped {
            self.apply_drop(sel, dropped);
            ui.ctx().request_repaint();
        } else if let Some(group) = pressed {
            self.activate(sel, group);
            ui.ctx().request_repaint();
        }
        self.drop_empty_groups(sel);
        out
    }

    /// An object selected (e.g. in the tree) that another group has a tab of is shown there, as VS
    /// Code does: not opened twice, two editors of one text.
    fn go_where_open(&mut self, sel: &mut Selection) {
        let Some(id) = sel.selected.clone() else { return };
        let opened_here =
            self.pane.opened.as_ref().is_some_and(|(behavior, open)| *behavior == sel.active && **open == *id);
        if opened_here || self.tabs.of(sel.active).any(|tab| *tab.id == *id) {
            return;
        }
        let behavior = sel.active;
        let elsewhere =
            self.groups.parked.iter().find(|(_, parked)| parked.tabs.of(behavior).any(|tab| *tab.id == *id));
        let Some((&group, _)) = elsewhere else { return };
        // The active group keeps what it showed.
        sel.selected = self.pane.opened.as_ref().map(|(_, open)| Rc::from(open.as_str()));
        self.activate(sel, group);
        (sel.active, sel.selected) = (behavior, Some(id));
    }

    /// The side tab is there while the preview is on or a conflict is merged: in the active group,
    /// shown, when it opens.
    fn sync_side(&mut self) {
        let merging = self.pane.merge_values.is_some();
        match (self.pane.show_preview || merging, self.groups.side.is_some()) {
            (true, false) => {
                // As VS Code's "Open Preview to the Side": a group of its own right of the active one.
                let group = self.groups.new_id();
                let mut pane = EditorPane::default();
                pane.take_settings_from(&self.pane);
                self.groups.parked.insert(group, Parked { pane, ..Parked::default() });
                let at = self.groups.dock.groups().into_iter().find(|(_, id)| **id == self.groups.active);
                let side = (egui_dock_area::Axis::Horizontal, egui_dock_area::Half::Second);
                if let Some((place, _)) = at
                    && self.groups.dock.put_beside(&place, side, group).is_ok()
                {
                    self.groups.side = Some((group, true));
                }
            }
            (false, true) => self.groups.side = None,
            _ => {}
        }
    }

    /// Does what a dropped tab asked: see the module docs.
    fn apply_drop(&mut self, sel: &mut Selection, Dropped { item, from, target }: Dropped<DragItem>) {
        let group_at = |dock: &egui_dock_area::Dock<GroupId, DragItem>, place: &egui_dock_area::Place| {
            dock.groups().into_iter().find(|(at, _)| at == place).map(|(_, group)| *group)
        };
        let Some(from) = group_at(&self.groups.dock, &from) else { return };
        let to = match target {
            Target::Join(place, _) => match group_at(&self.groups.dock, &place) {
                Some(to) if to != from => to,
                _ => return, // among its own tabs: they stay as they are
            },
            Target::Beside(place, axis, half) => {
                let group = self.new_group();
                if self.groups.dock.put_beside(&place, (axis, half), group).is_err() {
                    self.groups.parked.remove(&group);
                    return;
                }
                group
            }
            Target::NewWindow(at) => {
                let group = self.new_group();
                self.groups.dock.open_window(group, at);
                group
            }
        };

        match item {
            DragItem::Object(behavior, id) => self.move_object(sel, (from, to), (behavior, &id)),
            DragItem::View(at) => self.move_view(sel, (from, to), at),
            DragItem::Side => self.groups.side = Some((to, true)),
        }
        self.activate(sel, to);
    }

    /// A new group, parked, with the active one's settings. Returns its number.
    fn new_group(&mut self) -> GroupId {
        let group = self.groups.new_id();
        let mut pane = EditorPane::default();
        pane.take_settings_from(&self.pane);
        self.groups.parked.insert(group, Parked { pane, ..Parked::default() });
        group
    }

    /// Object `id`'s tab moves from group `from` to `to`, with its editor state, shown there.
    fn move_object(&mut self, sel: &mut Selection, (from, to): (GroupId, GroupId), (behavior, id): (usize, &str)) {
        self.activate(sel, from);
        let shown = self.pane.opened.as_ref().is_some_and(|(at, open)| *at == behavior && open == id);
        let state = if shown { Some(std::mem::take(&mut self.pane.state)) } else { self.tabs.take_state(behavior, id) };
        let pinned = self.tabs.of(behavior).find(|tab| tab.id == id).is_some_and(|tab| tab.pinned);
        let current = sel.selected.as_deref().unwrap_or_default().to_owned();
        self.apply_tab_event(sel, Some(TabEvent::Object(egui_widgets::TabAction::Close, id.to_owned())), &current);

        self.activate(sel, to);
        let left = sel.selected.clone();
        self.tabs.open(behavior, id, left.as_deref().filter(|_| sel.active == behavior));
        if pinned {
            self.tabs.pin(behavior, id);
        }
        if let Some(state) = state {
            self.tabs.store_state(behavior, id, state);
        }
        sel.active = behavior;
        sel.selected = Some(Rc::from(id));
        self.side_off_in(to);
    }

    /// View `at`'s tab moves from group `from` to `to`, shown there.
    fn move_view(&mut self, sel: &mut Selection, (from, to): (GroupId, GroupId), at: usize) {
        self.activate(sel, from);
        let Some(view) = self.views.list.take(at) else { return };
        self.activate(sel, to);
        let (behavior, id) = (view.group, view.id.clone());
        self.views.list.push(view);
        self.views.show(self.views.list.len() - 1);
        sel.active = behavior;
        sel.selected = Some(Rc::from(id.as_str()));
        self.side_off_in(to);
    }

    /// Group `group` shows another tab than the side one now.
    fn side_off_in(&mut self, group: GroupId) {
        if self.groups.side_shown(group) {
            self.groups.side = Some((group, false));
        }
    }

    /// A window closed: the tabs of its groups come back to the active group, the side tab too.
    fn take_closed(&mut self, node: Node<GroupId>) {
        for (_, group) in node.groups() {
            if self.groups.has_side(*group) {
                self.groups.side = Some((self.groups.active, false));
            }
            let Some(parked) = self.groups.parked.remove(group) else { continue };
            for tab in parked.tabs.all() {
                self.tabs.restore(tab.group, &tab.id, tab.pinned);
            }
        }
    }

    /// The side tab's content, of the active group's object: a conflict's values while it is merged,
    /// else the patch preview (none for a new object: it is written as is). `sel`: the active
    /// group's, unless it is away while another is drawn.
    pub(super) fn side_ui(&mut self, ui: &mut egui::Ui, i18n: &I18nMap, work: &mut Work, sel: &Selection) {
        let (pane, sel) = match &mut self.away {
            Some(away) => (&mut away.pane, &away.sel),
            None => (&mut self.pane, sel),
        };
        let Some(id) = sel.selected.clone() else {
            ui.weak(i18n.t(I18nKey::ObjectSelectHint));
            return;
        };
        if pane.merge_values.as_ref().is_some_and(|values| *values.id == *id) {
            pane.merge_panel(ui, i18n, (work, sel.active), &id);
            return;
        }
        let behavior = &work.workspace.behaviors[sel.active];
        if !behavior.has_preview(&id) {
            ui.weak(i18n.t(I18nKey::NoChange));
            return;
        }
        let event: Option<PreviewEvent> = pane.preview_tab(ui, i18n);
        self.side_event = event.or_else(|| self.side_event.take());
    }

    /// The side tab's title, as its content: the merge's while a conflict is merged.
    pub(super) fn side_title(&self, i18n: &I18nMap) -> String {
        let pane = self.away.as_ref().map_or(&self.pane, |away| &away.pane);
        let key = if pane.merge_values.is_some() { I18nKey::MergeTitle } else { I18nKey::PatchPreviewTitle };
        i18n.t(key).to_owned()
    }
}

#[cfg(test)]
mod tests {
    use egui_dock_area::{Axis, Half, Place};

    use super::*;

    /// Two pinned tabs `#a`, `#b` of behavior 0, `#b` shown.
    fn area_of_two() -> (EditorArea, Selection) {
        let (mut area, mut sel) = (EditorArea::default(), Selection::default());
        for id in ["#a", "#b"] {
            area.tabs.restore(0, id, true);
        }
        sel.selected = Some(Rc::from("#b"));
        (area, sel)
    }

    fn ids(area: &EditorArea) -> Vec<String> {
        area.tabs.of(0).map(|tab| tab.id.clone()).collect()
    }

    fn drop(area: &mut EditorArea, sel: &mut Selection, (from, target): (Place, Target), id: &str) {
        let item = DragItem::Object(0, id.to_owned());
        area.apply_drop(sel, Dropped { item, from, target });
        area.drop_empty_groups(sel);
    }

    /// Dragged beside its group, a tab is a new group's, shown and active there; the group it left
    /// shows the one beside it. Dragged back onto it, its emptied group goes.
    #[test]
    fn a_tab_makes_a_group_beside_and_goes_back() {
        let (mut area, mut sel) = area_of_two();
        let beside = Target::Beside(Place::default(), Axis::Horizontal, Half::Second);
        drop(&mut area, &mut sel, (Place::default(), beside), "#b");
        assert_eq!(area.groups.dock.groups().len(), 2);
        assert_eq!((ids(&area), sel.selected.as_deref()), (vec!["#b".to_owned()], Some("#b")), "the new group");
        let left = &area.groups.parked[&0];
        assert_eq!(left.tabs.of(0).map(|tab| tab.id.as_str()).collect::<Vec<_>>(), ["#a"]);
        assert_eq!(left.sel.selected.as_deref(), Some("#a"), "the one beside it");

        let first = Place::new(egui_dock_area::Area::Main, vec![Half::First]);
        let second = Place::new(egui_dock_area::Area::Main, vec![Half::Second]);
        drop(&mut area, &mut sel, (second, Target::Join(first, 1)), "#b");
        assert_eq!(area.groups.dock.groups().len(), 1, "the emptied group went");
        assert_eq!(ids(&area), ["#a", "#b"]);
        assert_eq!(sel.selected.as_deref(), Some("#b"));
    }

    /// The preview on: the side tab, shown, in a group of its own right of the active one; off, it
    /// goes, and its emptied group too.
    #[test]
    fn the_side_tab_opens_to_the_side() {
        let (mut area, mut sel) = area_of_two();
        area.pane.show_preview = true;
        area.sync_side();
        let groups: Vec<GroupId> = area.groups.dock.groups().into_iter().map(|(_, group)| *group).collect();
        assert_eq!(groups.len(), 2);
        assert_eq!(area.groups.side, Some((groups[1], true)), "right of the active group, shown");

        area.pane.show_preview = false;
        area.sync_side();
        area.drop_empty_groups(&mut sel);
        assert_eq!(area.groups.side, None);
        assert_eq!(area.groups.dock.groups().len(), 1);
    }

    /// An object another group has a tab of is shown there when selected: not opened twice.
    #[test]
    fn an_object_open_elsewhere_is_shown_there() {
        let (mut area, mut sel) = area_of_two();
        let beside = Target::Beside(Place::default(), Axis::Horizontal, Half::Second);
        drop(&mut area, &mut sel, (Place::default(), beside), "#b");
        let with_b = area.groups.active;
        area.activate(&mut sel, 0);
        assert_eq!(sel.selected.as_deref(), Some("#a"));
        area.pane.opened = Some((0, "#a".to_owned())); // as its editor was drawn

        sel.selected = Some(Rc::from("#b")); // e.g. clicked in the tree
        area.go_where_open(&mut sel);
        assert_eq!(area.groups.active, with_b);
        assert_eq!(sel.selected.as_deref(), Some("#b"));
        assert_eq!(area.groups.parked[&0].sel.selected.as_deref(), Some("#a"), "the other group kept its own");
    }

    /// In a group showing the side tab, a click on another of its tabs shows that one.
    #[test]
    fn another_tab_clicked_hides_the_side_tab() {
        let (mut area, mut sel) = area_of_two();
        area.groups.side = Some((area.groups.active, true));
        let show = TabEvent::Object(egui_widgets::TabAction::Show, "#a".to_owned());
        area.apply_tab_event(&mut sel, Some(show), "#b");
        assert_eq!(area.groups.side, Some((area.groups.active, false)));
        assert_eq!(sel.selected.as_deref(), Some("#a"));
    }

    /// A group's `✖` closes it, the active one too (another is active then); the last stays.
    #[test]
    fn a_group_closes_but_the_last() {
        let (mut area, mut sel) = area_of_two();
        let beside = Target::Beside(Place::default(), Axis::Horizontal, Half::Second);
        drop(&mut area, &mut sel, (Place::default(), beside), "#b");
        let with_b = area.groups.active;

        area.close_group(&mut sel, with_b);
        assert_eq!(area.groups.dock.groups().len(), 1);
        assert_eq!((area.groups.active, sel.selected.as_deref()), (0, Some("#a")), "the other is active");
        area.close_group(&mut sel, 0);
        assert_eq!(area.groups.dock.groups().len(), 1, "the last stays");
    }

    /// A press on the group showing the side tab leaves the active group as it is: the preview
    /// keeps its object. Another tab shown there, the group is pressed into focus again.
    #[test]
    fn the_preview_keeps_its_object() {
        let (mut area, _) = area_of_two();
        area.pane.show_preview = true;
        area.sync_side();
        let (side, _) = area.groups.side.unwrap();
        assert!(!area.groups.takes_focus(side), "the preview's group");
        assert!(area.groups.takes_focus(area.groups.active));
        area.groups.side = Some((side, false));
        assert!(area.groups.takes_focus(side), "another of its tabs shown");
    }
}
