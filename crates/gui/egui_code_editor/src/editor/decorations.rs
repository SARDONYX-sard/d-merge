//! Step 4 of a frame: what is drawn beside and under the text, and scrolling.

use crate::{
    Keymap,
    cursor::scroll_request_id,
    keymap::{Scroll, View},
    state::CodeEditorState,
    text_pos::LinePos,
    ui::overlays,
};

/// The line numbers, and the rectangle of Vim's Visual block.
pub(crate) fn draw_line_decorations(
    ui: &egui::Ui,
    state: &CodeEditorState,
    keymap: Keymap,
    text: &str,
    (galley, galley_pos): (&egui::Galley, egui::Pos2),
    gutter: Option<std::ops::Range<f32>>,
    cursor: Option<LinePos>,
) {
    if let Some(x_range) = gutter {
        overlays::line_numbers(ui, galley, galley_pos, x_range, cursor.map(|cursor| cursor.line));
    }
    if let Some(ranges) = state.keymap.block_selection(keymap, text) {
        overlays::block_selection(ui, galley, galley_pos, &ranges);
    }
}

/// Scrolls for a cursor moved from outside the `TextEdit` (it only scrolls for its own moves):
/// - requested by [`set_cursor`] (e.g., a click in a problems list): centers it
/// - moved by the keymap (e.g., Vim `G`, `n`): just brings it into view
pub(crate) fn scroll_to_cursor(
    ui: &egui::Ui,
    id: egui::Id,
    cursor: egui::Rect,
    galley: &egui::Galley,
    scroll: Option<Scroll>,
) {
    if ui.data_mut(|d| d.remove_temp::<bool>(scroll_request_id(id))).unwrap_or(false) {
        ui.scroll_to_rect(cursor, Some(egui::Align::Center));
        return;
    }
    match scroll {
        // Keymap moves (Vim `gg`, `G`, `zz`, ...) jump at once: an animation is slow over a long
        // distance.
        Some(Scroll::IntoView) => ui.scroll_to_rect_animation(cursor, None, instant()),
        Some(Scroll::Align(align)) => ui.scroll_to_rect_animation(cursor, Some(align), instant()),
        Some(Scroll::Lines(lines)) => {
            let row_height = galley.rows.first().map_or(16.0, |row| row.rect().height());
            #[expect(clippy::cast_precision_loss, reason = "a few lines")]
            ui.scroll_with_delta(egui::vec2(0.0, -(lines as f32) * row_height));
        }
        None => {}
    }
}

/// What goes behind the text: the cursor line's tint and the whitespace marks.
pub(crate) fn draw_text_background(
    ui: &egui::Ui,
    (galley, galley_pos): (&egui::Galley, egui::Pos2),
    cursor_line: Option<usize>,
    whitespace: bool,
) {
    if let Some(line) = cursor_line {
        overlays::current_line(ui, galley, galley_pos, line);
    }
    if whitespace {
        overlays::whitespace(ui, galley, galley_pos);
    }
}

/// The lines of `galley` **fully** inside the visible area (lines are never wrapped: row = line).
///
/// Half-visible rows do not count: Vim `L` on one would scroll the view to show it, so `H` / `L`
/// pressed in turn would creep down a line each time.
pub(crate) fn visible_lines(ui: &egui::Ui, galley: &egui::Galley, galley_pos: egui::Pos2) -> View {
    const EPS: f32 = 0.5; // rounding of the scroll offset
    let clip = ui.clip_rect();
    let rows = &galley.rows;
    let first = rows.iter().position(|r| galley_pos.y + r.rect().min.y >= clip.top() - EPS).unwrap_or(0);
    let last =
        rows.iter().rposition(|r| galley_pos.y + r.rect().max.y <= clip.bottom() + EPS).unwrap_or(first).max(first);
    View { first_line: first, lines: last - first + 1 }
}

/// No scroll animation.
pub(crate) fn instant() -> egui::style::ScrollAnimation {
    egui::style::ScrollAnimation::none()
}
