//! Step 3 of a frame: what opens over the text.

use crate::{
    Keymap, Language, QuickFix,
    command::EditorCommand,
    completion::CompletionPopup,
    cursor::{set_cursor, set_selection},
    editor::EditorFrame,
    hover::HoverCtx,
    state::CodeEditorState,
    text_pos,
    text_pos::LinePos,
    ui,
    ui::overlays,
};

/// The list of what can follow a Vim key that waits for another, once it has waited for a moment.
pub(crate) fn show_which_key(
    ui: &egui::Ui,
    id: egui::Id,
    state: &mut CodeEditorState,
    keymap: Keymap,
    visible: egui::Rect,
    host_commands: &[EditorCommand],
) {
    let Some(prefix) = state.keymap.which_key_prefix(keymap) else {
        state.which_key_waiting = None;
        return;
    };
    let now = ui.input(|input| input.time);
    match ui::which_key::wait(&mut state.which_key_waiting, prefix, now) {
        Some(left) => ui.ctx().request_repaint_after_secs(left as f32),
        None => ui::which_key::show(ui.ctx(), id, visible, prefix, host_commands),
    }
}

/// Step 3: opens, updates or closes the completion list.
///
/// `(requested, typed)`: `Ctrl+Space` was pressed, the text changed.
pub(crate) fn update_completion_popup(
    frame: EditorFrame<'_>,
    ui: &egui::Ui,
    state: &mut CodeEditorState,
    text: &str,
    cursor: Option<LinePos>,
    inserting: bool,
    request: (bool, bool),
) {
    if frame.features.completion && inserting {
        update_completion(&mut state.completion, ui, frame.language, text, cursor, request);
    } else {
        state.completion.close();
    }
}

/// Draws the popup and puts in the item picked by a click or `accept_key`.
/// Returns whether the text changed.
#[expect(clippy::too_many_arguments, reason = "each is one piece of this step's data, no natural group")]
pub(crate) fn accept_completion(
    ui: &egui::Ui,
    id: egui::Id,
    state: &mut CodeEditorState,
    text: &mut String,
    anchor: Option<egui::Rect>,
    accept_key: bool,
    cursor: &mut Option<LinePos>,
    response: &egui::Response,
) -> bool {
    let clicked = anchor.is_some_and(|anchor| state.completion.ui(ui.ctx(), id, anchor));
    if !(clicked || accept_key) {
        return false;
    }
    let Some(accepted) = state.completion.accept(text) else { return false };
    *text = accepted.text;
    set_selection(ui.ctx(), id, accepted.select.clone());
    *cursor = Some(LinePos::from_char_index(text, accepted.select.end));
    response.request_focus();
    true
}

/// Draws the quick fix menu and applies the fix picked by a click or `key_fix`.
/// Returns whether the text changed.
pub(crate) fn run_quick_fix(
    ctx: &egui::Context,
    id: egui::Id,
    state: &mut CodeEditorState,
    text: &mut String,
    anchor: Option<egui::Rect>,
    key_fix: Option<QuickFix>,
) -> bool {
    let clicked = anchor.and_then(|anchor| state.quick_fix.ui(ctx, id, anchor));
    let Some(fix) = clicked.or(key_fix) else { return false };
    let at = apply_quick_fix(text, &fix);
    set_cursor(ctx, id, at);
    true
}

/// The hover popup (diagnostics and docs). `open_at`: opens it there, not at the pointer.
pub(crate) fn show_hover(
    frame: EditorFrame<'_>,
    ui: &egui::Ui,
    state: &mut CodeEditorState,
    text: &str,
    response: &egui::Response,
    (galley, galley_pos): (&egui::Galley, egui::Pos2),
    open_at: Option<usize>,
) {
    if !(frame.features.hover || frame.features.diagnostics) {
        return;
    }
    let hover = HoverCtx {
        id: frame.id,
        response,
        galley,
        galley_pos,
        text,
        language: frame.language,
        diagnostics: frame.underlines,
        with_docs: frame.features.hover,
    };
    state.hover.show(ui, &hover, open_at);
}

/// The signature help above the cursor.
pub(crate) fn show_signature(
    frame: EditorFrame<'_>,
    ui: &egui::Ui,
    text: &str,
    cursor: Option<LinePos>,
    anchor: Option<egui::Rect>,
) {
    if frame.features.signature_help
        && let (Some(cursor), Some(anchor)) = (cursor, anchor)
    {
        overlays::signature_box(ui.ctx(), frame.id, anchor, frame.language, text, cursor);
    }
}

/// Refreshes the completion list. `(requested, typed)`: `Ctrl+Space` was pressed, the text changed.
pub(crate) fn update_completion(
    completion: &mut CompletionPopup,
    ui: &egui::Ui,
    language: &dyn Language,
    text: &str,
    cursor: Option<LinePos>,
    (requested, typed): (bool, bool),
) {
    if let Some(cursor) = cursor {
        if requested {
            completion.refresh(language, text, cursor);
        } else if typed {
            completion.on_text_changed(ui, language, text, cursor);
        }
    }
    completion.follow_cursor(cursor);
}

/// Applies `fix` to `text`. Returns the char index where it starts, for the cursor.
pub(crate) fn apply_quick_fix(text: &mut String, fix: &QuickFix) -> usize {
    let line_start = LinePos { line: fix.line, column: 0 }.to_char_index(text);
    let start = text_pos::char_to_byte(text, line_start + fix.columns.start);
    let end = text_pos::char_to_byte(text, line_start + fix.columns.end);
    text.replace_range(start..end, &fix.new_text);
    line_start + fix.columns.start
}
