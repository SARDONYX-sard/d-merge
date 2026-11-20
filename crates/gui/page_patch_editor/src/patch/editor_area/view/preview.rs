//! Draws the patch preview. The data is in [`crate::patch::editor::preview`].

use egui_widgets::shadcn::{Button, Icon};

use crate::patch::editor::preview::{Kind, Preview};

/// Where the text starts: the `↶` gutter is left of it.
const TEXT_LEFT: f32 = 24.0;

/// `▲ ▼ 2/5`: moves the current block (and scrolls to it).
pub(super) fn nav_ui(preview: &mut Preview, ui: &mut egui::Ui, hover: &str) {
    if preview.blocks() == 0 {
        return;
    }
    for (forward, icon) in [(false, Icon::ChevronUp), (true, Icon::ChevronDown)] {
        if ui.add(Button::new("").icon(icon).small()).on_hover_text(hover).clicked() {
            preview.step(forward);
        }
    }
    ui.label(format!("{}/{}", preview.current + 1, preview.blocks()));
}

/// What a click in the preview asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::patch::editor_area) enum PreviewEvent {
    /// `↶`: put this changed block back to the template.
    Revert(usize),
    /// A double click: show this preview row in the editor.
    JumpToEditor(usize),
}

/// The lines (only the visible ones are laid out).
/// `object`: `(behavior, id)` of the object it is of: each keeps where it was scrolled to.
/// `language`: of the patch's lines (XML, or the animation data's).
pub(super) fn lines_ui(
    preview: &mut Preview,
    ui: &mut egui::Ui,
    object: Option<(usize, &str)>,
    (revert_hover, language): (&str, &dyn egui_code_editor::Language),
) -> Option<PreviewEvent> {
    let row_height = ui.text_style_height(&egui::TextStyle::Monospace) + 2.0;
    let font = egui::TextStyle::Monospace.resolve(ui.style());
    // The painter below does not grow the rows, so the horizontal scroll needs the real width.
    let char_width = ui.fonts_mut(|fonts| fonts.glyph_width(&font, 'x'));
    #[expect(clippy::cast_precision_loss, reason = "line lengths are small")]
    let content_width = (preview.longest_line() as f32).mul_add(char_width, TEXT_LEFT + 8.0);

    // `show_rows` puts `item_spacing` between rows.
    #[expect(clippy::cast_precision_loss, reason = "row counts are small")]
    let row_top = |row: usize| row as f32 * (row_height + ui.spacing().item_spacing.y);
    // Within what can be scrolled: past the end, the scroll area would go beyond and spring back.
    // The view is what the scroll area measured last frame: the room left less the horizontal bar
    // (shown with a long line). Before that, the room left less the bar's.
    let view_key = ui.make_persistent_id("patch_preview_view_height");
    let room = ui.available_height();
    let view_height =
        ui.data(|data| data.get_temp(view_key)).unwrap_or_else(|| room - ui.spacing().scroll.allocated_width());
    let max_offset = (row_top(preview.lines().len()) - ui.spacing().item_spacing.y - view_height).max(0.0);
    let offset = |top: f32| top.clamp(0.0, max_offset);
    // Per object, as the editor's: one object's place, past another's end, would spring back slowly.
    let id_salt = ("patch_preview_scroll", object);
    let mut scroll = egui::ScrollArea::both().id_salt(id_salt).auto_shrink(false);
    // Shorter since it was scrolled (e.g., an edit took lines away): at its end at once.
    let scrolled = egui::scroll_area::State::load(ui.ctx(), ui.make_persistent_id(id_salt));
    if scrolled.is_some_and(|state| state.unclamped_offset().y > max_offset) {
        scroll = scroll.vertical_scroll_offset(max_offset);
    }
    if let Some(row) = preview.take_followed_scroll() {
        scroll = scroll.vertical_scroll_offset(offset(row_top(row) - view_height / 2.0));
    }
    // After the line above: ▲ / ▼ was asked for last, so it wins. The `OPEN` line lands at the top.
    if let Some(row) = preview.take_scroll_row() {
        scroll = scroll.vertical_scroll_offset(offset(row_top(row)));
    }
    let followed = preview.followed_row();

    let current = preview.current;
    let mut event = None;
    let shown = scroll.show_rows(ui, row_height, preview.lines().len(), |ui, range| {
        let first_row = range.start;
        for (i, (kind, text)) in preview.lines()[range].iter().enumerate() {
            let size = egui::vec2(ui.available_width().max(content_width), row_height);
            let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
            if response.double_clicked() {
                event = Some(PreviewEvent::JumpToEditor(first_row + i));
            }
            if let Some(fill) = line_fill(ui, *kind, current) {
                ui.painter().rect_filled(rect, 0.0, fill);
            }
            if followed == Some(first_row + i) {
                ui.painter().rect_filled(rect, 0.0, ui.visuals().selection.bg_fill.gamma_multiply(0.3));
            }
            // `↶` at the start of each block, like git's conflict buttons.
            let gutter = egui::Rect::from_min_size(rect.min, egui::vec2(20.0, row_height));
            if let Some(block) = kind.block()
                && ui.put(gutter, Button::new("").icon(Icon::Undo2).small()).on_hover_text(revert_hover).clicked()
            {
                event = Some(PreviewEvent::Revert(block));
            }
            let pos = rect.left_center() + egui::vec2(TEXT_LEFT, 0.0);
            if matches!(kind, Kind::Open(_) | Kind::Marker) {
                // The markers are not XML: they stay out of the way. Pandora's operations are.
                let color = ui.visuals().weak_text_color();
                ui.painter().text(pos, egui::Align2::LEFT_CENTER, text, font.clone(), color);
            } else {
                // Colored per visible row only, so huge files stay fast.
                let job = egui_code_editor::line_job_at(ui, first_row + i, text, language);
                let galley = ui.painter().layout_job(job);
                let top = pos - egui::vec2(0.0, galley.size().y / 2.0);
                ui.painter().galley(top, galley, ui.visuals().text_color());
            }
        }
    });
    ui.data_mut(|data| data.insert_temp(view_key, shown.inner_rect.height()));
    event
}

/// The background of a line: git's green / red, and a tint on the current block's `OPEN` line.
fn line_fill(ui: &egui::Ui, kind: Kind, current: usize) -> Option<egui::Color32> {
    match kind {
        Kind::Open(block) | Kind::Op { block, .. } if block == current => {
            Some(ui.visuals().selection.bg_fill.gamma_multiply(0.35))
        }
        Kind::New => Some(egui::Color32::from_rgb(46, 160, 67).gamma_multiply(0.18)),
        Kind::Old | Kind::Op { removes: true, .. } => Some(egui::Color32::from_rgb(248, 81, 73).gamma_multiply(0.18)),
        _ => None,
    }
}
