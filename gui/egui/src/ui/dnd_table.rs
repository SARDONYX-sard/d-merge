use d_merge_gui_shared::mod_item::ModItem;
use egui_extras::TableBody;
use rayon::prelude::*;

use super::label_ext::{CellAlign, ROW_HEIGHT, hyperlink_with_hover, label_with_hover};

/// Handle drag-and-drop reordering of mods and Shift-click range selection.
pub(crate) fn dnd_table_body(
    body: &mut TableBody,
    items: &mut [ModItem],
    widths: [f32; 6],
    selection_anchor: &mut Option<String>,
) {
    let ui = body.ui_mut();
    let checkbox_rect = [widths[0], ROW_HEIGHT];
    let w_path = widths[1];
    let w_name = widths[2];
    let w_mod_type = widths[3];
    let w_site = widths[4];
    let priority_size = [widths[5], ROW_HEIGHT];

    let row_width = widths.iter().sum::<f32>() + 38.0;
    let mut selection_request: Option<(bool, String, bool)> = None;

    let response =
        egui_dnd::dnd(ui, "mod_list_dnd").show_vec(items, |ui, item, draggable_handle, state| {
            let row_rect = ui
                .allocate_rect(
                    egui::Rect::from_min_size(ui.min_rect().min, egui::vec2(row_width, ROW_HEIGHT)),
                    egui::Sense::hover(),
                )
                .rect;

            // Stripe must be implemented manually.
            let bg_color = if state.index.is_multiple_of(2) {
                // ui.visuals().faint_bg_color // default table stripe
                ui.style().visuals.widgets.active.bg_fill.gamma_multiply(0.5) // gray
            } else if state.dragged {
                ui.style().visuals.widgets.active.bg_fill
            } else {
                egui::Color32::TRANSPARENT
            };

            ui.painter().rect_filled(row_rect, egui::CornerRadius::ZERO, bg_color);

            ui.scope_builder(egui::UiBuilder::new().max_rect(row_rect), |ui| {
                ui.horizontal(|ui| {
                    let response = ui
                        .add_sized(checkbox_rect, egui::Checkbox::without_text(&mut item.enabled));

                    if response.clicked() {
                        selection_request = Some((
                            ui.input(|input| input.modifiers.shift),
                            item.id.clone(),
                            item.enabled,
                        ));
                    }

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

    if let Some((shift, clicked_id, enabled)) = selection_request {
        if shift {
            if let Some(anchor_id) = selection_anchor.as_deref() {
                apply_range(items, anchor_id, &clicked_id, enabled);
            } else {
                *selection_anchor = Some(clicked_id);
            }
        } else {
            *selection_anchor = Some(clicked_id);
        }
    }

    // Reorder priority by index.
    if response.final_update().is_some() {
        items.par_iter_mut().enumerate().for_each(|(idx, item)| {
            item.priority = idx + 1;
        });
    }
}

/// This table is read-only for all fields except the `enabled` checkbox.
/// Useful for displaying filtered or sorted items where drag-and-drop is disabled.
pub(crate) fn check_only_table_body(
    body: &mut TableBody,
    filtered_ids: &[ModItem],
    original_items: &mut [ModItem],
    widths: [f32; 6],
    selection_anchor: &mut Option<String>,
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

    // is_shift, id, enabled
    let mut selection_request: Option<(bool, String, bool)> = None;

    for filtered_mod in filtered_ids {
        let Some(&mut &mut ModItem {
            ref id,
            ref name,
            ref mod_type,
            ref site,
            priority,
            ref mut enabled,
        }) = orig_map.get_mut(&filtered_mod.id)
        else {
            continue;
        };

        body.row(ROW_HEIGHT, |mut row| {
            row.col(|ui| {
                let response = ui.add_sized(checkbox_size, egui::Checkbox::without_text(enabled));

                if response.clicked() {
                    selection_request =
                        Some((ui.input(|input| input.modifiers.shift), id.clone(), *enabled));
                }
            });
            row.col(|ui| label_with_hover(ui, id, w_path, CellAlign::Left));
            row.col(|ui| label_with_hover(ui, name, w_name, CellAlign::Center));
            row.col(|ui| label_with_hover(ui, mod_type.as_str(), w_mod_type, CellAlign::Center));
            row.col(|ui| hyperlink_with_hover(ui, site, w_site));
            row.col(|ui| centered_ui(ui, |ui| ui.label(priority.to_string())));
        });
    }

    if let Some((shift, clicked_id, enabled)) = selection_request {
        if shift {
            if let Some(anchor_id) = selection_anchor.as_deref() {
                apply_filtered_range(filtered_ids, original_items, anchor_id, &clicked_id, enabled);
                *selection_anchor = None;
            } else {
                *selection_anchor = Some(clicked_id);
            }
        } else {
            *selection_anchor = Some(clicked_id);
        }
    }
}

fn apply_range(items: &mut [ModItem], anchor_id: &str, clicked_id: &str, enabled: bool) {
    let Some(anchor_index) = items.iter().position(|item| item.id == anchor_id) else {
        return;
    };

    let Some(clicked_index) = items.iter().position(|item| item.id == clicked_id) else {
        return;
    };

    let (start, end) = if anchor_index <= clicked_index {
        (anchor_index, clicked_index)
    } else {
        (clicked_index, anchor_index)
    };

    for item in &mut items[start..=end] {
        item.enabled = enabled;
    }
}

fn apply_filtered_range(
    filtered_items: &[ModItem],
    original_items: &mut [ModItem],
    anchor_id: &str,
    clicked_id: &str,
    enabled: bool,
) {
    let Some(anchor_index) = filtered_items.iter().position(|item| item.id == anchor_id) else {
        return;
    };

    let Some(clicked_index) = filtered_items.iter().position(|item| item.id == clicked_id) else {
        return;
    };

    let (start, end) = if anchor_index <= clicked_index {
        (anchor_index, clicked_index)
    } else {
        (clicked_index, anchor_index)
    };

    for filtered_item in &filtered_items[start..=end] {
        if let Some(item) = original_items.iter_mut().find(|item| item.id == filtered_item.id) {
            item.enabled = enabled;
        }
    }
}

fn centered_ui<R>(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui) -> R) {
    ui.with_layout(
        egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
        add_contents,
    );
}
