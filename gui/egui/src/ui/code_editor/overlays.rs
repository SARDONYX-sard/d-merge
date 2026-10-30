//! Things drawn on top of the text: hover tooltip, signature help, inlay hints,
//! current-line tint and the Vim block cursor.
//!
//! None of them change the text or its layout, so they cannot break cursor mapping.

use egui::{Color32, Galley, Pos2};

use super::{
    language::{Diagnostic, Language},
    text_pos::LinePos,
};

/// Hover tooltip: diagnostics under the pointer + the language's hover Markdown.
///
/// `with_docs`: show [`Language::hover`] (diagnostic messages are shown regardless).
pub(super) fn hover_tooltip(
    response: &egui::Response,
    galley: &Galley,
    galley_pos: Pos2,
    text: &str,
    language: &dyn Language,
    diagnostics: &[Diagnostic],
    with_docs: bool,
) {
    let Some(pointer) = response.hover_pos() else { return };

    // Ignore the empty area right of a line: only real glyphs have hover info.
    let cursor = galley.cursor_from_pos(pointer - galley_pos);
    let glyph = galley.pos_from_cursor(cursor).translate(galley_pos.to_vec2());
    let near_glyph = (pointer.x - glyph.left()).abs() <= 40.0
        && glyph.expand2(egui::vec2(0.0, 2.0)).y_range().contains(pointer.y);
    if !near_glyph {
        return;
    }

    let pos = LinePos::from_char_index(text, cursor.index.0);
    let line = text.split('\n').nth(pos.line).unwrap_or_default();

    let problems: Vec<&Diagnostic> = diagnostics
        .iter()
        .filter(|d| {
            d.line == pos.line && d.columns.start <= pos.column && pos.column <= d.columns.end
        })
        .collect();
    let docs = if with_docs { language.hover(line, pos.column) } else { None };

    if problems.is_empty() && docs.is_none() {
        return;
    }

    response.clone().on_hover_ui_at_pointer(|ui| {
        ui.set_max_width(480.0);
        for problem in &problems {
            ui.colored_label(problem.severity.color(ui.visuals()), &problem.message);
        }
        if let Some(docs) = &docs {
            if !problems.is_empty() {
                ui.separator();
            }
            crate::ui::markdown::markdown(ui, docs);
        }
    });
}

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
                    crate::ui::markdown::markdown(ui, doc);
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
