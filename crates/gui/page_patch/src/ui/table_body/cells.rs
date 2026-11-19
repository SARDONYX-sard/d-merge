//! The cells of a row, left to right, the same in both tables. See the module docs of
//! [`super`].

use gui_shared::settings::pages::patch::mod_item::ModItem;

use super::{CheckClick, RowConflicts};
use crate::ui::label_ext::{CellAlign, ROW_HEIGHT, hyperlink_with_hover, label_with_hover};

/// `[☑]`. Returns the click on it, with Shift.
pub(super) fn check(
    ui: &mut egui::Ui,
    item: &mut ModItem,
    width: f32,
    conflicts: &mut RowConflicts<'_>,
) -> Option<CheckClick> {
    let response = ui.add_sized([width, ROW_HEIGHT], egui::Checkbox::without_text(&mut item.enabled));
    conflicts.menu(&response, &item.id);

    if !response.clicked() {
        return None;
    }
    Some((ui.input(|input| input.modifiers.shift), item.id.clone(), item.enabled))
}

/// `+`, `-`, ... : see [`RowConflicts::flag`].
pub(super) fn flag(ui: &mut egui::Ui, item: &ModItem, width: f32, conflicts: &RowConflicts<'_>) {
    conflicts.flag(ui, &item.id, width);
}

/// The mod's directory. e.g. `D:/mods/a/Nemesis_Engine/mod/para`
pub(super) fn path(ui: &mut egui::Ui, item: &ModItem, width: f32) {
    label_with_hover(ui, &item.id, width, CellAlign::Left);
}

/// e.g. `Paraglider`
pub(super) fn name(ui: &mut egui::Ui, item: &ModItem, width: f32) {
    label_with_hover(ui, &item.name, width, CellAlign::Center);
}

/// e.g. `Nemesis`
pub(super) fn mod_type(ui: &mut egui::Ui, item: &ModItem, width: f32) {
    label_with_hover(ui, item.mod_type.as_str(), width, CellAlign::Center);
}

/// e.g. `https://www.nexusmods.com/...`, with the row's right-click menu.
pub(super) fn site(ui: &mut egui::Ui, item: &ModItem, width: f32, conflicts: &mut RowConflicts<'_>) {
    let site = hyperlink_with_hover(ui, &item.site, width);
    conflicts.menu(&site, &item.id);
}

/// e.g. `1`. Not selectable: it would take the clicks, and with them the row's right-click menu.
pub(super) fn priority(ui: &mut egui::Ui, item: &ModItem, width: f32) {
    let priority = egui::Label::new(item.priority.to_string()).selectable(false);
    let centered = egui::Layout::centered_and_justified(egui::Direction::LeftToRight);
    ui.with_layout(centered, |ui| ui.add_sized([width, ROW_HEIGHT], priority));
}
