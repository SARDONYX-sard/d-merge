use d_merge_gui_shared::mod_item::ModItem;
use eframe::egui::{self};
use rayon::prelude::*;

use super::label_ext::{CellAlign, ROW_HEIGHT, hyperlink_with_hover, label_with_hover};

/// Handle drag-and-drop reordering of mods.
pub(crate) fn dnd_table_body(ui: &mut egui::Ui, items: &mut [ModItem], widths: [f32; 6]) {
    let checkbox_rect = [widths[0], ROW_HEIGHT];
    let w_path = widths[1];
    let w_name = widths[2];
    let w_mod_type = widths[3];
    let w_site = widths[4];
    let priority_size = [widths[5], ROW_HEIGHT];

    let row_width = widths.iter().sum::<f32>() + 33.0;

    let response =
        egui_dnd::dnd(ui, "mod_list_dnd").show_vec(items, |ui, item, draggable_handle, state| {
            // Since body cannot be used, stripe must be implemented manually.
            let bg_color = if state.index.is_multiple_of(2) {
                ui.style().visuals.widgets.active.bg_fill
            } else {
                ui.visuals().widgets.noninteractive.bg_fill // gray
            }
            .gamma_multiply(0.5);

            let row_rect = ui
                .allocate_rect(
                    egui::Rect::from_min_size(ui.min_rect().min, egui::vec2(row_width, ROW_HEIGHT)),
                    egui::Sense::hover(),
                )
                .rect;
            ui.painter().rect_filled(row_rect, 0.0, bg_color);

            ui.scope_builder(egui::UiBuilder::new().max_rect(row_rect), |ui| {
                ui.horizontal(|ui| {
                    ui.add_sized(checkbox_rect, egui::Checkbox::without_text(&mut item.enabled));
                    label_with_hover(ui, &item.id, w_path, CellAlign::Left);
                    draggable_handle
                        .ui(ui, |ui| label_with_hover(ui, &item.name, w_name, CellAlign::Center));

                    label_with_hover(ui, item.mod_type.as_str(), w_mod_type, CellAlign::Center);
                    hyperlink_with_hover(ui, &item.site, w_site);
                    centered_ui(ui, |ui| {
                        ui.add_sized(priority_size, egui::Label::new(item.priority.to_string()))
                    });
                });
            });
        });

    // reorder priority by index
    if response.final_update().is_some() {
        use rayon::prelude::*;
        items.par_iter_mut().enumerate().for_each(|(idx, item)| {
            item.priority = idx + 1;
        });
    }
}

/// This table is read-only for all fields except the `enabled` checkbox.
/// Useful for displaying filtered or sorted items where drag-and-drop is disabled.
pub(crate) fn check_only_table_body(
    body: &mut egui_extras::TableBody,
    filtered_ids: &[ModItem],
    original_items: &mut [ModItem],
    widths: [f32; 6],
) {
    // If `mod_list` is empty after the fetch operation, unnecessary rendering can be avoided early on.
    if original_items.is_empty() {
        return;
    }

    let checkbox_size = [widths[0], ROW_HEIGHT];
    let w_path = widths[1];
    let w_name = widths[2];
    let w_mod_type = widths[3];
    let w_site = widths[4];

    let mut orig_map: rapidhash::fast::RapidHashMap<String, &mut ModItem> =
        original_items.par_iter_mut().map(|o| (o.id.clone(), o)).collect();

    for filtered_mod in filtered_ids {
        let Some(&mut &mut ModItem {
            ref mut id,
            ref mut name,
            ref mut mod_type,
            ref mut site,
            ref mut priority,
            mut enabled,
        }) = orig_map.get_mut(&filtered_mod.id)
        else {
            continue;
        };

        body.row(ROW_HEIGHT, |mut row| {
            row.col(|ui| {
                ui.add_sized(checkbox_size, egui::Checkbox::without_text(&mut enabled));
            });
            row.col(|ui| label_with_hover(ui, id, w_path, CellAlign::Left));
            row.col(|ui| label_with_hover(ui, name, w_name, CellAlign::Center));
            row.col(|ui| label_with_hover(ui, mod_type.as_str(), w_mod_type, CellAlign::Center));
            row.col(|ui| hyperlink_with_hover(ui, site, w_site));
            row.col(|ui| centered_ui(ui, |ui| ui.label(priority.to_string())));
        });
    }
}

fn centered_ui<R>(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui) -> R) {
    ui.with_layout(
        egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
        add_contents,
    );
}
