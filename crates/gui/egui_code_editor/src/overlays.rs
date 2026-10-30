//! Things drawn on top of the text: signature help, inlay hints,
//! current-line tint and the Vim block cursor. (The hover popup has state: see `hover`.)
//!
//! None of them change the text or its layout, so they cannot break cursor mapping.

use egui::{Color32, Galley, Pos2};

use super::{language::Language, text_pos::LinePos};

/// Signature help box just above the cursor line.
///
/// e.g., `animmotion <x: f32> <y: f32> <z: f32>` with the active parameter emphasized.
pub(super) fn signature_box(
    ctx: &egui::Context,
    id: egui::Id,
    cursor_rect: egui::Rect,
    language: &dyn Language,
    text: &str,
    cursor: LinePos,
) {
    let line = text.split('\n').nth(cursor.line).unwrap_or_default();
    let Some(help) = language.signature_help(line, cursor.column) else { return };

    egui::Area::new(id.with("signature_help"))
        .order(egui::Order::Tooltip)
        .pivot(egui::Align2::LEFT_BOTTOM)
        .fixed_pos(cursor_rect.left_top() - egui::vec2(0.0, 2.0))
        .interactable(false)
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.label(egui::RichText::new(&help.label).monospace());
                ui.horizontal(|ui| {
                    for (i, param) in help.params.iter().enumerate() {
                        let text = egui::RichText::new(param).monospace();
                        ui.label(if i == help.active {
                            text.strong().underline()
                        } else {
                            text.weak()
                        });
                    }
                });
                if let Some(doc) = &help.documentation {
                    ui.separator();
                    ui.set_max_width(420.0);
                    egui_widgets::markdown::markdown(ui, doc);
                }
            });
        });
}

/// Inlay hints, drawn faintly after the end of each line.
///
/// e.g., `0.5 animmotion 1 2 3      time · event · x · y · z`
///
/// Real inline hints would need virtual text inside the galley, which breaks egui's
/// char-index <-> position mapping, so they are shown at the line end instead.
pub(super) fn inlay_hints(
    ui: &egui::Ui,
    galley: &Galley,
    galley_pos: Pos2,
    language: &dyn Language,
    text: &str,
) {
    let painter = ui.painter();
    let font = egui::TextStyle::Monospace.resolve(ui.style());
    let color = ui.visuals().weak_text_color().gamma_multiply(0.8);

    // Lines are never wrapped (see `layout`), so row `n` is line `n`.
    for (row, line) in galley.rows.iter().zip(text.split('\n')) {
        let hints = language.inlay_hints(line);
        if hints.is_empty() {
            continue;
        }
        let pos = galley_pos + row.rect().right_top().to_vec2() + egui::vec2(24.0, 0.0);
        painter.text(pos, egui::Align2::LEFT_TOP, hints.join(" · "), font.clone(), color);
    }
}

/// Faint background behind the cursor line.
pub(super) fn current_line(ui: &egui::Ui, galley: &Galley, galley_pos: Pos2, line: usize) {
    let Some(row) = galley.rows.get(line) else { return };
    let rect = row.rect().translate(galley_pos.to_vec2());
    let rect = egui::Rect::from_x_y_ranges(ui.clip_rect().x_range(), rect.y_range());
    // A faint tint of the text color works for both dark and light themes.
    let tint: Color32 = ui.visuals().text_color().gamma_multiply(0.06);
    ui.painter().rect_filled(rect, 0.0, tint);
}

/// Vim's block cursor (Normal / Visual mode) over the char at `index`.
pub(super) fn block_cursor(ui: &egui::Ui, galley: &Galley, galley_pos: Pos2, index: usize) {
    let left = galley.pos_from_cursor(egui::text::CCursor::new(index));
    let right = galley.pos_from_cursor(egui::text::CCursor::new(index + 1));

    // On an empty line (or at `\n`) the next cursor is on another row: use a space-wide block.
    let space_width = egui::TextStyle::Monospace.resolve(ui.style()).size * 0.6;
    let width = if (right.top() - left.top()).abs() < 1.0 && right.left() > left.left() {
        right.left() - left.left()
    } else {
        space_width
    };

    let rect = egui::Rect::from_min_size(left.left_top(), egui::vec2(width, left.height()))
        .translate(galley_pos.to_vec2());
    let color = ui.visuals().text_cursor.stroke.color;
    ui.painter().rect_filled(rect, 0.0, color.gamma_multiply(0.5));
}
