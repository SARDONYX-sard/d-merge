//! Draws the patch preview. The data is in [`crate::patch::preview`].

use crate::patch::preview::{Kind, Preview};

/// Where the text starts: the `↶` gutter is left of it.
const TEXT_LEFT: f32 = 24.0;

/// `▲ ▼ 2/5`: moves the current block (and scrolls to it).
pub(in crate::patch) fn nav_ui(preview: &mut Preview, ui: &mut egui::Ui, hover: &str) {
    if preview.blocks() == 0 {
        return;
    }
    for (forward, glyph) in [(false, "▲"), (true, "▼")] {
        if ui.small_button(glyph).on_hover_text(hover).clicked() {
            preview.step(forward);
        }
    }
    ui.label(format!("{}/{}", preview.current + 1, preview.blocks()));
}

/// The lines (only the visible ones are laid out). Returns the block whose `↶` was clicked.
pub(in crate::patch) fn lines_ui(preview: &mut Preview, ui: &mut egui::Ui, revert_hover: &str) -> Option<usize> {
    let row_height = ui.text_style_height(&egui::TextStyle::Monospace) + 2.0;
    let font = egui::TextStyle::Monospace.resolve(ui.style());
    // The painter below does not grow the rows, so the horizontal scroll needs the real width.
    let char_width = ui.fonts_mut(|fonts| fonts.glyph_width(&font, 'x'));
    #[expect(clippy::cast_precision_loss, reason = "line lengths are small")]
    let content_width = (preview.longest_line() as f32).mul_add(char_width, TEXT_LEFT + 8.0);

    let mut scroll = egui::ScrollArea::both().auto_shrink(false);
    if let Some(row) = preview.take_scroll_row() {
        // `show_rows` puts `item_spacing` between rows: the `OPEN` line lands at the very top.
        #[expect(clippy::cast_precision_loss, reason = "row counts are small")]
        let offset = row as f32 * (row_height + ui.spacing().item_spacing.y);
        scroll = scroll.vertical_scroll_offset(offset);
    }

    let current = preview.current;
    let mut revert = None;
    scroll.show_rows(ui, row_height, preview.lines().len(), |ui, range| {
        for (kind, text) in &preview.lines()[range] {
            let size = egui::vec2(ui.available_width().max(content_width), row_height);
            let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
            if let Some(fill) = line_fill(ui, *kind, current) {
                ui.painter().rect_filled(rect, 0.0, fill);
            }
            // `↶` at the start of each block, like git's conflict buttons.
            let gutter = egui::Rect::from_min_size(rect.min, egui::vec2(20.0, row_height));
            if let Kind::Open(block) = kind
                && ui.put(gutter, egui::Button::new("↶").small()).on_hover_text(revert_hover).clicked()
            {
                revert = Some(*block);
            }
            let color = match kind {
                Kind::Open(_) | Kind::Marker => ui.visuals().weak_text_color(),
                _ => ui.visuals().text_color(),
            };
            let pos = rect.left_center() + egui::vec2(TEXT_LEFT, 0.0);
            ui.painter().text(pos, egui::Align2::LEFT_CENTER, text, font.clone(), color);
        }
    });
    revert
}

/// The background of a line: git's green / red, and a tint on the current block's `OPEN` line.
fn line_fill(ui: &egui::Ui, kind: Kind, current: usize) -> Option<egui::Color32> {
    match kind {
        Kind::New => Some(egui::Color32::from_rgb(46, 160, 67).gamma_multiply(0.18)),
        Kind::Old => Some(egui::Color32::from_rgb(248, 81, 73).gamma_multiply(0.18)),
        Kind::Open(block) if block == current => Some(ui.visuals().selection.bg_fill.gamma_multiply(0.35)),
        _ => None,
    }
}
