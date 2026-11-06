//! Step 1 of a frame: what the keys ask for, before the `TextEdit` sees them.

use egui::text::{CCursor, CCursorRange};

use crate::{
    EditorAction, EditorLabels, Indent, Keymap, QuickFix,
    cursor::set_selection,
    edit_ops,
    editor::{EditorFrame, text::sorted_chars},
    keymap::KeymapOutput,
    palette,
    palette::PaletteContext,
    state::CodeEditorState,
    text_pos::LinePos,
    ui,
    ui::palette::PaletteInput,
};

/// What the keys of this frame asked for. See [`read_input`].
pub(crate) struct FrameInput<'a> {
    pub(crate) picked: Option<palette::Picked<'a>>,
    pub(crate) triggers: palette::Triggers,
    /// `Enter` / `Tab` in the quick fix menu.
    pub(crate) fix: Option<QuickFix>,
    /// `Enter` / `Tab` in the completion popup.
    pub(crate) accept_key: bool,
    /// The text changed by the menu's Paste.
    pub(crate) pasted: bool,
    pub(crate) keys: KeymapOutput,
}

/// Step 1: the keys owned by the popups and the keymap. Must run before the `TextEdit`.
pub(crate) fn read_input<'a>(
    frame: EditorFrame<'a>,
    ui: &egui::Ui,
    state: &mut CodeEditorState,
    text: &mut String,
    labels: &EditorLabels<'a>,
    actions: &'a [EditorAction<'a>],
) -> FrameInput<'a> {
    let EditorFrame { id, language, features, keymap, .. } = frame;
    let has_focus = current_focus(ui, id, keymap, state.had_focus);

    state.hover.handle_input(ui, id); // before the keymap swallows the key events
    let context = PaletteContext { keymap, features, inserting: state.keymap.is_inserting(keymap) };
    let palette_input = PaletteInput { id, has_focus, context, labels, actions };
    let picked = ui::palette::show(&mut state.palette, ui, palette_input);
    let triggers = ui::palette::read_triggers(ui, has_focus).with_picked(picked);
    state.open_requested_menus(ui.ctx(), id, text, language, triggers);
    // While the quick fix menu is open it owns the arrow keys.
    let fix = state.quick_fix.handle_keys(ui);
    state.find.handle_keys(keymap, ui, id, text, has_focus);
    let accept_key = state.completion.handle_keys(ui);
    let pasted = take_requested_paste(ui, id, state, text);
    let keys = if has_focus && !state.quick_fix.is_open() {
        state.keymap.handle(keymap, ui, id, text, state.view)
    } else {
        KeymapOutput::default()
    };
    FrameInput { picked, triggers, fix, accept_key, pasted, keys }
}

/// `Shift+Alt+F`. Returns whether the text changed.
pub(crate) fn apply_format(frame: EditorFrame<'_>, ui: &egui::Ui, text: &mut String, asked: bool) -> bool {
    if !(frame.features.format && asked) {
        return false;
    }
    match frame.language.format(text) {
        Some(formatted) if formatted != *text => {
            let cursor = egui::TextEdit::load_state(ui.ctx(), frame.id)
                .and_then(|edit| edit.cursor.char_range())
                .map(|range| LinePos::from_char_index(text, range.primary.index.0));
            *text = formatted;
            if let Some(cursor) = cursor {
                keep_cursor_line(ui.ctx(), frame.id, text, cursor);
            }
            true
        }
        _ => false,
    }
}

/// Puts the cursor on its line after the formatter changed the text. The char index would be on
/// another line by then: every line before it may have changed its indentation.
pub(crate) fn keep_cursor_line(ctx: &egui::Context, id: egui::Id, text: &str, cursor: LinePos) {
    let mut edit = egui::TextEdit::load_state(ctx, id).unwrap_or_default();
    let index = cursor.to_char_index(text);
    edit.cursor.set_char_range(Some(CCursorRange::one(CCursor::new(index))));
    edit.store(ctx, id);
}

/// egui's `TextEdit` always inserts a tab character for `Tab`: where the language indents with spaces,
/// it gets those instead. What the keymap or a popup wanted the key for is gone from the input by now.
pub(crate) fn tab_key_follows_indent(ui: &egui::Ui, id: egui::Id, indent: Indent) {
    let Indent::Spaces(width) = indent else { return };
    if !ui.memory(|memory| memory.has_focus(id)) {
        return;
    }
    ui.input_mut(|input| {
        for event in &mut input.events {
            if let egui::Event::Key { key: egui::Key::Tab, pressed: true, modifiers, .. } = event
                && modifiers.is_none()
            {
                *event = egui::Event::Text(" ".repeat(width));
            }
        }
    });
}

/// Whether the editor has the keyboard.
///
/// egui drops a widget's focus on `Esc` before we run, but Vim needs that key (e.g., to leave
/// Visual mode): the focus is taken back then.
pub(crate) fn current_focus(ui: &egui::Ui, id: egui::Id, keymap: Keymap, had_focus: bool) -> bool {
    let has_focus = ui.memory(|m| m.has_focus(id));
    if !has_focus && had_focus && keymap == Keymap::Vim && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        ui.memory_mut(|m| m.request_focus(id));
        return true;
    }
    has_focus
}

/// The clipboard text the menu's Paste asked for, put in at the selection. Returns whether it came.
///
/// It is taken before the keymap: Vim's Normal mode reads a paste event as `Ctrl+V`.
pub(crate) fn take_requested_paste(
    ui: &egui::Ui,
    id: egui::Id,
    state: &mut CodeEditorState,
    text: &mut String,
) -> bool {
    /// Seconds until the request is forgotten, so that a later `Ctrl+V` means what it should.
    const WAIT: f64 = 0.5;
    let Some(asked_at) = state.pending_paste else { return false };
    let pasted = ui.input_mut(|i| {
        let at = i.events.iter().position(|event| matches!(event, egui::Event::Paste(_)))?;
        match i.events.remove(at) {
            egui::Event::Paste(pasted) => Some(pasted),
            _ => None,
        }
    });
    let Some(pasted) = pasted else {
        if ui.input(|i| i.time) - asked_at > WAIT {
            state.pending_paste = None;
        } else {
            ui.ctx().request_repaint();
        }
        return false;
    };
    state.pending_paste = None;
    let selection = egui::TextEdit::load_state(ui.ctx(), id)
        .and_then(|state| state.cursor.char_range())
        .map_or(0..0, |range| sorted_chars(&range));
    let at = edit_ops::replace_chars(text, &selection, &pasted);
    set_selection(ui.ctx(), id, at..at);
    true
}
