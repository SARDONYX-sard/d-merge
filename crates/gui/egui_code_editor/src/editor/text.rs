//! Step 2 of a frame: the `TextEdit`, with the selection kept for the right-click menu.

use egui::text::CCursorRange;

use crate::{Keymap, editor::EditorFrame, layout, state::CodeEditorState};

/// Step 2: the `TextEdit`, with the syntax colors, underlines and search matches.
pub(crate) fn show_text(
    frame: EditorFrame<'_>,
    ui: &mut egui::Ui,
    state: &mut CodeEditorState,
    text: &mut String,
    desired_rows: usize,
    gutter: Option<f32>,
) -> (f32, egui::text_edit::TextEditOutput) {
    let search = state.search_highlight(frame.keymap);
    // e.g., Vim Normal mode draws a block cursor: no blinking I-beam then.
    let hide_text_cursor = state.keymap.hides_text_cursor(frame.keymap);
    let look = TextLook { desired_rows, dim_selection: frame.keymap == Keymap::Vim, hide_text_cursor, gutter };
    let mut layouter = |ui: &egui::Ui, buf: &dyn egui::TextBuffer, _wrap_width: f32| {
        layout::cached(
            &mut state.layout,
            ui,
            buf.as_str(),
            frame.language,
            frame.underlines,
            search.as_ref(),
            frame.inline_diff,
        )
    };
    let before = ui
        .input(|input| input.pointer.secondary_pressed())
        .then(|| egui::TextEdit::load_state(ui.ctx(), frame.id)?.cursor.char_range())
        .flatten();
    let (gutter_left, mut output) = show_text_edit(ui, frame.id, text, look, &mut layouter);
    keep_selection_for_menu(ui, frame.id, &mut state.held_selection, before, &mut output);
    (gutter_left, output)
}

/// Pressing the right button on the selection keeps it, for the menu's Copy and Cut. egui's
/// `TextEdit` moves the cursor on the press of any button, so it is put back. A press elsewhere moves
/// the cursor there, as in VS Code.
///
/// `before`: the selection before this frame's press of the right button, if it was pressed.
pub(crate) fn keep_selection_for_menu(
    ui: &egui::Ui,
    id: egui::Id,
    held: &mut Option<CCursorRange>,
    before: Option<CCursorRange>,
    output: &mut egui::text_edit::TextEditOutput,
) {
    // The press has just moved the cursor to where it happened.
    let pressed_at = output.state.cursor.char_range().map(|range| range.primary.index.0);
    if let Some(before) = before
        && pressed_at.is_some_and(|at| selection_covers(&before, at))
    {
        *held = Some(before);
    }
    let Some(selection) = *held else { return };

    output.state.cursor.set_char_range(Some(selection));
    output.state.clone().store(ui.ctx(), id);
    output.cursor_range = Some(selection);
    // This frame has painted what egui moved: without the next one, it stays until the release.
    ui.ctx().request_repaint();
    if !ui.input(|input| input.pointer.secondary_down()) {
        *held = None;
    }
}

/// Is the char index `at` on a selection? Its end counts: the pointer on the right half of the last
/// char is there. An empty selection has nothing to keep.
pub(crate) const fn selection_covers(selection: &CCursorRange, at: usize) -> bool {
    let range = sorted_chars(selection);
    range.start < range.end && range.start <= at && at <= range.end
}

/// How the `TextEdit` looks. See [`show_text_edit`].
#[derive(Debug, Clone, Copy)]
pub(crate) struct TextLook {
    pub(crate) desired_rows: usize,
    /// Vim selections often span many lines: a lighter color keeps the text readable.
    pub(crate) dim_selection: bool,
    /// e.g., Vim Normal mode draws a block cursor instead.
    pub(crate) hide_text_cursor: bool,
    /// Room to leave for the line numbers.
    pub(crate) gutter: Option<f32>,
}

/// Shows the `TextEdit`. Returns the gutter's left edge and the output.
pub(crate) fn show_text_edit(
    ui: &mut egui::Ui,
    id: egui::Id,
    text: &mut String,
    look: TextLook,
    layouter: &mut dyn FnMut(&egui::Ui, &dyn egui::TextBuffer, f32) -> std::sync::Arc<egui::Galley>,
) -> (f32, egui::text_edit::TextEditOutput) {
    // As VS Code's: the whole view takes a click, not only as many rows as the text has. In a scroll
    // area the height left is the view's; elsewhere it may be unbounded, and the rows decide.
    let fill = Some(ui.available_height()).filter(|height| height.is_finite()).unwrap_or_default();
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        let gutter_left = ui.cursor().left();
        if let Some(width) = look.gutter {
            ui.add_space(width);
        }
        let output = ui.scope(|ui| {
            if look.dim_selection {
                let selection = &mut ui.visuals_mut().selection.bg_fill;
                *selection = selection.gamma_multiply(0.55);
            }
            if look.hide_text_cursor {
                let cursor = &mut ui.visuals_mut().text_cursor;
                cursor.stroke = egui::Stroke::NONE;
                cursor.blink = false;
            }
            egui::TextEdit::multiline(text)
                .id(id)
                .code_editor()
                .desired_width(f32::INFINITY)
                .desired_rows(look.desired_rows)
                .min_size(egui::vec2(0.0, fill))
                .layouter(layouter)
                .show(ui)
        });
        (gutter_left, output.inner)
    })
    .inner
}

/// The char range of a selection, start first.
pub(crate) const fn sorted_chars(range: &CCursorRange) -> std::ops::Range<usize> {
    let (a, b) = (range.primary.index.0, range.secondary.index.0);
    if a <= b { a..b } else { b..a }
}
