//! What the rows of the mod table tell of the conflicts: each mod's flag, and, once a mod is picked,
//! what the others are to it, as MO2 colors the rows when a mod is selected.
//!
//! ```text
//!  ╭──● [☑] ±  Paraglider      <- the picked mod's row: the selection's color
//!  │    [☑] +  Flinching        <- green: the picked mod replaces its values
//!  ╰──● [☑] -  Precision        <- red: it replaces the picked mod's; orange: both
//!  ^ the lines in the gutter, thicker the more they meet
//! ```
//! See [`crate::conflicts::Relation`] for what is counted.

use egui_widgets::HoverMarkdown as _;
use gui_shared::settings::pages::patch::mod_item::ModItem;

use super::super::label_ext::ROW_HEIGHT;
use crate::conflicts::{ConflictScan, Mark, Relation};

/// What the rows need for their conflict flag and right-click menu.
pub(in crate::ui) struct RowConflicts<'a> {
    pub(in crate::ui) scan: &'a ConflictScan,
    /// The menu's "show conflicts".
    pub(in crate::ui) label: &'a str,
    /// The flag's hover: wins, loses, resolved in the user patch.
    pub(in crate::ui) hovers: (&'a str, &'a str, &'a str),
    /// The hover of a mod whose patch does nothing in the end.
    pub(in crate::ui) dead_hover: &'a str,
    /// The mod whose conflicts were asked for.
    pub(in crate::ui) asked: Option<String>,
    /// What the rows tell of the picked mod: see [`Links`].
    pub(in crate::ui) links: Links,
}

/// The rows drawn that are something to the picked mod, and the picked one's: what the lines join.
#[derive(Debug, Default)]
pub(in crate::ui) struct Links {
    pub(in crate::ui) picked: Option<egui::Rect>,
    pub(in crate::ui) others: Vec<(egui::Rect, Relation)>,
    /// The highest [`Relation::count`]: the strongest line.
    pub(in crate::ui) max: u32,
    /// A row clicked: its mod is picked, or put down.
    pub(in crate::ui) clicked: Option<String>,
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
/// replaces the picked mod's; orange: both; gray: only where the user patch decided.
fn relation_color(visuals: &egui::Visuals, relation: Relation) -> egui::Color32 {
    if relation.is_all_resolved() {
        return visuals.weak_text_color();
    }
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
pub(in crate::ui) fn paint_links(painter: &egui::Painter, visuals: &egui::Visuals, gutter: (f32, f32), links: &Links) {
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
    /// `+`, `-` or `±`. Green: it overrides others, orange: others override it. `✓` in gray: all its
    /// conflicts are resolved in the user patch. `⚡`, like MO2: all it does loses, so its patch does
    /// nothing.
    pub(super) fn flag(&self, ui: &mut egui::Ui, id: &str, width: f32) {
        let mark = self.scan.mark_of(id);
        if mark == Mark::default() {
            ui.allocate_exact_size(egui::vec2(width, ROW_HEIGHT), egui::Sense::hover());
            return;
        }

        let color = if mark.is_all_resolved() {
            ui.visuals().weak_text_color()
        } else {
            conflict_color(ui.visuals(), (mark.wins > 0, mark.loses > 0))
        };
        let is_dead = self.scan.is_dead(id);

        let (wins, loses, resolved) = self.hovers;
        let counts = format!("{wins}: {}\n{loses}: {}\n{resolved}: {}", mark.wins, mark.loses, mark.resolved);
        let hover = if is_dead { format!("{}\n\n{counts}", self.dead_hover) } else { counts };

        // Not selectable: it would take the clicks, and with them the row's right-click menu.
        let text = if is_dead { "⚡" } else { mark.glyph() };
        let glyph = egui::Label::new(egui::RichText::new(text).color(color).strong()).selectable(false);
        ui.add_sized([width, ROW_HEIGHT], glyph).on_hover_markdown(&hover);
    }

    /// Paints the background of mod `id`'s row at `rect` with `painter`: the picked mod's, or what it
    /// is to it. Nothing if it is neither.
    pub(super) fn paint_tint(&self, (painter, visuals): (&egui::Painter, &egui::Visuals), rect: egui::Rect, id: &str) {
        if let Some(tint) = self.tint(visuals, id) {
            painter.rect_filled(rect, egui::CornerRadius::ZERO, tint);
        }
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
    pub(super) fn link_row(&mut self, response: &egui::Response, rect: egui::Rect, id: &str) {
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

        let (wins, loses, resolved) = self.hovers;
        let (overrides, overridden) = (relation.overrides_picked, relation.overridden_by_picked);
        let hover = format!("{picked}\n{wins}: {overrides}\n{loses}: {overridden}\n{resolved}: {}", relation.resolved);
        response.clone().on_hover_text(hover);
    }

    /// The right-click menu of a part of mod `id`'s row: "show conflicts".
    pub(super) fn menu(&mut self, response: &egui::Response, id: &str) {
        response.context_menu(|ui| {
            if ui.button(self.label).clicked() {
                self.asked = Some(id.to_owned());
                ui.close();
            }
        });
    }

    /// The picked mod's row is off screen (only the rows on screen are laid out): where it would be,
    /// from the rows drawn, so that the lines still go toward it.
    ///
    /// `drawn`: the first and the last row drawn, by their index in `rows`, and their rects.
    pub(super) fn place_picked_off_screen(&mut self, (rows, items): (&[u32], &[ModItem]), drawn: Option<super::Drawn>) {
        if self.links.picked.is_some() || self.links.others.is_empty() {
            return;
        }

        let Some(picked) = self.scan.picked() else { return };
        let Some(((first, first_rect), (last, last_rect))) = drawn.filter(|((first, _), (last, _))| last > first)
        else {
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
        self.links.picked = Some(egui::Rect::from_min_size(egui::pos2(first_rect.left(), top), first_rect.size()));
    }
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
