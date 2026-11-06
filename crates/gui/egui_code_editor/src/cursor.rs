//! The cursor and selection of an editor, from outside of it.

use egui::text::{CCursor, CCursorRange};

/// The `TextEdit` id used by `CodeEditor::new(id_salt, ..)`.
///
/// Lets a host address the editor (e.g., [`set_cursor`]) without holding the text.
pub fn editor_id(id_salt: impl egui::AsIdSalt) -> egui::Id {
    egui::Id::unique(("code_editor", id_salt))
}

/// Moves the cursor of the editor `id` and scrolls it into view (applied on the next frame).
///
/// e.g., jumping to a diagnostic from a problems list. Get `id` from [`editor_id`].
pub fn set_cursor(ctx: &egui::Context, id: egui::Id, index: usize) {
    set_selection(ctx, id, index..index);
    // `TextEdit` only scrolls for cursor moves it makes itself, so ask `show` to do it.
    ctx.data_mut(|data| data.insert_temp(scroll_request_id(id), true));
}

/// Temp-memory flag: "scroll the cursor into view on the next `show`".
pub(crate) fn scroll_request_id(id: egui::Id) -> egui::Id {
    id.with("scroll_to_cursor")
}

/// Selects `range` (char indices) in the editor `id` and focuses it.
pub(crate) fn set_selection(ctx: &egui::Context, id: egui::Id, range: std::ops::Range<usize>) {
    let mut state = egui::TextEdit::load_state(ctx, id).unwrap_or_default();
    state.cursor.set_char_range(Some(CCursorRange::two(CCursor::new(range.start), CCursor::new(range.end))));
    state.store(ctx, id);
    ctx.memory_mut(|mem| mem.request_focus(id));
}
