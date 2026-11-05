//! The rows of the mod table: draggable, or read-only with only the checkboxes active.

use egui_extras::TableBody;
use gui_shared::settings::pages::patch::mod_item::ModItem;

use super::label_ext::{CellAlign, ROW_HEIGHT, hyperlink_with_hover, label_with_hover};
use crate::visible_mods::{apply_range, apply_visible_range};

/// A clicked checkbox: `(Shift held, mod id, its new state)`.
type CheckClick = (bool, String, bool);

/// Handle drag-and-drop reordering of mods and Shift-click range selection.
pub(super) fn dnd_table_body(
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
    let mut click: Option<CheckClick> = None;

    let response =
        egui_dnd::dnd(ui, egui::Id::unique("mod_list_dnd")).show_vec(items, |ui, item, draggable_handle, state| {
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
                    let response = ui.add_sized(checkbox_rect, egui::Checkbox::without_text(&mut item.enabled));

                    if response.clicked() {
                        click = Some((ui.input(|input| input.modifiers.shift), item.id.clone(), item.enabled));
                    }

                    label_with_hover(ui, &item.id, w_path, CellAlign::Left);
                    draggable_handle.ui(ui, |ui| label_with_hover(ui, &item.name, w_name, CellAlign::Center));
                    label_with_hover(ui, item.mod_type.as_str(), w_mod_type, CellAlign::Center);
                    hyperlink_with_hover(ui, &item.site, w_site);
                    centered_ui(ui, |ui| ui.add_sized(priority_size, egui::Label::new(item.priority.to_string())));
                });
            });
        });

    if let Some((shift, clicked_id, enabled)) = click {
        match selection_anchor.as_deref() {
            Some(anchor) if shift => apply_range(items, anchor, &clicked_id, enabled),
            _ => *selection_anchor = Some(clicked_id),
        }
    }

    // Reorder priority by index.
    if response.final_update().is_some() {
        items.iter_mut().enumerate().for_each(|(idx, item)| {
            item.priority = idx + 1;
        });
    }
}

/// The read-only view: only the `enabled` checkboxes respond, as the rows are filtered or sorted.
///
/// `rows`: indices into `items`, in the order to show. Only the rows on screen are laid out.
pub(super) fn check_only_table_body(
    body: TableBody,
    rows: &[u32],
    items: &mut [ModItem],
    widths: [f32; 6],
    selection_anchor: &mut Option<String>,
) {
    // The last fetch found nothing: `items` is empty, while `rows` may still be the old list's.
    if items.is_empty() {
        return;
    }
    let checkbox_size = [widths[0], ROW_HEIGHT];
    let (w_path, w_name, w_mod_type, w_site) = (widths[1], widths[2], widths[3], widths[4]);
    let mut click: Option<CheckClick> = None;

    body.rows(ROW_HEIGHT, rows.len(), |mut row| {
        let Some(item) = rows.get(row.index()).and_then(|&index| items.get_mut(index as usize)) else { return };
        row.col(|ui| {
            let response = ui.add_sized(checkbox_size, egui::Checkbox::without_text(&mut item.enabled));
            if response.clicked() {
                click = Some((ui.input(|input| input.modifiers.shift), item.id.clone(), item.enabled));
            }
        });
        row.col(|ui| label_with_hover(ui, &item.id, w_path, CellAlign::Left));
        row.col(|ui| label_with_hover(ui, &item.name, w_name, CellAlign::Center));
        row.col(|ui| label_with_hover(ui, item.mod_type.as_str(), w_mod_type, CellAlign::Center));
        row.col(|ui| hyperlink_with_hover(ui, &item.site, w_site));
        row.col(|ui| centered_ui(ui, |ui| ui.label(item.priority.to_string())));
    });

    if let Some((shift, clicked_id, enabled)) = click {
        match selection_anchor.as_deref() {
            Some(anchor) if shift => {
                apply_visible_range(rows, items, anchor, &clicked_id, enabled);
                *selection_anchor = None;
            }
            _ => *selection_anchor = Some(clicked_id),
        }
    }
}

fn centered_ui<R>(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui) -> R) {
    ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), add_contents);
}
