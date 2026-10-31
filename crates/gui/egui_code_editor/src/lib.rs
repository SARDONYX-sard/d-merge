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
mod find;
mod hover;
pub mod keymap;
pub mod language;
pub mod languages;
mod layout;
mod overlays;
mod quick_fix;
mod search;
mod status;
pub mod text_pos;

use egui::text::{CCursor, CCursorRange};

use self::{
    completion::CompletionPopup,
    find::FindBar,
    hover::{HoverCtx, HoverPopup},
    keymap::{KeymapOutput, Scroll, View},
    quick_fix::QuickFixMenu,
    text_pos::LinePos,
};
pub use self::{
    keymap::{Keymap, KeymapState, StatusLine, StatusMode, VimOptions},
    language::{
        CompletionItem, CompletionList, Diagnostic, Language, QuickFix, Severity, SignatureHelp,
    },
    layout::line_job,
    status::status_line,
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
    quick_fix: QuickFixMenu,
    /// `Ctrl+F` / `Ctrl+G` bar (Normal keymap).
    find: FindBar,
    /// State of the key binding styles (e.g., Vim mode and registers).
    pub keymap: KeymapState,
    /// Lines visible last frame (Vim `H`, `Ctrl+D`, ...).
    view: View,
    /// The editor had the keyboard focus last frame.
    had_focus: bool,
}

impl CodeEditorState {
    /// Matches to highlight: the find bar (Normal) or the last `/` search (Vim).
    fn search_highlight(&self, keymap: Keymap) -> Option<search::Matcher> {
        match keymap {
            Keymap::Normal => self.find.highlight(),
            Keymap::Vim => self.keymap.search_highlight(keymap),
        }
    }
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
    line_numbers: bool,
    whitespace: bool,
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
            line_numbers: true,
            whitespace: true,
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

    /// Show the line number gutter (default `true`).
    /// Show `·` / `→` for spaces / tabs and tint the indent levels (default `true`).
    pub const fn whitespace(mut self, show: bool) -> Self {
        self.whitespace = show;
        self
    }

    pub const fn line_numbers(mut self, show: bool) -> Self {
        self.line_numbers = show;
        self
    }

    /// Minimum height in text rows (default 30).
    pub const fn desired_rows(mut self, rows: usize) -> Self {
        self.desired_rows = rows;
        self
    }

    pub fn show(self, ui: &mut egui::Ui, state: &mut CodeEditorState) -> CodeEditorOutput {
        let Self {
            id,
            text,
            language,
            diagnostics,
            features,
            keymap,
            desired_rows,
            line_numbers,
            whitespace,
        } = self;
        let mut has_focus = ui.memory(|m| m.has_focus(id));
        // egui drops a widget's focus on `Esc` before we run, but Vim needs that key
        // (e.g., to leave Visual mode): take the focus back.
        if !has_focus
            && state.had_focus
            && keymap == Keymap::Vim
            && ui.input(|i| i.key_pressed(egui::Key::Escape))
        {
            ui.memory_mut(|m| m.request_focus(id));
            has_focus = true;
        }

        // ── 1. Keys owned by the popups / keymap (must run before the TextEdit) ──
        state.hover.handle_input(ui, id); // before the keymap swallows the key events
        // `Ctrl+.`: quick fixes at the cursor; while the menu is open it owns the arrow keys.
        let mut fix = state.quick_fix.handle_keys(ui);
        if has_focus
            && ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Period))
            && let Some(index) = egui::TextEdit::load_state(ui.ctx(), id)
                .and_then(|s| s.cursor.char_range())
                .map(|r| r.primary.index.0)
        {
            let pos = LinePos::from_char_index(text, index);
            state.quick_fix.open(language.quick_fixes(text, pos.line, pos.column));
        }
        state.find.handle_keys(keymap, ui, id, text, has_focus);
        let accept_key = state.completion.handle_keys(ui);
        let keys = if has_focus && !state.quick_fix.is_open() {
            state.keymap.handle(keymap, ui, id, text, state.view)
        } else {
            KeymapOutput::default()
        };
        let mut changed = keys.changed;
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
        let search = state.search_highlight(keymap);
        let mut layouter = |ui: &egui::Ui, buf: &dyn egui::TextBuffer, _wrap_width: f32| {
            layout::layout(ui, buf.as_str(), language, underlines, search.as_ref())
        };
        // e.g., Vim Normal mode draws a block cursor: no blinking I-beam then.
        let hide_text_cursor = state.keymap.hides_text_cursor(keymap);
        // Gutter for the line numbers on the left (drawn after the text is laid out, below).
        let gutter = line_numbers.then(|| overlays::gutter_width(ui, text));
        let (gutter_left, output) = ui
            .horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                let gutter_left = ui.cursor().left();
                if let Some(width) = gutter {
                    ui.add_space(width);
                }
                let output = ui.scope(|ui| {
                    if keymap == Keymap::Vim {
                        // Vim selections often span many lines: a lighter color keeps the text readable.
                        let selection = &mut ui.visuals_mut().selection.bg_fill;
                        *selection = selection.gamma_multiply(0.55);
                    }
                    if hide_text_cursor {
                        let cursor = &mut ui.visuals_mut().text_cursor;
                        cursor.stroke = egui::Stroke::NONE;
                        cursor.blink = false;
                    }
                    egui::TextEdit::multiline(text)
                        .id(id)
                        .code_editor()
                        .desired_width(f32::INFINITY)
                        .desired_rows(desired_rows)
                        .layouter(&mut layouter)
                        .show(ui)
                });
                (gutter_left, output.inner)
            })
            .inner;

        let response = &output.response.response;
        changed |= response.changed();
        // egui reports the cursor only while the editor has the focus; a jump from outside (e.g., a
        // problems list, `set_cursor`) arrives one frame before the focus does, so read the state.
        let cursor_range = output.cursor_range.or_else(|| {
            egui::TextEdit::load_state(ui.ctx(), id).and_then(|state| state.cursor.char_range())
        });
        let cursor_index = cursor_range.map(|range| range.primary.index.0);
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
        if whitespace {
            overlays::whitespace(ui, galley, galley_pos);
        }
        if let Some(anchor) = anchor
            && let Some(clicked) = state.quick_fix.ui(ui.ctx(), id, anchor)
        {
            fix = Some(clicked);
        }
        if let Some(QuickFix { line, columns, new_text, .. }) = fix.take() {
            let line_start = LinePos { line, column: 0 }.to_char_index(text);
            let start = text_pos::char_to_byte(text, line_start + columns.start);
            let end = text_pos::char_to_byte(text, line_start + columns.end);
            text.replace_range(start..end, &new_text);
            set_cursor(ui.ctx(), id, line_start + columns.start);
            changed = true;
        }
        if let Some(width) = gutter {
            let current = cursor.map(|c| c.line);
            overlays::line_numbers(
                ui,
                galley,
                galley_pos,
                gutter_left..gutter_left + width,
                current,
            );
        }
        if let Some(ranges) = state.keymap.block_selection(keymap, text) {
            overlays::block_selection(ui, galley, galley_pos, &ranges);
        }
        // Requested by `set_cursor` (e.g., a click in a problems list): center it.
        // Moved by the keymap (e.g., Vim `G`, `n`): just bring it into view.
        // (The `TextEdit` only scrolls for cursor moves it makes itself.)
        if let Some(cursor) = anchor {
            if ui.data_mut(|d| d.remove_temp::<bool>(scroll_request_id(id))).unwrap_or(false) {
                ui.scroll_to_rect(cursor, Some(egui::Align::Center));
            } else {
                match keys.scroll {
                    Some(Scroll::IntoView) => ui.scroll_to_rect(cursor, None),
                    Some(Scroll::Align(align)) => ui.scroll_to_rect(cursor, Some(align)),
                    Some(Scroll::Lines(lines)) => {
                        let row_height =
                            galley.rows.first().map_or(16.0, |row| row.rect().height());
                        #[expect(clippy::cast_precision_loss, reason = "a few lines")]
                        ui.scroll_with_delta(egui::vec2(0.0, -(lines as f32) * row_height));
                    }
                    None => {}
                }
            }
        }
        state.find.ui(ui, id, response.rect.intersect(ui.clip_rect()), text, (galley, galley_pos));
        state.view = visible_lines(ui, galley, galley_pos);
        state.had_focus = response.has_focus();
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
            if keys.show_hover
                && let Some(index) = cursor_index
            {
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

        CodeEditorOutput { changed, cursor, command: keys.command }
    }
}

/// The lines of `galley` **fully** inside the visible area (lines are never wrapped: row = line).
///
/// Half-visible rows do not count: Vim `L` on one would scroll the view to show it, so `H` / `L`
/// pressed in turn would creep down a line each time.
fn visible_lines(ui: &egui::Ui, galley: &egui::Galley, galley_pos: egui::Pos2) -> View {
    const EPS: f32 = 0.5; // rounding of the scroll offset
    let clip = ui.clip_rect();
    let rows = &galley.rows;
    let first =
        rows.iter().position(|r| galley_pos.y + r.rect().min.y >= clip.top() - EPS).unwrap_or(0);
    let last = rows
        .iter()
        .rposition(|r| galley_pos.y + r.rect().max.y <= clip.bottom() + EPS)
        .unwrap_or(first)
        .max(first);
    View { first_line: first, lines: last - first + 1 }
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
pub(crate) fn scroll_request_id(id: egui::Id) -> egui::Id {
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
