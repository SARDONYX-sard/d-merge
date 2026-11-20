//! A tab dragged from a group's own bar, as a host draws it (a central panel, a scrolling row of
//! `EditorTab`s), onto another group: the drop comes back.

use egui_dock_area::{Axis, Dock, DockArea, Dropped, GroupViewer, Half, Place, TabDrags, Target};

/// Groups named by a number; each a bar of one tab, its number.
struct Viewer;

impl GroupViewer for Viewer {
    type Group = u32;
    type Item = u32;

    fn ui(&mut self, ui: &mut egui::Ui, group: &mut u32, tabs: &mut TabDrags<'_, u32>) {
        egui::CentralPanel::default().show(ui, |ui| {
            let scroll = egui::ScrollArea::horizontal().id_salt("tabs").show(ui, |ui| {
                ui.horizontal(|ui| egui_widgets::EditorTab::new(group.to_string(), true).show(ui)).inner
            });
            let group = *group;
            tabs.tab(&scroll.inner.response, &group.to_string(), true, || group);
            tabs.bar(scroll.inner_rect);
        });
    }

    fn title(&mut self, group: &u32) -> String {
        group.to_string()
    }
}

const fn pointer(pos: egui::Pos2) -> egui::Event {
    egui::Event::PointerMoved(pos)
}

const fn button(pos: egui::Pos2, pressed: bool) -> egui::Event {
    let button = egui::PointerButton::Primary;
    egui::Event::PointerButton { pos, button, pressed, modifiers: egui::Modifiers::NONE }
}

#[test]
fn a_tab_dragged_onto_another_group_is_dropped_there() {
    let mut dock: Dock<u32, u32> = Dock::new(1);
    dock.put_beside(&Place::default(), (Axis::Horizontal, Half::Second), 2).unwrap();
    let ctx = egui::Context::default();
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0));
    let tab = egui::pos2(420.0, 12.0); // the tab of group 2, at the top left of the right half
    let onto = egui::pos2(100.0, 300.0); // the middle of group 1

    let mut steps = vec![vec![], vec![pointer(tab)], vec![button(tab, true)]];
    for x in (0..10).map(|i| egui::lerp(tab.x..=onto.x, i as f32 / 9.0)) {
        steps.push(vec![pointer(egui::pos2(x, egui::lerp(tab.y..=onto.y, (tab.x - x) / (tab.x - onto.x))))]);
    }
    steps.push(vec![button(onto, false)]);

    let mut dropped = None;
    for events in steps {
        let raw = egui::RawInput { events, screen_rect: Some(screen), ..Default::default() };
        let mut output = ctx.run_ui(raw, |ui| {
            let out = DockArea::new("dock", &mut dock).show_inside(ui, &mut Viewer);
            dropped = dropped.take().or(out.dropped);
        });
        output.textures_delta.clear();
    }
    let Some(Dropped { item, from, target }) = dropped else { panic!("nothing was dropped") };
    assert_eq!((item, from), (2, Place::new(egui_dock_area::Area::Main, vec![Half::Second])));
    assert!(matches!(&target, Target::Join(place, _) if place.path == [Half::First]), "{target:?}");
}
