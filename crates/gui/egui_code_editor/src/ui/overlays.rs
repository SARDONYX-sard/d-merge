//! Things drawn on top of the text: signature help, inlay hints,
//! current-line tint and the Vim block cursor. (The hover popup has state: see `hover`.)
//!
//! None of them change the text or its layout, so they cannot break cursor mapping.

use egui::{Color32, Galley, Pos2};

use crate::{
    inline_diff::{CharChange, InlineDiff},
    language::Language,
    text_pos::LinePos,
};

/// Signature help box just above the cursor line.
///
/// e.g., `animmotion <x: f32> <y: f32> <z: f32>` with the active parameter emphasized.
pub(crate) fn signature_box(
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
                        ui.label(if i == help.active { text.strong().underline() } else { text.weak() });
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
pub(crate) fn inlay_hints(ui: &egui::Ui, galley: &Galley, galley_pos: Pos2, language: &dyn Language, text: &str) {
    let painter = ui.painter();
    let font = egui::TextStyle::Monospace.resolve(ui.style());
    let color = ui.visuals().weak_text_color().gamma_multiply(0.8);

    // Lines are never wrapped (see `layout`), so row `n` is line `n`.
    for (line_no, (row, line)) in galley.rows.iter().zip(text.split('\n')).enumerate() {
        let hints = language.inlay_hints_at(line_no, line);
        if hints.is_empty() {
            continue;
        }
        let pos = galley_pos + row.rect().right_top().to_vec2() + egui::vec2(24.0, 0.0);
        painter.text(pos, egui::Align2::LEFT_TOP, hints.join(" · "), font.clone(), color);
    }
}

/// Width of the line number gutter for `text`: room for its largest number (at least 3 digits).
pub(crate) fn gutter_width(ui: &egui::Ui, text: &str) -> f32 {
    let lines = text.split('\n').count();
    let digits = lines.to_string().len().max(3);
    let font = egui::TextStyle::Monospace.resolve(ui.style());
    let digit_width = ui.fonts_mut(|f| f.glyph_width(&font, '0'));
    #[expect(clippy::cast_precision_loss, reason = "a few digits")]
    let width = digits as f32 * digit_width;
    width + 16.0 // padding on both sides
}

/// Line numbers right-aligned in the gutter `x_range`; the cursor line is brighter.
pub(crate) fn line_numbers(
    ui: &egui::Ui,
    galley: &Galley,
    galley_pos: Pos2,
    x_range: std::ops::Range<f32>,
    current_line: Option<usize>,
) {
    let font = egui::TextStyle::Monospace.resolve(ui.style());
    let visuals = ui.visuals();
    let (weak, strong) = (visuals.weak_text_color(), visuals.text_color());
    let clip = ui.clip_rect();
    let painter = ui.painter();

    // Lines are never wrapped (see `layout`), so row `n` is line `n`.
    for (line, row) in galley.rows.iter().enumerate() {
        let rect = row.rect().translate(galley_pos.to_vec2());
        if rect.bottom() < clip.top() || rect.top() > clip.bottom() {
            continue; // only the visible ones
        }
        let color = if current_line == Some(line) { strong } else { weak };
        let pos = egui::pos2(x_range.end - 8.0, rect.center().y);
        painter.text(pos, egui::Align2::RIGHT_CENTER, (line + 1).to_string(), font.clone(), color);
    }
}

/// The inline diff: a green tint on new lines, the old lines in red in their gaps, what differs
/// darker, and `↶` at each place. Returns the place whose `↶` was clicked.
/// See [`crate::inline_diff`].
pub(crate) fn inline_diff(
    ui: &egui::Ui,
    galley: &Galley,
    galley_pos: Pos2,
    diff: &InlineDiff,
    id: egui::Id,
    hover: &str,
) -> Option<usize> {
    let row_rect = |row: usize| galley.rows.get(row).map(|r| r.rect().translate(galley_pos.to_vec2()));
    let height = galley.rows.first().map_or(0.0, |row| row.rect().height());
    let mut clicked = None;
    for (index, hunk) in diff.hunks.iter().enumerate() {
        #[expect(clippy::cast_precision_loss, reason = "line counts are small")]
        let old_height = hunk.old_lines.len() as f32 * height;
        // In the gap above its first new line, or under the last line.
        let top = row_rect(hunk.new_lines.start).map_or_else(
            || galley.rows.last().map_or(galley_pos.y, |r| r.rect().bottom() + galley_pos.y),
            |rect| rect.top() - old_height,
        );
        for (i, line) in hunk.new_lines.clone().enumerate() {
            if let Some(rect) = row_rect(line) {
                draw_new_line(ui, rect, galley_pos.x, hunk.changes.get(i), text_at(galley, line));
            }
        }
        for (i, line) in hunk.old_lines.iter().enumerate() {
            #[expect(clippy::cast_precision_loss, reason = "line counts are small")]
            let y = (i as f32).mul_add(height, top);
            draw_old_line(ui, (galley_pos.x, y, height), line, hunk.changes.get(i));
        }
        let button = egui::Rect::from_min_size(egui::pos2(galley_pos.x - height, top), egui::vec2(height, height));
        if revert_button(ui, button, id.with(("inline_diff_revert", index)), hover) {
            clicked = Some(index);
        }
    }
    clicked
}

/// The text of a row, for finding on-screen columns.
fn text_at(galley: &Galley, row: usize) -> String {
    galley.rows.get(row).map_or_else(String::new, |placed| placed.row.glyphs.iter().map(|glyph| glyph.chr).collect())
}

/// A rect from char `chars.start` to `chars.end` of `line`, by on-screen columns (monospace).
fn chars_rect(
    ui: &egui::Ui,
    line: &str,
    chars: &std::ops::Range<usize>,
    left: f32,
    y: std::ops::RangeInclusive<f32>,
) -> egui::Rect {
    let font = egui::TextStyle::Monospace.resolve(ui.style());
    let width = ui.fonts_mut(|fonts| fonts.glyph_width(&font, '0'));
    #[expect(clippy::cast_precision_loss, reason = "columns are small")]
    let x = |char: usize| (crate::status::visual_column(line, char) as f32).mul_add(width, left);
    egui::Rect::from_x_y_ranges(x(chars.start)..=x(chars.end), y)
}

fn draw_new_line(ui: &egui::Ui, rect: egui::Rect, left: f32, change: Option<&CharChange>, text: String) {
    let clip = ui.clip_rect();
    if !rect.intersects(clip) {
        return;
    }
    let green = Color32::from_rgb(46, 160, 67);
    let across = egui::Rect::from_x_y_ranges(clip.x_range(), rect.y_range());
    ui.painter().rect_filled(across, 0.0, green.gamma_multiply(0.18));
    if let Some(change) = change {
        let chars = chars_rect(ui, &text, &change.new, left, rect.y_range().into());
        ui.painter().rect_filled(chars, 0.0, green.gamma_multiply(0.3));
    }
}

fn draw_old_line(ui: &egui::Ui, (left, y, height): (f32, f32, f32), line: &str, change: Option<&CharChange>) {
    let clip = ui.clip_rect();
    let across = egui::Rect::from_x_y_ranges(clip.x_range(), y..=y + height);
    if !across.intersects(clip) {
        return;
    }
    let red = Color32::from_rgb(248, 81, 73);
    let painter = ui.painter();
    painter.rect_filled(across, 0.0, red.gamma_multiply(0.18));
    if let Some(change) = change {
        painter.rect_filled(chars_rect(ui, line, &change.old, left, y..=y + height), 0.0, red.gamma_multiply(0.3));
    }
    let font = egui::TextStyle::Monospace.resolve(ui.style());
    let color = ui.visuals().weak_text_color();
    painter.text(egui::pos2(left, y), egui::Align2::LEFT_TOP, line.replace('\t', "    "), font, color);
}

/// `↶` in the gutter. Returns whether it was clicked.
fn revert_button(ui: &egui::Ui, rect: egui::Rect, id: egui::Id, hover: &str) -> bool {
    if !rect.intersects(ui.clip_rect()) {
        return false;
    }
    let response = ui.interact(rect, id, egui::Sense::click()).on_hover_text(hover);
    let color = if response.hovered() { ui.visuals().strong_text_color() } else { ui.visuals().weak_text_color() };
    let font = egui::TextStyle::Monospace.resolve(ui.style());
    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, "↶", font, color);
    response.clicked()
}

/// Faint background behind the cursor line.
pub(crate) fn current_line(ui: &egui::Ui, galley: &Galley, galley_pos: Pos2, line: usize) {
    let Some(row) = galley.rows.get(line) else { return };
    let rect = row.rect().translate(galley_pos.to_vec2());
    let rect = egui::Rect::from_x_y_ranges(ui.clip_rect().x_range(), rect.y_range());
    // A faint tint of the text color works for both dark and light themes.
    let tint: Color32 = ui.visuals().text_color().gamma_multiply(0.06);
    ui.painter().rect_filled(rect, 0.0, tint);
}

/// Indent rainbow + whitespace marks, on the visible rows only.
///
/// ```text
/// →   →   <hkparam ...>      tabs: `→`, spaces: `·`
/// ████████                   leading indent: one tint per level (a tab, or 4 spaces)
/// ```
pub(crate) fn whitespace(ui: &egui::Ui, galley: &Galley, galley_pos: Pos2) {
    // Yellow, green, purple, blue: the indent-rainbow palette, very faint.
    const RAINBOW: [Color32; 4] = [
        Color32::from_rgb(255, 255, 64),
        Color32::from_rgb(127, 255, 127),
        Color32::from_rgb(255, 127, 255),
        Color32::from_rgb(79, 236, 236),
    ];
    let clip = ui.clip_rect();
    let painter = ui.painter();
    let mark_color = ui.visuals().weak_text_color().gamma_multiply(0.6);
    let font = egui::TextStyle::Monospace.resolve(ui.style());
    // Laid out once and reused: there are hundreds of marks on screen, every frame.
    let tab_mark = painter.layout_no_wrap("→".to_string(), font.clone(), mark_color);
    let space_mark = painter.layout_no_wrap("·".to_string(), font, mark_color);

    for placed in &galley.rows {
        let row_rect = placed.rect().translate(galley_pos.to_vec2());
        if row_rect.bottom() < clip.top() || row_rect.top() > clip.bottom() {
            continue;
        }
        let (mut leading, mut spaces) = (true, 0_usize);
        let mut level = 0_usize;
        for glyph in &placed.row.glyphs {
            let (is_tab, is_space) = (glyph.chr == '\t', glyph.chr == ' ');
            if !is_tab && !is_space {
                leading = false;
                continue;
            }
            let left = row_rect.left() + glyph.pos.x;
            let rect = egui::Rect::from_x_y_ranges(left..=left + glyph.advance_width, row_rect.y_range());
            if leading {
                painter.rect_filled(rect, 0.0, RAINBOW[level % RAINBOW.len()].gamma_multiply(0.07));
                if is_tab {
                    level += 1;
                    spaces = 0;
                } else {
                    spaces += 1;
                    if spaces == 4 {
                        level += 1;
                        spaces = 0;
                    }
                }
            }
            let mark = if is_tab { &tab_mark } else { &space_mark };
            painter.galley(rect.center() - mark.size() / 2.0, std::sync::Arc::clone(mark), mark_color);
        }
    }
}

/// A rectangular selection (Vim `Ctrl+V`): one char range per line, drawn like a selection.
pub(crate) fn block_selection(ui: &egui::Ui, galley: &Galley, galley_pos: Pos2, ranges: &[std::ops::Range<usize>]) {
    // Painted over the text (unlike egui's own selection), so it must stay see-through.
    let fill = ui.visuals().selection.bg_fill.gamma_multiply(0.35);
    for range in ranges.iter().filter(|r| !r.is_empty()) {
        let start = galley.pos_from_cursor(egui::text::CCursor::new(range.start));
        let end = galley.pos_from_cursor(egui::text::CCursor::new(range.end));
        let rect = egui::Rect::from_min_max(start.left_top(), egui::pos2(end.left(), start.bottom()));
        ui.painter().rect_filled(rect.translate(galley_pos.to_vec2()), 0.0, fill);
    }
}

/// Vim's block cursor (Normal / Visual mode) over the char at `index`.
pub(crate) fn block_cursor(ui: &egui::Ui, galley: &Galley, galley_pos: Pos2, index: usize) {
    let left = galley.pos_from_cursor(egui::text::CCursor::new(index));
    let right = galley.pos_from_cursor(egui::text::CCursor::new(index + 1));

    // On an empty line (or at `\n`) the next cursor is on another row: use a space-wide block.
    let space_width = egui::TextStyle::Monospace.resolve(ui.style()).size * 0.6;
    let width = if (right.top() - left.top()).abs() < 1.0 && right.left() > left.left() {
        right.left() - left.left()
    } else {
        space_width
    };

    let rect =
        egui::Rect::from_min_size(left.left_top(), egui::vec2(width, left.height())).translate(galley_pos.to_vec2());
    let color = ui.visuals().text_cursor.stroke.color;
    ui.painter().rect_filled(rect, 0.0, color.gamma_multiply(0.5));
}
