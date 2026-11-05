//! The rows of the mod table: draggable, or read-only with only the checkboxes active.

use egui_extras::TableBody;
use gui_shared::settings::pages::patch::mod_item::ModItem;

use super::label_ext::{CellAlign, ROW_HEIGHT, hyperlink_with_hover, label_with_hover};
use crate::{
    conflicts::{ConflictScan, Mark},
    visible_mods::{apply_range, apply_visible_range},
};

/// What the rows need for their conflict flag and right-click menu.
pub(super) struct RowConflicts<'a> {
    pub(super) scan: &'a ConflictScan,
    /// The menu's "show conflicts".
    pub(super) label: &'a str,
    /// The flag's hover: wins, loses.
    pub(super) hovers: (&'a str, &'a str),
    /// The hover of a mod whose patch does nothing in the end.
    pub(super) dead_hover: &'a str,
    /// The mod whose conflicts were asked for.
    pub(super) asked: Option<String>,
}

impl RowConflicts<'_> {
    /// `+`, `-` or `±`. Green: it overrides others, orange: others override it. `⚡`, like MO2: all it
    /// does loses, so its patch does nothing.
    fn flag(&self, ui: &mut egui::Ui, id: &str, width: f32) {
        let mark = self.scan.mark_of(id);
        if mark == Mark::default() {
            ui.allocate_exact_size(egui::vec2(width, ROW_HEIGHT), egui::Sense::hover());
            return;
        }
        let color = if mark.wins > 0 && mark.loses > 0 {
            ui.visuals().warn_fg_color
        } else if mark.loses > 0 {
            ui.visuals().error_fg_color
        } else {
            egui::Color32::from_rgb(100, 200, 120)
        };
        let (wins, loses) = self.hovers;
        let counts = format!("{wins}: {}\n{loses}: {}", mark.wins, mark.loses);
        let is_dead = self.scan.is_dead(id);
        let hover = if is_dead { format!("{}\n\n{counts}", self.dead_hover) } else { counts };
        // Not selectable: it would take the clicks, and with them the row's right-click menu.
        let text = if is_dead { "⚡" } else { mark.glyph() };
        let glyph = egui::Label::new(egui::RichText::new(text).color(color).strong()).selectable(false);
        ui.add_sized([width, ROW_HEIGHT], glyph).on_hover_text(hover);
    }

    fn menu(&mut self, response: &egui::Response, id: &str) {
        response.context_menu(|ui| {
            if ui.button(self.label).clicked() {
                self.asked = Some(id.to_owned());
                ui.close();
            }
        });
    }
}

/// A clicked checkbox: `(Shift held, mod id, its new state)`.
type CheckClick = (bool, String, bool);

/// Handle drag-and-drop reordering of mods and Shift-click range selection.
pub(super) fn dnd_table_body(
    body: &mut TableBody,
    items: &mut [ModItem],
    widths: [f32; 7],
    selection_anchor: &mut Option<String>,
    conflicts: &mut RowConflicts<'_>,
) {
    let ui = body.ui_mut();
    let checkbox_rect = [widths[0], ROW_HEIGHT];
    let w_flag = widths[1];
    let w_path = widths[2];
    let w_name = widths[3];
    let w_mod_type = widths[4];
    let w_site = widths[5];
    let priority_size = [widths[6], ROW_HEIGHT];

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
            // Under the row's widgets: a right click anywhere on the row.
            let row = ui.interact(row_rect, ui.scope_id().with("mod_row_menu"), egui::Sense::click());
            conflicts.menu(&row, &item.id);

            ui.scope_builder(egui::UiBuilder::new().max_rect(row_rect), |ui| {
                ui.horizontal(|ui| {
                    let response = ui.add_sized(checkbox_rect, egui::Checkbox::without_text(&mut item.enabled));

                    if response.clicked() {
                        click = Some((ui.input(|input| input.modifiers.shift), item.id.clone(), item.enabled));
                    }
                    conflicts.menu(&response, &item.id);

                    conflicts.flag(ui, &item.id, w_flag);
                    label_with_hover(ui, &item.id, w_path, CellAlign::Left);
                    draggable_handle.ui(ui, |ui| label_with_hover(ui, &item.name, w_name, CellAlign::Center));
                    label_with_hover(ui, item.mod_type.as_str(), w_mod_type, CellAlign::Center);
                    let site = hyperlink_with_hover(ui, &item.site, w_site);
                    conflicts.menu(&site, &item.id);
                    let priority = egui::Label::new(item.priority.to_string()).selectable(false);
                    centered_ui(ui, |ui| ui.add_sized(priority_size, priority));
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
    widths: [f32; 7],
    selection_anchor: &mut Option<String>,
    conflicts: &mut RowConflicts<'_>,
) {
    // The last fetch found nothing: `items` is empty, while `rows` may still be the old list's.
    if items.is_empty() {
        return;
    }
    let checkbox_size = [widths[0], ROW_HEIGHT];
    let w_flag = widths[1];
    let (w_path, w_name, w_mod_type, w_site) = (widths[2], widths[3], widths[4], widths[5]);
    let mut click: Option<CheckClick> = None;

    body.rows(ROW_HEIGHT, rows.len(), |mut row| {
        let Some(item) = rows.get(row.index()).and_then(|&index| items.get_mut(index as usize)) else { return };
        row.col(|ui| {
            let response = ui.add_sized(checkbox_size, egui::Checkbox::without_text(&mut item.enabled));
            if response.clicked() {
                click = Some((ui.input(|input| input.modifiers.shift), item.id.clone(), item.enabled));
            }
            conflicts.menu(&response, &item.id);
        });
        row.col(|ui| conflicts.flag(ui, &item.id, w_flag));
        row.col(|ui| label_with_hover(ui, &item.id, w_path, CellAlign::Left));
        row.col(|ui| label_with_hover(ui, &item.name, w_name, CellAlign::Center));
        row.col(|ui| label_with_hover(ui, item.mod_type.as_str(), w_mod_type, CellAlign::Center));
        row.col(|ui| {
            let site = hyperlink_with_hover(ui, &item.site, w_site);
            conflicts.menu(&site, &item.id);
        });
        row.col(|ui| {
            centered_ui(ui, |ui| ui.add(egui::Label::new(item.priority.to_string()).selectable(false)));
        });
        let response = row.response();
        conflicts.menu(&response, &item.id);
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
