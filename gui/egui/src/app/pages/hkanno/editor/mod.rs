//! The annotation text editor: an egui `TextEdit` + hkanno language features.
//!
//! # One frame
//! 1. completion popup consumes its keys (Enter/Tab/arrows/Escape) if open
//! 2. `TextEdit` is shown with a custom layouter ([`layout`]: colors + diagnostic underlines)
//! 3. a picked completion is applied to the text (cursor moves to the snippet placeholder)
//! 4. overlays are drawn: completion list, hover, signature help, inlay hints ([`overlays`])
//!
//! The caller owns the text and the diagnostics; this module never runs `diagnostics()` itself,
//! so they are recomputed only when the text changes ([`EditorOutput::changed`]).

mod completion;
mod layout;
mod overlays;
pub(crate) mod text_pos;

use d_merge_gui_shared::settings::hkanno::HkannoLspOptions;
use egui::text::{CCursor, CCursorRange};
use hkanno_lsp::features::diagnostics::Diagnostic;

pub(crate) use self::layout::severity_color;
use self::{completion::CompletionPopup, text_pos::LinePos};

/// Per-tab editor state that is not part of the document.
#[derive(Default)]
pub(crate) struct EditorState {
    completion: CompletionPopup,
    /// The saved cursor (from `settings.json`) was already applied.
    cursor_restored: bool,
}

impl EditorState {
    /// Returns `saved` the first time only, so the caller restores the cursor once per tab.
    pub(crate) const fn take_pending_cursor(&mut self, saved: usize) -> Option<usize> {
        if self.cursor_restored {
            None
        } else {
            self.cursor_restored = true;
            Some(saved)
        }
    }
}

/// What happened in the editor this frame.
pub(crate) struct EditorOutput {
    /// The text was edited (by typing or by accepting a completion).
    pub(crate) changed: bool,
    /// Cursor position, if the editor has one.
    pub(crate) cursor: Option<LinePos>,
}

/// Shows the editor for `text`.
///
/// - `id`: stable per tab, so each tab keeps its own cursor and undo history
/// - `diagnostics`: computed by the caller from the current text
pub(crate) fn editor_ui(
    ui: &mut egui::Ui,
    id: egui::Id,
    text: &mut String,
    diagnostics: &[Diagnostic],
    options: &HkannoLspOptions,
    state: &mut EditorState,
) -> EditorOutput {
    // ── 1. Keys for the popup (must run before the TextEdit sees them) ───────
    let popup_key = state.completion.handle_keys(ui);
    let ctrl_space = options.completion
        && ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Space));

    // ── 2. The TextEdit ───────────────────────────────────────────────────
    let underlines = if options.diagnostics { diagnostics } else { &[] };
    let semantic = options.semantic_tokens;
    let mut layouter = |ui: &egui::Ui, buf: &dyn egui::TextBuffer, _wrap_width: f32| {
        layout::layout(ui, buf.as_str(), semantic, underlines)
    };

    let output = egui::TextEdit::multiline(text)
        .id(id)
        .code_editor()
        .desired_width(f32::INFINITY)
        .desired_rows(30)
        .layouter(&mut layouter)
        .show(ui);

    let response = &output.response.response;
    let mut changed = response.changed();
    let cursor_index = output.cursor_range.map(|range| range.primary.index.0);
    let mut cursor = cursor_index.map(|i| LinePos::from_char_index(text, i));

    // ── 3. Completion: refresh / accept ──────────────────────────────────────
    if options.completion {
        if let Some(cursor) = cursor {
            if ctrl_space {
                state.completion.refresh(text, cursor);
            } else if changed {
                state.completion.on_text_changed(ui, text, cursor);
            }
        }
        state.completion.follow_cursor(cursor);
    } else {
        state.completion.close();
    }

    let anchor = cursor_index.map(|i| {
        output.galley.pos_from_cursor(CCursor::new(i)).translate(output.galley_pos.to_vec2())
    });
    let clicked = anchor.is_some_and(|anchor| state.completion.ui(ui.ctx(), anchor));

    if (clicked || popup_key == Some(completion::PopupKey::Accept))
        && let Some(accepted) = state.completion.accept(text)
    {
        *text = accepted.text;
        set_selection(ui.ctx(), id, accepted.select.clone());
        cursor = Some(LinePos::from_char_index(text, accepted.select.end));
        changed = true;
        response.request_focus();
    }

    // ── 4. Overlays ──────────────────────────────────────────────────────────
    if let Some(cursor) = cursor {
        overlays::current_line_highlight(ui, &output.galley, output.galley_pos, cursor.line);
    }
    if options.inlay_hints {
        overlays::inlay_hint_labels(ui, &output.galley, output.galley_pos, text);
    }
    if options.hover || options.diagnostics {
        overlays::hover_tooltip(
            response,
            &output.galley,
            output.galley_pos,
            text,
            underlines,
            options.hover,
        );
    }
    if options.signature_help
        && response.has_focus()
        && !state.completion.is_open()
        && let (Some(cursor), Some(anchor)) = (cursor, anchor)
    {
        overlays::signature_box(ui.ctx(), anchor, text, cursor);
    }

    EditorOutput { changed, cursor }
}

/// Moves the cursor of the editor `id` (applied on the next frame).
///
/// e.g., jumping to a diagnostic from the problems list.
pub(crate) fn set_cursor(ctx: &egui::Context, id: egui::Id, index: usize) {
    set_selection(ctx, id, index..index);
}

/// Selects `range` (char indices) in the editor `id`.
fn set_selection(ctx: &egui::Context, id: egui::Id, range: std::ops::Range<usize>) {
    let mut state = egui::TextEdit::load_state(ctx, id).unwrap_or_default();
    state.cursor.set_char_range(Some(CCursorRange::two(
        CCursor::new(range.start),
        CCursor::new(range.end),
    )));
    state.store(ctx, id);
    ctx.memory_mut(|mem| mem.request_focus(id));
}
