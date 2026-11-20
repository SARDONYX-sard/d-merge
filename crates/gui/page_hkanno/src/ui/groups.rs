//! The editor groups, as VS Code's (see `egui_dock_area`): each a bar of tabs and the editor of the
//! one it shows, side by side, one above the other, or in a window of its own. A tab dragged from a
//! group's bar goes onto another's, beside a group (a new one), or out of the window.
//!
//! ```text
//! ┌ group 0 (active) ──────────────────┬ group 1 ───────────────┐
//! │ [attack.hkx ✖] [idle.hkx ●]  [Open]│ [walk.hkx ✖]           │
//! │ the editor of idle.hkx             │ the editor of walk.hkx │
//! └────────────────────────────────────┴────────────────────────┘
//! ```
//!
//! A file's tab keeps its own editor, so a group is only which tabs are its (each tab's `group`) and
//! the one it shows. The active group (the one last clicked) has the keys; the page's bars (the file,
//! the toolbar, Problems, the preview) are of its shown tab, [`HkannoPage`]'s `active`.

use egui_dock_area::{DockArea, Dropped, GroupViewer, Node, TabDrags, Target};
use egui_widgets::{TabAction, shadcn::Button};
use gui_shared::i18n::I18nKey;

use crate::{HkannoPage, HkannoPageCtx};

/// A group: its number, and the input path of the tab it shows.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Group {
    pub(crate) id: u64,
    pub(crate) shown: Option<String>,
}

/// The groups, docked. See the module docs.
#[derive(Debug, Default)]
pub(crate) struct Groups {
    dock: egui_dock_area::Dock<Group, String>,
    pub(crate) active: u64,
    next: u64,
}

impl Groups {
    /// A new group's number.
    const fn new_id(&mut self) -> u64 {
        self.next += 1;
        self.next
    }
}

/// Draws each group.
struct Viewer<'a, 'c> {
    page: &'a mut HkannoPage,
    ctx: &'a mut HkannoPageCtx<'c>,
    /// A group was pressed: it becomes the active one.
    pressed: Option<u64>,
}

impl GroupViewer for Viewer<'_, '_> {
    type Group = Group;
    type Item = String;

    fn ui(&mut self, ui: &mut egui::Ui, group: &mut Group, drags: &mut TabDrags<'_, String>) {
        let active = group.id == self.page.groups.active;
        self.page.group_bar(ui, self.ctx, group, drags);
        let shown = group.shown.as_deref().and_then(|path| self.page.index_of(path));
        let Some(index) = shown else {
            ui.centered_and_justified(|ui| ui.weak(self.ctx.i18n.t(I18nKey::HkannoEmptyMessage)));
            return;
        };

        if active {
            self.page.active = index;
            self.page.editor(ui, self.ctx);
        } else {
            // Drawn as if active, without the keys: they are the active group's.
            let (active_tab, keys) = (self.page.active, std::mem::take(&mut self.page.key_commands));
            self.page.active = index;
            self.page.editor(ui, self.ctx);
            (self.page.active, self.page.key_commands) = (active_tab, keys);
            let pressed = ui.input(|input| input.pointer.any_pressed());
            if pressed && ui.rect_contains_pointer(ui.max_rect()) && !drags.dragging() {
                self.pressed = Some(group.id);
            }
        }
    }

    fn title(&mut self, group: &Group) -> String {
        group
            .shown
            .as_deref()
            .and_then(|path| self.page.index_of(path))
            .map_or_else(String::new, |at| self.page.tabs[at].title().to_owned())
    }
}

impl HkannoPage {
    /// The index of the tab of `path`.
    fn index_of(&self, path: &str) -> Option<usize> {
        self.tabs.iter().position(|tab| tab.settings.input_path == path)
    }

    /// Every group, docked, in all the room of `ui`; then what was dropped, pressed or closed.
    pub(super) fn groups_ui(&mut self, ui: &mut egui::Ui, ctx: &mut HkannoPageCtx<'_>) {
        self.tidy_groups();
        let mut dock = std::mem::take(&mut self.groups.dock);
        let mut viewer = Viewer { page: self, ctx, pressed: None };
        let output = DockArea::new("hkanno_groups", &mut dock).show_inside(ui, &mut viewer);
        let pressed = viewer.pressed;
        self.groups.dock = dock;

        for node in output.closed {
            self.take_closed(&node);
        }
        if let Some(dropped) = output.dropped {
            self.apply_drop(dropped);
            ui.ctx().request_repaint();
        } else if let Some(group) = pressed {
            self.groups.active = group;
            ui.ctx().request_repaint();
        }
        self.tidy_groups();
    }

    /// The groups as the tabs are: a group shows one of its tabs (the active tab, in the active
    /// group), a group with no tab goes (but one), a tab of no group is the active group's.
    pub(crate) fn tidy_groups(&mut self) {
        let ids: Vec<u64> = self.groups.dock.groups().into_iter().map(|(_, group)| group.id).collect();
        let active = self.groups.active;
        for tab in &mut self.tabs {
            if !ids.contains(&tab.group) {
                tab.group = active;
            }
        }
        let tabs = &self.tabs;
        self.groups.dock.retain(|group| tabs.iter().any(|tab| tab.group == group.id));
        let ids: Vec<u64> = self.groups.dock.groups().into_iter().map(|(_, group)| group.id).collect();
        if !ids.contains(&self.groups.active) {
            self.groups.active = ids.first().copied().unwrap_or_default();
        }

        let active_path = self.tabs.get(self.active).map(|tab| (tab.group, tab.settings.input_path.clone()));
        for group in self.groups.dock.groups_mut() {
            if let Some((tab_group, path)) = &active_path
                && *tab_group == group.id
                && group.id == self.groups.active
            {
                group.shown = Some(path.clone());
            }
            let shown_is_its = group.shown.as_deref().is_some_and(|path| {
                self.tabs.iter().any(|tab| tab.group == group.id && tab.settings.input_path == path)
            });
            if !shown_is_its {
                group.shown =
                    self.tabs.iter().find(|tab| tab.group == group.id).map(|tab| tab.settings.input_path.clone());
            }
        }
        // The active tab is the active group's shown one.
        let shown = self.groups.dock.groups().into_iter().find(|(_, group)| group.id == self.groups.active);
        if let Some(index) = shown.and_then(|(_, group)| group.shown.as_deref()).and_then(|path| self.index_of(path)) {
            self.active = index;
        }
    }

    /// The tab of `path` goes where it was dropped: onto a group, or into a new one.
    fn apply_drop(&mut self, Dropped { item: path, target, .. }: Dropped<String>) {
        let Some(at) = self.index_of(&path) else { return };
        let to = match target {
            Target::Join(place, _) => {
                let group = self.groups.dock.groups().into_iter().find(|(at, _)| *at == place);
                let Some((_, group)) = group else { return };
                group.id
            }
            Target::Beside(place, axis, half) => {
                let id = self.groups.new_id();
                let group = Group { id, shown: Some(path.clone()) };
                if self.groups.dock.put_beside(&place, (axis, half), group).is_err() {
                    return;
                }
                id
            }
            Target::NewWindow(rect) => {
                let id = self.groups.new_id();
                self.groups.dock.open_window(Group { id, shown: Some(path.clone()) }, rect);
                id
            }
        };
        self.tabs[at].group = to;
        for group in self.groups.dock.groups_mut() {
            if group.id == to {
                group.shown = Some(path.clone());
            }
        }
        self.groups.active = to;
        self.active = at;
    }

    /// A window closed: its tabs come back to the active group.
    fn take_closed(&mut self, node: &Node<Group>) {
        let closed: Vec<u64> = node.groups().into_iter().map(|(_, group)| group.id).collect();
        for tab in &mut self.tabs {
            if closed.contains(&tab.group) {
                tab.group = self.groups.active;
            }
        }
    }

    /// `│ attack.hkx ✖ │ idle.hkx ● │ ...  [Open]`: the tabs of `group`, VS Code's
    /// ([`egui_widgets::EditorTab`]), each told to the dock to be dragged. `●`: unsaved edits; red /
    /// yellow: the file has errors / warnings. `[Open]` in the active group's.
    fn group_bar(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &mut HkannoPageCtx<'_>,
        group: &mut Group,
        drags: &mut TabDrags<'_, String>,
    ) {
        let menu = egui_widgets::TabMenu::from_texts(gui_shared::i18n::tab_menu_texts(ctx.i18n));
        let mut picked = None;
        let mut open = None;
        let active = group.id == self.groups.active;
        ui.horizontal(|ui| {
            let scroll = egui::ScrollArea::horizontal().id_salt("hkanno_tab_scroll").show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0; // the tabs touch, with a line between them
                    for (i, tab) in self.tabs.iter().enumerate().filter(|(_, tab)| tab.group == group.id) {
                        let worst = egui_code_editor::problems::worst(&tab.editor_diagnostics);
                        let tone = worst.map(|severity| severity.color(ui.visuals()));
                        let focused = ui.memory(|m| m.has_focus(tab.editor_id()));
                        let path = tab.settings.input_path.as_str();
                        let shown = egui_widgets::EditorTab::new(tab.title(), group.shown.as_deref() == Some(path))
                            .focused(focused)
                            .modified(tab.dirty)
                            .tone(tone)
                            .hover(path)
                            .show(ui);
                        let more = |ui: &mut egui::Ui| {
                            super::tab_menu(ui, tab, ctx.i18n);
                            None
                        };
                        if let Some(action) = shown.action(&menu, |action| (i, action), more) {
                            picked = Some(action);
                        }
                        drags.tab(&shown.response, tab.title(), true, || path.to_owned());
                    }
                });
            });
            drags.bar(scroll.inner_rect);

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let open_button = Button::new(ctx.i18n.t(I18nKey::OpenButton));
                if active && ui.add(open_button).on_hover_text(ctx.i18n.t(I18nKey::HkannoOpenHover)).clicked() {
                    open =
                        gui_shared::rfd::FileDialog::new().add_filter("Havok Animation", &["hkx", "xml"]).pick_files();
                }
            });
        });

        if let Some(files) = open {
            self.open(files.into_iter().map(|p| p.display().to_string()), ctx);
        }
        let unsaved = match picked {
            Some((at, TabAction::Show)) => {
                group.shown = Some(self.tabs[at].settings.input_path.clone());
                self.groups.active = group.id;
                self.active = at;
                false
            }
            Some((at, TabAction::Close)) => {
                self.close(at);
                false
            }
            Some((at, TabAction::CloseOthers)) => self.close_in(group.id, |index| index != at),
            Some((at, TabAction::CloseToTheRight)) => self.close_in(group.id, |index| index > at),
            Some((_, TabAction::CloseAll)) => self.close_in(group.id, |_| true),
            // No preview tabs here: every file opened stays.
            Some((_, TabAction::KeepOpen)) | None => false,
        };
        if unsaved {
            ctx.notify.error(ctx.i18n.t(I18nKey::HkannoTabUnsaved).to_owned());
        }
    }

    /// Closes the tabs of group `group` at the indices `close` says. See [`Self::close_where`].
    fn close_in(&mut self, group: u64, close: impl Fn(usize) -> bool) -> bool {
        let of_group: Vec<bool> = self.tabs.iter().map(|tab| tab.group == group).collect();
        self.close_where(|index| of_group[index] && close(index))
    }
}

#[cfg(test)]
mod tests {
    use egui_dock_area::{Axis, Half, Place};

    use super::*;
    use crate::tab::HkannoTab;

    /// Tabs of the files `paths`, in group 0, the first active.
    fn page(paths: &[&str]) -> HkannoPage {
        let tabs = paths.iter().map(|path| HkannoTab::blank(path)).collect();
        let mut page = HkannoPage { tabs, ..HkannoPage::default() };
        page.tidy_groups();
        page
    }

    fn dropped(path: &str, target: Target) -> Dropped<String> {
        Dropped { item: path.to_owned(), from: Place::default(), target }
    }

    /// Dragged beside its group, a tab is a new group's, shown and active; dragged back onto it,
    /// the emptied group goes.
    #[test]
    fn a_tab_makes_a_group_beside_and_goes_back() {
        let mut page = page(&["a.hkx", "b.hkx"]);
        page.apply_drop(dropped("b.hkx", Target::Beside(Place::default(), Axis::Horizontal, Half::Second)));
        page.tidy_groups();
        let groups = page.groups.dock.groups();
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].1.shown.as_deref(), Some("a.hkx"), "the group it left shows another");
        assert_eq!(groups[1].1.shown.as_deref(), Some("b.hkx"));
        assert_eq!((page.groups.active, page.active), (groups[1].1.id, 1), "the new group, active");

        let first = Place::new(egui_dock_area::Area::Main, vec![Half::First]);
        page.apply_drop(dropped("b.hkx", Target::Join(first, 1)));
        page.tidy_groups();
        assert_eq!(page.groups.dock.groups().len(), 1, "the emptied group went");
        assert_eq!(page.tabs.iter().filter(|tab| tab.group == page.groups.active).count(), 2);
    }

    /// Closing the last tab of a group closes it; the active tab is of the group left.
    #[test]
    fn a_group_without_tabs_goes() {
        let mut page = page(&["a.hkx", "b.hkx"]);
        page.apply_drop(dropped("b.hkx", Target::Beside(Place::default(), Axis::Horizontal, Half::Second)));
        page.close(1);
        page.tidy_groups();
        assert_eq!(page.groups.dock.groups().len(), 1);
        assert_eq!(page.tabs[page.active].settings.input_path, "a.hkx");
    }
}
