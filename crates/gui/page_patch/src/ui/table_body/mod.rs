//! The rows of the mod table: draggable, or read-only with only the checkboxes active.
//!
//! ```text
//! [☑] ±  D:/mods/a/..  Paraglider  Nemesis  nexusmods.com/..  1
//!  ^check ^flag ^path   ^name       ^type    ^site             ^priority   <- [`Columns`]
//! ```
//! Both tables draw a row with the same cells, in that order ([`cells`]), each in a column: the
//! table's (`row.col`), or one of the draggable row's own ([`col`]). Only how a row is laid out
//! differs: the draggable one in a row of its own (`egui_dnd`), the read-only one in the table.
//! What the rows tell of the conflicts is [`conflicts`].

mod cells;
mod conflicts;

use gui_shared::settings::pages::patch::mod_item::ModItem;

pub(super) use self::conflicts::{Links, RowConflicts, paint_links};
use super::label_ext::ROW_HEIGHT;
use crate::visible_mods::{apply_range, apply_visible_range};

/// The widths of a row's cells, left to right. See the module docs.
#[derive(Debug, Clone, Copy)]
struct Columns {
    check: f32,
    flag: f32,
    path: f32,
    name: f32,
    mod_type: f32,
    site: f32,
    priority: f32,
}

impl Columns {
    /// From the table's widths, in the order of the module docs.
    const fn of([check, flag, path, name, mod_type, site, priority]: [f32; 7]) -> Self {
        Self { check, flag, path, name, mod_type, site, priority }
    }

    /// All the cells together.
    fn sum(self) -> f32 {
        [self.check, self.flag, self.path, self.name, self.mod_type, self.site, self.priority].iter().sum()
    }
}

/// A clicked checkbox: `(Shift held, mod id, its new state)`.
type CheckClick = (bool, String, bool);

/// The first and the last row drawn, by their index in the list, and where they are: where a row off
/// screen would be. See [`RowConflicts::place_picked_off_screen`].
type Drawn = ((usize, egui::Rect), (usize, egui::Rect));

/// A cell of the draggable row, as `TableRow::col` is one of the table's: the rows of both read
/// alike.
fn col<R>(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui) -> R) -> R {
    add_contents(ui)
}

/// Keeps `checked`, the last checkbox clicked, in `click`.
fn note_click(click: &mut Option<CheckClick>, checked: Option<CheckClick>) {
    if checked.is_some() {
        *click = checked;
    }
}

/// The rows of the mod table, and what they need: the same for both tables, drawn by
/// [`Self::draggable`] or [`Self::check_only`].
pub(super) struct TableBody<'a, 'c> {
    /// All the mods, by priority.
    pub(super) items: &'a mut [ModItem],
    /// The cells' widths: see [`Columns`].
    pub(super) widths: [f32; 7],
    /// The mod whose checkbox was clicked last: a Shift-click checks from it to the one clicked.
    pub(super) selection_anchor: &'a mut Option<String>,
    pub(super) conflicts: &'a mut RowConflicts<'c>,
}

impl TableBody<'_, '_> {
    /// Handle drag-and-drop reordering of mods and Shift-click range selection.
    pub(super) fn draggable(self, body: &mut egui_extras::TableBody) {
        /// The background of row `state.index`: every other one striped (egui's table does it, the
        /// drag list does not), the dragged one stronger.
        fn stripe(visuals: &egui::Visuals, state: egui_dnd::ItemState) -> egui::Color32 {
            if state.index.is_multiple_of(2) {
                visuals.widgets.active.bg_fill.gamma_multiply(0.5) // gray
            } else if state.dragged {
                visuals.widgets.active.bg_fill
            } else {
                egui::Color32::TRANSPARENT
            }
        }

        let Self { items, widths, selection_anchor, conflicts } = self;
        let columns = Columns::of(widths);
        // The cells, and the gaps between them.
        let row_size = egui::vec2(columns.sum() + 38.0, ROW_HEIGHT);
        let mut click: Option<CheckClick> = None;

        let dnd = egui_dnd::dnd(body.ui_mut(), egui::Id::unique("mod_list_dnd"));
        let response = dnd.show_vec(items, |ui, item, handle, state| {
            let row_rect = egui::Rect::from_min_size(ui.min_rect().min, row_size);
            let row_rect = ui.allocate_rect(row_rect, egui::Sense::hover()).rect;
            ui.painter().rect_filled(row_rect, egui::CornerRadius::ZERO, stripe(ui.visuals(), state));
            conflicts.paint_tint((ui.painter(), ui.visuals()), row_rect, &item.id);

            // Under the row's widgets: a right click anywhere on the row.
            let row = ui.interact(row_rect, ui.scope_id().with("mod_row_menu"), egui::Sense::click());
            conflicts.menu(&row, &item.id);
            conflicts.link_row(&row, row_rect, &item.id);

            ui.scope_builder(egui::UiBuilder::new().max_rect(row_rect), |ui| {
                ui.horizontal(|ui| {
                    col(ui, |ui| note_click(&mut click, cells::check(ui, item, columns.check, conflicts)));
                    col(ui, |ui| cells::flag(ui, item, columns.flag, conflicts));
                    col(ui, |ui| cells::path(ui, item, columns.path));
                    col(ui, |ui| handle.ui(ui, |ui| cells::name(ui, item, columns.name)));
                    col(ui, |ui| cells::mod_type(ui, item, columns.mod_type));
                    col(ui, |ui| cells::site(ui, item, columns.site, conflicts));
                    col(ui, |ui| cells::priority(ui, item, columns.priority));
                });
            });
        });

        // With Shift, the mods from the anchor to the one clicked take its state; else it is the anchor.
        if let Some((shift, id, enabled)) = click {
            match selection_anchor.as_deref() {
                Some(from) if shift => apply_range(items, from, &id, enabled),
                _ => *selection_anchor = Some(id),
            }
        }
        // Priorities by the new order: the first mod is `1`.
        if response.final_update().is_some() {
            for (index, item) in items.iter_mut().enumerate() {
                item.priority = index + 1;
            }
        }
    }

    /// The read-only view: only the `enabled` checkboxes respond, as the rows are filtered or sorted.
    ///
    /// `rows`: indices into `items`, in the order to show. Only the rows on screen are laid out.
    pub(super) fn check_only(self, body: egui_extras::TableBody, rows: &[u32]) {
        let Self { items, widths, selection_anchor, conflicts } = self;
        // The last fetch found nothing: `items` is empty, while `rows` may still be the old list's.
        if items.is_empty() {
            return;
        }

        let columns = Columns::of(widths);
        let mut click: Option<CheckClick> = None;
        let mut drawn: Option<Drawn> = None;

        body.rows(ROW_HEIGHT, rows.len(), |mut row| {
            let index = row.index();
            let Some(&at) = rows.get(index) else { return };
            let Some(item) = items.get_mut(at as usize) else { return };

            let mut row_rect = egui::Rect::NOTHING;
            row.col(|ui| {
                // The whole row, from its first cell: the cells' widths and the gaps between them.
                let cell = ui.max_rect();
                let gaps = ui.spacing().item_spacing.x * 6.0;
                row_rect = egui::Rect::from_min_size(cell.min, egui::vec2(columns.sum() + gaps, cell.height()));

                // The last row drawn so far, and the first if none was before.
                let this = (index, row_rect);
                drawn = Some((drawn.map_or(this, |(first, _)| first), this));

                // A painter over the whole row: the cell's own would clip it to the checkbox.
                let painter = egui::Painter::new(ui.ctx().clone(), ui.layer_id(), row_rect);
                conflicts.paint_tint((&painter, ui.visuals()), row_rect, &item.id);

                note_click(&mut click, cells::check(ui, item, columns.check, conflicts));
            });
            row.col(|ui| cells::flag(ui, item, columns.flag, conflicts));
            row.col(|ui| cells::path(ui, item, columns.path));
            row.col(|ui| cells::name(ui, item, columns.name));
            row.col(|ui| cells::mod_type(ui, item, columns.mod_type));
            row.col(|ui| cells::site(ui, item, columns.site, conflicts));
            row.col(|ui| cells::priority(ui, item, columns.priority));

            // A right click anywhere on the row.
            let row = row.response();
            conflicts.menu(&row, &item.id);
            conflicts.link_row(&row, row_rect, &item.id);
        });
        conflicts.place_picked_off_screen((rows, items), drawn);

        // As the draggable table's, over the rows shown; then no anchor.
        if let Some((shift, id, enabled)) = click {
            match selection_anchor.as_deref() {
                Some(from) if shift => {
                    apply_visible_range(rows, items, from, &id, enabled);
                    *selection_anchor = None;
                }
                _ => *selection_anchor = Some(id),
            }
        }
    }
}
