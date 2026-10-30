//! A small VS Code–like code editor widget for egui, independent of any language.
//!
//! - syntax highlighting, diagnostics underlines, hover, completion popup, signature help,
//!   inlay hints and formatting, all provided by a [`Language`] implementation
//! - optional Vim key bindings ([`vim`])
//!
//! # Usage (egui builder style)
//! ``ignore
//! // Kept by the caller, one per document:
//! let mut state = CodeEditorState::default();
//!
//! let output = CodeEditor::new("my_editor", &mut text)
//!     .language(&MyLanguage)          // optional; plain text without it
//!     .diagnostics(&diagnostics)      // optional; computed by the caller when text changes
//!     .features(EditorFeatures::default())
//!     .vim(true)
//!     .show(ui, &mut state);
//!
//! if output.changed { /* re-run diagnostics */ }
//! if output.command == Some(EditorCommand::Save) { /* `:w` in Vim */ }
//! ``
//!
//! # One frame
//! 1. Vim (if on) and the completion popup consume their keys **before** the `TextEdit`
//! 2. `TextEdit` is shown with a custom layouter ([`layout`]: colors + underlines)
//! 3. a picked completion / `Shift+Alt+F` formatting is applied to the text
//! 4. overlays are drawn ([`overlays`])

mod completion;
pub(crate) mod language;
mod layout;
mod overlays;
pub(crate) mod text_pos;
pub(crate) mod vim;

use egui::text::{CCursor, CCursorRange};

pub(crate) use self::language::{
    CompletionItem, CompletionList, Diagnostic, Language, Severity, SignatureHelp,
};
use self::{completion::CompletionPopup, text_pos::LinePos, vim::VimState};

/// Which optional features are on. All default to `true` except inlay hints / signature help.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EditorFeatures {
    pub(crate) completion: bool,
    pub(crate) hover: bool,
    pub(crate) diagnostics: bool,
    pub(crate) format: bool,
    pub(crate) inlay_hints: bool,
    pub(crate) signature_help: bool,
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
pub(crate) enum EditorCommand {
    /// Vim `:w`.
    Save,
}

/// Per-document state that is not part of the text. Keep one per open document.
#[derive(Default)]
pub(crate) struct CodeEditorState {
    completion: CompletionPopup,
    pub(crate) vim: VimState,
}

/// What happened this frame.
pub(crate) struct CodeEditorOutput {
    /// The text was edited (typing, completion, formatting, Vim commands).
    pub(crate) changed: bool,
    /// Cursor position, if the editor has one.
    pub(crate) cursor: Option<LinePos>,
    /// Request for the host, e.g., Vim `:w`.
    pub(crate) command: Option<EditorCommand>,
}

/// Language used when none is given: no colors, no completion, ...
struct PlainText;
impl Language for PlainText {}

/// The editor widget. See the module docs.
#[must_use = "You should call .show()"]
pub(crate) struct CodeEditor<'a> {
    id: egui::Id,
    text: &'a mut String,
    language: &'a dyn Language,
    diagnostics: &'a [Diagnostic],
    features: EditorFeatures,
    vim: bool,
    desired_rows: usize,
}

impl<'a> CodeEditor<'a> {
    /// `id_salt` must be stable per document (cursor and undo history are stored under it).
    pub(crate) fn new(id_salt: impl egui::AsIdSalt, text: &'a mut String) -> Self {
        Self {
            id: editor_id(id_salt),
            text,
            language: &PlainText,
            diagnostics: &[],
            features: EditorFeatures::default(),
            vim: false,
            desired_rows: 30,
        }
    }

    pub(crate) const fn language(mut self, language: &'a dyn Language) -> Self {
        self.language = language;
        self
    }

    /// Problems to underline (and show on hover).
    pub(crate) const fn diagnostics(mut self, diagnostics: &'a [Diagnostic]) -> Self {
        self.diagnostics = diagnostics;
        self
    }

    pub(crate) const fn features(mut self, features: EditorFeatures) -> Self {
        self.features = features;
        self
    }

    /// Enables Vim key bindings.
    pub(crate) const fn vim(mut self, vim: bool) -> Self {
        self.vim = vim;
        self
    }

    /// Minimum height in text rows (default 30).
    #[allow(
        dead_code,
        reason = "builder option of this generic widget; unused by the current pages"
    )]
    pub(crate) const fn desired_rows(mut self, rows: usize) -> Self {
        self.desired_rows = rows;
        self
    }

    pub(crate) fn show(self, ui: &mut egui::Ui, state: &mut CodeEditorState) -> CodeEditorOutput {
        let Self { id, text, language, diagnostics, features, vim, desired_rows } = self;
        let has_focus = ui.memory(|m| m.has_focus(id));

        // ── 1. Keys owned by the popup / Vim (must run before the TextEdit) ──────
        let accept_key = state.completion.handle_keys(ui);
        let (mut changed, command) = if vim && has_focus {
            let out = state.vim.handle(ui, id, text);
            (out.changed, out.command)
        } else {
            (false, None)
        };
        let insert_like = !vim || state.vim.mode() == vim::Mode::Insert;

        let ctrl_space = features.completion
            && insert_like
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
        if features.completion && insert_like {
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
        if vim && let Some(index) = state.vim.block_cursor(cursor_index, text) {
            overlays::block_cursor(ui, galley, galley_pos, index);
        }
        if features.inlay_hints {
            overlays::inlay_hints(ui, galley, galley_pos, language, text);
        }
        if features.hover || features.diagnostics {
            overlays::hover_tooltip(
                response,
                galley,
                galley_pos,
                text,
                language,
                underlines,
                features.hover,
            );
        }
        if features.signature_help
            && response.has_focus()
            && insert_like
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
pub(crate) fn editor_id(id_salt: impl egui::AsIdSalt) -> egui::Id {
    egui::Id::unique(("code_editor", id_salt))
}

/// Moves the cursor of the editor `id` (applied on the next frame).
///
/// e.g., jumping to a diagnostic from a problems list. Get id from [ditor_id].
pub(crate) fn set_cursor(ctx: &egui::Context, id: egui::Id, index: usize) {
    set_selection(ctx, id, index..index);
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
