//! A small VS Code–like code editor widget for egui, independent of any language.
//!
//! - syntax highlighting, diagnostics underlines, hover, completion popup, signature help,
//!   inlay hints and formatting, all provided by a [`Language`] implementation
//! - key binding styles ([`Keymap`]): plain editing or Vim (Emacs can be added in [`keymap`])
//!
//! # Usage (egui builder style)
//! ```ignore
//! // Kept by the caller, one per document:
//! let mut state = CodeEditorState::default();
//!
//! let output = CodeEditor::new("my_editor", &mut text)
//!     .language(&MyLanguage)          // optional; plain text without it
//!     .diagnostics(&diagnostics)      // optional; computed by the caller when text changes
//!     .features(EditorFeatures::default())
//!     .keymap(Keymap::Vim)
//!     .show(ui, &mut state);
//!
//! if output.changed { /* re-run diagnostics */ }
//! if output.command == Some(EditorCommand::Save) { /* `:w` in Vim */ }
//! ```
//!
//! # One frame
//! 1. the keymap (e.g., Vim) and the completion popup consume their keys **before** the `TextEdit`
//! 2. `TextEdit` is shown with a custom layouter (colors + diagnostic underlines)
//! 3. a picked completion / `Shift+Alt+F` formatting is applied to the text
//! 4. overlays are drawn (cursor line, block cursor, hover, signature help, inlay hints)

mod completion;
mod hover;
pub mod keymap;
pub mod language;
pub mod languages;
mod layout;
mod overlays;
pub mod text_pos;

use egui::text::{CCursor, CCursorRange};

use self::{
    completion::CompletionPopup,
    hover::{HoverCtx, HoverPopup},
    text_pos::LinePos,
};
pub use self::{
    keymap::{Keymap, KeymapState, VimOptions},
    language::{CompletionItem, CompletionList, Diagnostic, Language, Severity, SignatureHelp},
    layout::line_job,
};

/// Which optional features are on. All default to `true` except inlay hints / signature help.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditorFeatures {
    pub completion: bool,
    pub hover: bool,
    pub diagnostics: bool,
    pub format: bool,
    pub inlay_hints: bool,
    pub signature_help: bool,
}

impl Default for EditorFeatures {
    fn default() -> Self {
        Self {
            completion: true,
            hover: true,
            diagnostics: true,
            format: true,
            inlay_hints: false,
            signature_help: false,
        }
    }
}

/// Requests from the editor to its host (the host decides what they mean).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorCommand {
    /// e.g., Vim `:w`.
    Save,
}

/// Per-document state that is not part of the text. Keep one per open document.
#[derive(Debug, Default)]
pub struct CodeEditorState {
    completion: CompletionPopup,
    hover: HoverPopup,
    /// State of the key binding styles (e.g., Vim mode and registers).
    pub keymap: KeymapState,
}

/// What happened this frame.
#[derive(Debug)]
pub struct CodeEditorOutput {
    /// The text was edited (typing, completion, formatting, keymap commands).
    pub changed: bool,
    /// Cursor position, if the editor has one.
    pub cursor: Option<LinePos>,
    /// Request for the host, e.g., Vim `:w`.
    pub command: Option<EditorCommand>,
}

/// Language used when none is given: no colors, no completion, ...
struct PlainText;
impl Language for PlainText {}

/// The editor widget. See the crate docs.
#[must_use = "You should call .show()"]
pub struct CodeEditor<'a> {
    id: egui::Id,
    text: &'a mut String,
    language: &'a dyn Language,
    diagnostics: &'a [Diagnostic],
    features: EditorFeatures,
    keymap: Keymap,
    desired_rows: usize,
}

impl std::fmt::Debug for CodeEditor<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CodeEditor")
            .field("id", &self.id)
            .field("keymap", &self.keymap)
            .finish_non_exhaustive()
    }
}

impl<'a> CodeEditor<'a> {
    /// `id_salt` must be stable per document (cursor and undo history are stored under it).
    pub fn new(id_salt: impl egui::AsIdSalt, text: &'a mut String) -> Self {
        Self {
            id: editor_id(id_salt),
            text,
            language: &PlainText,
            diagnostics: &[],
            features: EditorFeatures::default(),
            keymap: Keymap::Normal,
            desired_rows: 30,
        }
    }

    pub const fn language(mut self, language: &'a dyn Language) -> Self {
        self.language = language;
        self
    }

    /// Problems to underline (and show on hover).
    pub const fn diagnostics(mut self, diagnostics: &'a [Diagnostic]) -> Self {
        self.diagnostics = diagnostics;
        self
    }

    pub const fn features(mut self, features: EditorFeatures) -> Self {
        self.features = features;
        self
    }

    /// Key binding style. Default: [`Keymap::Normal`].
    pub const fn keymap(mut self, keymap: Keymap) -> Self {
        self.keymap = keymap;
        self
    }

    /// Minimum height in text rows (default 30).
    pub const fn desired_rows(mut self, rows: usize) -> Self {
        self.desired_rows = rows;
        self
    }

    pub fn show(self, ui: &mut egui::Ui, state: &mut CodeEditorState) -> CodeEditorOutput {
        let Self { id, text, language, diagnostics, features, keymap, desired_rows } = self;
        let has_focus = ui.memory(|m| m.has_focus(id));

        // ── 1. Keys owned by the popups / keymap (must run before the TextEdit) ──
        state.hover.close_on_input(ui); // before the keymap swallows the key events
        let accept_key = state.completion.handle_keys(ui);
        let (mut changed, command, show_hover) = if has_focus {
            let out = state.keymap.handle(keymap, ui, id, text);
            (out.changed, out.command, out.show_hover)
        } else {
            (false, None, false)
        };
        let inserting = state.keymap.is_inserting(keymap);

        let ctrl_space = features.completion
            && inserting
            && ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Space));
        let format_keys = egui::Modifiers::SHIFT | egui::Modifiers::ALT;
        if features.format
            && has_focus
            && ui.input_mut(|i| i.consume_key(format_keys, egui::Key::F))
            && let Some(formatted) = language.format(text)
            && formatted != *text
        {
            *text = formatted;
            changed = true;
        }

        // ── 2. The TextEdit ───────────────────────────────────────────────────
        let underlines = if features.diagnostics { diagnostics } else { &[] };
        let mut layouter = |ui: &egui::Ui, buf: &dyn egui::TextBuffer, _wrap_width: f32| {
            layout::layout(ui, buf.as_str(), language, underlines)
        };
        let output = egui::TextEdit::multiline(text)
            .id(id)
            .code_editor()
            .desired_width(f32::INFINITY)
            .desired_rows(desired_rows)
            .layouter(&mut layouter)
            .show(ui);

        let response = &output.response.response;
        changed |= response.changed();
        let cursor_index = output.cursor_range.map(|range| range.primary.index.0);
        let mut cursor = cursor_index.map(|i| LinePos::from_char_index(text, i));

        // ── 3. Completion ────────────────────────────────────────────────────────
        if features.completion && inserting {
            if let Some(cursor) = cursor {
                if ctrl_space {
                    state.completion.refresh(language, text, cursor);
                } else if response.changed() {
                    state.completion.on_text_changed(ui, language, text, cursor);
                }
            }
            state.completion.follow_cursor(cursor);
        } else {
            state.completion.close();
        }

        let anchor = cursor_index.map(|i| {
            output.galley.pos_from_cursor(CCursor::new(i)).translate(output.galley_pos.to_vec2())
        });
        let clicked = anchor.is_some_and(|anchor| state.completion.ui(ui.ctx(), id, anchor));
        if (clicked || accept_key)
            && let Some(accepted) = state.completion.accept(text)
        {
            *text = accepted.text;
            set_selection(ui.ctx(), id, accepted.select.clone());
            cursor = Some(LinePos::from_char_index(text, accepted.select.end));
            changed = true;
            response.request_focus();
        }

        // ── 4. Overlays ──────────────────────────────────────────────────────────
        let (galley, galley_pos) = (&output.galley, output.galley_pos);
        if let Some(cursor) = cursor {
            overlays::current_line(ui, galley, galley_pos, cursor.line);
        }
        // Requested by `set_cursor` (e.g., a click in a problems list).
        if ui.data_mut(|d| d.remove_temp::<bool>(scroll_request_id(id))).unwrap_or(false)
            && let Some(cursor) = anchor
        {
            ui.scroll_to_rect(cursor, Some(egui::Align::Center));
        }
        if let Some(index) = state.keymap.block_cursor(keymap, cursor_index, text) {
            overlays::block_cursor(ui, galley, galley_pos, index);
        }
        if features.inlay_hints {
            overlays::inlay_hints(ui, galley, galley_pos, language, text);
        }
        if features.hover || features.diagnostics {
            let hover = HoverCtx {
                id,
                response,
                galley,
                galley_pos,
                text,
                language,
                diagnostics: underlines,
                with_docs: features.hover,
            };
            if show_hover && let Some(index) = cursor_index {
                state.hover.open_at(index, &hover); // e.g., Vim `K`
            }
            state.hover.ui(ui, &hover);
        }
        if features.signature_help
            && response.has_focus()
            && inserting
            && !state.completion.is_open()
            && let (Some(cursor), Some(anchor)) = (cursor, anchor)
        {
            overlays::signature_box(ui.ctx(), id, anchor, language, text, cursor);
        }

        CodeEditorOutput { changed, cursor, command }
    }
}

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
fn scroll_request_id(id: egui::Id) -> egui::Id {
    id.with("scroll_to_cursor")
}

/// Selects `range` (char indices) in the editor `id` and focuses it.
fn set_selection(ctx: &egui::Context, id: egui::Id, range: std::ops::Range<usize>) {
    let mut state = egui::TextEdit::load_state(ctx, id).unwrap_or_default();
    state.cursor.set_char_range(Some(CCursorRange::two(
        CCursor::new(range.start),
        CCursor::new(range.end),
    )));
    state.store(ctx, id);
    ctx.memory_mut(|mem| mem.request_focus(id));
}
