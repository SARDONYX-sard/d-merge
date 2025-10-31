use crate::{
    mod_item::ModItem,
    ui::{hyperlink_with_hover, label_with_hover, ROW_HEIGHT},
};
use eframe::egui::{self};
use rayon::prelude::*;

/// Handle drag-and-drop reordering of mods.
pub fn dnd_table_body(ui: &mut egui::Ui, items: &mut [ModItem], widths: [f32; 6]) {
    let checkbox_rect = [widths[0], ROW_HEIGHT];
    let w_path = widths[1];
    let w_name = widths[2];
    let w_mod_type = widths[3];
    let w_site = widths[4];
    let priority_size = [widths[5], ROW_HEIGHT];

    let row_width = widths.par_iter().sum::<f32>() + 33.0;

    let response =
        egui_dnd::dnd(ui, "mod_list_dnd").show_vec(items, |ui, item, draggable_handle, state| {
            // Since body cannot be used, stripe must be implemented manually.
            let mut bg_color = if state.index % 2 == 0 {
                ui.style().visuals.widgets.active.bg_fill
            } else {
                ui.visuals().widgets.noninteractive.bg_fill // gray
            };
            bg_color = bg_color.gamma_multiply(0.5);

            let row_rect = ui
                .allocate_rect(
                    egui::Rect::from_min_size(ui.min_rect().min, egui::vec2(row_width, ROW_HEIGHT)),
                    egui::Sense::hover(),
                )
                .rect;
            ui.painter().rect_filled(row_rect, 0.0, bg_color);

            ui.scope_builder(egui::UiBuilder::new().max_rect(row_rect), |ui| {
                ui.horizontal(|ui| {
                    ui.add_sized(
                        checkbox_rect,
                        egui::Checkbox::without_text(&mut item.enabled),
                    );
                    label_with_hover(ui, &item.id, w_path);

                    draggable_handle.ui(ui, |ui| {
                        label_with_hover(ui, &item.name, w_name);
                    });

                    label_with_hover(ui, item.mod_type.as_str(), w_mod_type);
                    hyperlink_with_hover(ui, &item.site, w_site);
                    ui.add_sized(priority_size, egui::Label::new(item.priority.to_string()));
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
pub fn check_only_table_body(
    body: &mut egui_extras::TableBody,
    filtered_items: &[ModItem],
    original_items: &mut [ModItem],
    widths: [f32; 6],
) {
    let w_checkbox = widths[0];
    let w_path = widths[1];
    let w_name = widths[2];
    let w_mod_type = widths[3];
    let w_site = widths[4];

    for filtered_item in filtered_items {
        let orig_item = original_items
            .iter_mut()
            .find(|o| o.id == filtered_item.id)
            .expect("Original item must exist");

        body.row(ROW_HEIGHT, |mut row| {
            row.col(|ui| {
                ui.add_sized(
                    [w_checkbox, ROW_HEIGHT],
                    egui::Checkbox::without_text(&mut orig_item.enabled),
                );
            });
            row.col(|ui| {
                label_with_hover(ui, &filtered_item.id, w_path);
            });
            row.col(|ui| {
                label_with_hover(ui, &filtered_item.name, w_name);
            });
            row.col(|ui| {
                label_with_hover(ui, filtered_item.mod_type.as_str(), w_mod_type);
            });
            row.col(|ui| {
                hyperlink_with_hover(ui, &filtered_item.site, w_site);
            });
            row.col(|ui| {
                ui.with_layout(
                    egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                    |ui| {
                        ui.label(filtered_item.priority.to_string());
                    },
                );
            });
        });
    }
}
