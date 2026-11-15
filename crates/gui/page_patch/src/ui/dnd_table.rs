//! The rows of the mod table: draggable, or read-only with only the checkboxes active.

use egui_extras::TableBody;
use gui_shared::settings::pages::patch::mod_item::ModItem;

use super::label_ext::{CellAlign, ROW_HEIGHT, hyperlink_with_hover, label_with_hover};
use crate::{
    conflicts::{ConflictScan, Mark, Relation},
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
    /// What the rows tell of the picked mod: see [`Links`].
    pub(super) links: Links,
}

/// The rows drawn that are something to the picked mod, and the picked one's: what the lines join,
/// like MO2's colors when a mod is selected. See [`crate::conflicts::Relation`].
#[derive(Debug, Default)]
pub(super) struct Links {
    pub(super) picked: Option<egui::Rect>,
    pub(super) others: Vec<(egui::Rect, Relation)>,
    /// The highest [`Relation::count`]: the strongest line.
    pub(super) max: u32,
    /// A row clicked: its mod is picked, or put down.
    pub(super) clicked: Option<String>,
}

/// "Overrides": the `+` flag, and a row whose values the picked mod replaces. Red and orange are the
/// theme's (`error_fg_color`, `warn_fg_color`).
const OVERRIDES_COLOR: egui::Color32 = egui::Color32::from_rgb(100, 200, 120);

/// The picked row's background: the selection color, this strong.
const PICKED_TINT: f32 = 0.6;
/// The radius of the dot at the picked row's end of the lines.
const PICKED_DOT: f32 = 3.5;

// `(weakest, strongest)`: what a relation of `strength` 0.0 and 1.0 get. See `scaled`.
/// A row's background, the alpha of its color.
const ROW_TINT: (f32, f32) = (0.12, 0.5);
/// A line's alpha.
const LINE_ALPHA: (f32, f32) = (0.4, 1.0);
/// A line's width.
const LINE_WIDTH: (f32, f32) = (1.0, 3.5);
/// The radius of the dot at the other row's end.
const LINE_DOT: (f32, f32) = (1.5, 3.0);

/// How far apart two rows are when their line bends out to the gutter's left edge. Nearer, it bends
/// less (down to `LINE_BEND_MIN` of the gutter), so that the lines do not lie on one another.
const LINE_BEND_DISTANCE: f32 = 400.0;
const LINE_BEND_MIN: f32 = 0.3;

/// The color of a mod's conflicts: green where only `green`, red where only `red`, orange where both.
const fn conflict_color(visuals: &egui::Visuals, (green, red): (bool, bool)) -> egui::Color32 {
    match (green, red) {
        (true, true) => visuals.warn_fg_color,
        (true, false) => OVERRIDES_COLOR,
        (false, true) => visuals.error_fg_color,
        (false, false) => egui::Color32::TRANSPARENT,
    }
}

/// The color of what a mod is to the picked one. Green: the picked mod replaces its values; red: it
/// replaces the picked mod's; orange: both.
const fn relation_color(visuals: &egui::Visuals, relation: Relation) -> egui::Color32 {
    conflict_color(visuals, (relation.overridden_by_picked > 0, relation.overrides_picked > 0))
}

/// How strong `count` is among counts up to `max`, `0.0..=1.0`. Logarithmic: one conflict is seen,
/// and a thousand do not drown ten.
fn strength(count: u32, max: u32) -> f32 {
    if max == 0 {
        return 0.0;
    }

    #[expect(clippy::cast_precision_loss, reason = "a count, far below 2^24")]
    let (count, max) = (count as f32, max as f32);
    (count.ln_1p() / max.ln_1p()).clamp(0.0, 1.0)
}

/// The value of `(weakest, strongest)` for `strength`.
fn scaled((weakest, strongest): (f32, f32), strength: f32) -> f32 {
    (strongest - weakest).mul_add(strength, weakest)
}

/// The lines from the picked mod's row to the others, in the gutter left of the table (`gutter`: its
/// left and right x). Thicker and stronger the more they meet, in the colors of [`relation_color`].
pub(super) fn paint_links(painter: &egui::Painter, visuals: &egui::Visuals, gutter: (f32, f32), links: &Links) {
    let Some(picked) = links.picked else { return };
    let (left, right) = gutter;
    let from = egui::pos2(right, picked.center().y);

    for &(rect, relation) in &links.others {
        let strength = strength(relation.count(), links.max);
        let color = relation_color(visuals, relation).gamma_multiply(scaled(LINE_ALPHA, strength));
        let stroke = egui::Stroke::new(scaled(LINE_WIDTH, strength), color);

        let to = egui::pos2(right, rect.center().y);
        let curve = {
            let far = ((to.y - from.y).abs() / LINE_BEND_DISTANCE).clamp(LINE_BEND_MIN, 1.0);
            let bend = egui::lerp(right..=left, far);
            let points = [from, egui::pos2(bend, from.y), egui::pos2(bend, to.y), to];
            egui::epaint::CubicBezierShape::from_points_stroke(points, false, egui::Color32::TRANSPARENT, stroke)
        };
        painter.add(curve);
        painter.circle_filled(to, scaled(LINE_DOT, strength), color);
    }

    painter.circle_filled(from, PICKED_DOT, visuals.selection.stroke.color);
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

        let color = conflict_color(ui.visuals(), (mark.wins > 0, mark.loses > 0));
        let is_dead = self.scan.is_dead(id);

        let (wins, loses) = self.hovers;
        let counts = format!("{wins}: {}\n{loses}: {}", mark.wins, mark.loses);
        let hover = if is_dead { format!("{}\n\n{counts}", self.dead_hover) } else { counts };

        // Not selectable: it would take the clicks, and with them the row's right-click menu.
        let text = if is_dead { "⚡" } else { mark.glyph() };
        let glyph = egui::Label::new(egui::RichText::new(text).color(color).strong()).selectable(false);
        ui.add_sized([width, ROW_HEIGHT], glyph).on_hover_text(hover);
    }

    /// The background of a row: the picked mod's, or what it is to it. `None`: none of them.
    fn tint(&self, visuals: &egui::Visuals, id: &str) -> Option<egui::Color32> {
        if self.scan.picked() == Some(id) {
            return Some(visuals.selection.bg_fill.gamma_multiply(PICKED_TINT));
        }

        let (relation, max) = self.scan.relation_of(id);
        if relation.count() == 0 {
            return None;
        }
        let strength = strength(relation.count(), max);
        Some(relation_color(visuals, relation).gamma_multiply(scaled(ROW_TINT, strength)))
    }

    /// A row of mod `id` at `rect`: a click on it picks the mod; noted for the lines if it is the picked
    /// one or something to it, with how much on hover.
    fn link_row(&mut self, response: &egui::Response, rect: egui::Rect, id: &str) {
        if response.clicked() {
            self.links.clicked = Some(id.to_owned());
        }

        let Some(picked) = self.scan.picked() else { return };
        if picked == id {
            self.links.picked = Some(rect);
            return;
        }

        let (relation, max) = self.scan.relation_of(id);
        if relation.count() == 0 {
            return;
        }
        self.links.others.push((rect, relation));
        self.links.max = max;

        let (wins, loses) = self.hovers;
        let (overrides, overridden) = (relation.overrides_picked, relation.overridden_by_picked);
        response.clone().on_hover_text(format!("{picked}\n{wins}: {overrides}\n{loses}: {overridden}"));
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
            if let Some(tint) = conflicts.tint(ui.visuals(), &item.id) {
                ui.painter().rect_filled(row_rect, egui::CornerRadius::ZERO, tint);
            }
            // Under the row's widgets: a right click anywhere on the row.
            let row = ui.interact(row_rect, ui.scope_id().with("mod_row_menu"), egui::Sense::click());
            conflicts.menu(&row, &item.id);
            conflicts.link_row(&row, row_rect, &item.id);

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
    // The first and the last row drawn: where a row off screen would be. See `place_picked_off_screen`.
    let mut drawn: Option<((usize, egui::Rect), (usize, egui::Rect))> = None;

    body.rows(ROW_HEIGHT, rows.len(), |mut row| {
        let index = row.index();
        let Some(item) = rows.get(index).and_then(|&index| items.get_mut(index as usize)) else { return };

        let mut row_rect = egui::Rect::NOTHING;
        row.col(|ui| {
            row_rect = row_rect_from(ui, &widths);
            drawn = Some(drawn.map_or(((index, row_rect), (index, row_rect)), |(first, _)| (first, (index, row_rect))));
            if let Some(tint) = conflicts.tint(ui.visuals(), &item.id) {
                // A painter over the whole row: the cell's own would clip it to the checkbox.
                egui::Painter::new(ui.ctx().clone(), ui.layer_id(), row_rect).rect_filled(row_rect, 0.0, tint);
            }

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
        conflicts.link_row(&response, row_rect, &item.id);
    });
    place_picked_off_screen(conflicts, (rows, items), drawn);

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

/// The rect of the whole row whose first cell `ui` is: the cells' widths and the gaps between them.
fn row_rect_from(ui: &egui::Ui, widths: &[f32; 7]) -> egui::Rect {
    let cell = ui.max_rect();
    let gaps = ui.spacing().item_spacing.x * 6.0;
    egui::Rect::from_min_size(cell.min, egui::vec2(widths.iter().sum::<f32>() + gaps, cell.height()))
}

/// The picked mod's row is off screen (only the rows on screen are laid out): where it would be, from
/// the rows drawn, so that the lines still go toward it.
fn place_picked_off_screen(
    conflicts: &mut RowConflicts<'_>,
    (rows, items): (&[u32], &[ModItem]),
    drawn: Option<((usize, egui::Rect), (usize, egui::Rect))>,
) {
    if conflicts.links.picked.is_some() || conflicts.links.others.is_empty() {
        return;
    }

    let Some(picked) = conflicts.scan.picked() else { return };
    let Some(((first, first_rect), (last, last_rect))) = drawn.filter(|((first, _), (last, _))| last > first) else {
        return;
    };
    let Some(at) = rows.iter().position(|&index| items.get(index as usize).is_some_and(|item| item.id == picked))
    else {
        return;
    };

    let top = {
        #[expect(clippy::cast_precision_loss, reason = "a row index, far below 2^24")]
        let pitch = (last_rect.top() - first_rect.top()) / (last - first) as f32;
        #[expect(clippy::cast_precision_loss, reason = "a row index, far below 2^24")]
        let rows_away = at as f32 - first as f32;
        rows_away.mul_add(pitch, first_rect.top())
    };
    conflicts.links.picked = Some(egui::Rect::from_min_size(egui::pos2(first_rect.left(), top), first_rect.size()));
}

fn centered_ui<R>(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui) -> R) {
    ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), add_contents);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One conflict is seen, and a thousand do not drown ten.
    #[test]
    fn strength_is_logarithmic() {
        assert!(strength(0, 0).abs() < f32::EPSILON, "nothing picked");
        assert!((strength(1000, 1000) - 1.0).abs() < f32::EPSILON);
        let (one, ten) = (strength(1, 1000), strength(10, 1000));
        assert!(one > 0.05 && ten > 0.3 && ten < 1.0, "{one} {ten}");
    }
}
