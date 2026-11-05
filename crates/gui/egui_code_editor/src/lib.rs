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
//! if let Some(navigation) = output.navigation { /* open what `Language::definition` found */ }
//! if output.action == Some("save") { /* picked in the `Ctrl+Shift+P` palette (`.actions(..)`) */ }
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
mod palette;
mod quick_fix;
mod search;
mod status;
pub mod text_pos;
mod ui;

use egui::text::{CCursor, CCursorRange};

use self::{
    completion::CompletionPopup,
    find::FindBar,
    hover::{HoverCtx, HoverPopup},
    keymap::{KeymapOutput, Scroll, View},
    palette::{Palette, PaletteContext},
    quick_fix::QuickFixMenu,
    text_pos::LinePos,
    ui::palette::PaletteInput,
};
pub use self::{
    keymap::{Keymap, KeymapState, StatusLine, StatusMode, VimOptions},
    language::{
        CompletionItem, CompletionList, Diagnostic, Language, Location, QuickFix, References, Severity, SignatureHelp,
    },
    layout::line_job,
    palette::{EditorAction, EditorLabels},
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
    /// Vim `gd`: go to the definition of the symbol under the cursor.
    GoToDefinition,
    /// Vim `gD`: find the references of the symbol under the cursor.
    FindReferences,
    /// Vim `Ctrl+O` past the oldest jump of this text: back to the previous document / object.
    JumpBack,
    /// Vim `Ctrl+I` past the newest jump: forward again.
    JumpForward,
    /// Vim `<Space>e`: move the keyboard to the host's file / object tree.
    ToggleExplorer,
}

/// A resolved `F12` / `Shift+F12` / Vim `gd` / `gD` / `Ctrl+click` / right-click request:
/// the host opens the place, or lists the references.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Navigation<'a> {
    Definition(Location<'a>),
    References(References<'a>),
}

/// Per-document state that is not part of the text. Keep one per open document.
#[derive(Debug, Default)]
pub struct CodeEditorState {
    completion: CompletionPopup,
    hover: HoverPopup,
    quick_fix: QuickFixMenu,
    /// `Ctrl+F` / `Ctrl+G` bar (Normal keymap).
    find: FindBar,
    /// `Ctrl+Shift+P`.
    palette: Palette,
    /// State of the key binding styles (e.g., Vim mode and registers).
    pub keymap: KeymapState,
    /// Lines visible last frame (Vim `H`, `Ctrl+D`, ...).
    view: View,
    /// The editor had the keyboard focus last frame.
    had_focus: bool,
    layout: layout::LayoutCache,
    /// Where the right-click menu was opened (the pointer moves while it is open).
    menu_target: Option<LinePos>,
}

impl CodeEditorState {
    /// Opens what the keys or the palette asked for: the find bar, the quick fix menu.
    fn open_requested_menus(
        &mut self,
        ctx: &egui::Context,
        id: egui::Id,
        text: &str,
        language: &dyn Language,
        triggers: palette::Triggers,
    ) {
        if triggers.find {
            self.find.open_find(ctx, id, text);
        } else if triggers.go_to_line {
            self.find.open_go_to_line(ctx, id);
        }
        if triggers.quick_fix
            && let Some(range) = egui::TextEdit::load_state(ctx, id).and_then(|state| state.cursor.char_range())
        {
            let pos = LinePos::from_char_index(text, range.primary.index.0);
            self.quick_fix.open(language.quick_fixes(text, pos.line, pos.column));
        }
    }

    /// Matches to highlight: the find bar (Normal) or the last `/` search (Vim).
    fn search_highlight(&self, keymap: Keymap) -> Option<search::Matcher> {
        match keymap {
            Keymap::Normal => self.find.highlight(),
            Keymap::Vim => self.keymap.search_highlight(keymap),
        }
    }
}

/// What happened this frame.
///
/// `'a`: [`Self::navigation`] borrows from the language given to [`CodeEditor::language`].
#[derive(Debug)]
pub struct CodeEditorOutput<'a> {
    /// The text was edited (typing, completion, formatting, keymap commands).
    pub changed: bool,
    /// Cursor position, if the editor has one.
    pub cursor: Option<LinePos>,
    /// Request for the host, e.g., Vim `:w`.
    pub command: Option<EditorCommand>,
    /// The text area's response (e.g., to add a right-click menu).
    pub response: egui::Response,
    /// The char under the mouse pointer (a right click does not move the cursor there).
    pub pointer: Option<LinePos>,
    /// A go-to-definition / find-references result for the host to show.
    pub navigation: Option<Navigation<'a>>,
    /// The id of the [`EditorAction`] run from the command palette.
    pub action: Option<&'a str>,
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
    labels: EditorLabels<'a>,
    actions: &'a [EditorAction<'a>],
}

impl std::fmt::Debug for CodeEditor<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CodeEditor").field("id", &self.id).field("keymap", &self.keymap).finish_non_exhaustive()
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
            labels: EditorLabels::default(),
            actions: &[],
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

    /// Translated texts of the right-click menu and the command palette.
    pub const fn labels(mut self, labels: EditorLabels<'a>) -> Self {
        self.labels = labels;
        self
    }

    /// The host's commands in the palette (`Ctrl+Shift+P`). See [`CodeEditorOutput::action`].
    pub const fn actions(mut self, actions: &'a [EditorAction<'a>]) -> Self {
        self.actions = actions;
        self
    }

    /// Show `·` / `→` for spaces / tabs and tint the indent levels (default `true`).
    pub const fn whitespace(mut self, show: bool) -> Self {
        self.whitespace = show;
        self
    }

    /// Show the line number gutter (default `true`).
    pub const fn line_numbers(mut self, show: bool) -> Self {
        self.line_numbers = show;
        self
    }

    /// Minimum height in text rows (default 30).
    pub const fn desired_rows(mut self, rows: usize) -> Self {
        self.desired_rows = rows;
        self
    }

    pub fn show(self, ui: &mut egui::Ui, state: &mut CodeEditorState) -> CodeEditorOutput<'a> {
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
            labels,
            actions,
        } = self;
        let has_focus = current_focus(ui, id, keymap, state.had_focus);

        // ── 1. Keys owned by the popups / keymap (must run before the TextEdit) ──
        state.hover.handle_input(ui, id); // before the keymap swallows the key events
        let context = PaletteContext { keymap, features, inserting: state.keymap.is_inserting(keymap) };
        let palette_input = PaletteInput { id, has_focus, context, labels: &labels, actions };
        let picked = ui::palette::show(&mut state.palette, ui, palette_input);
        let triggers = ui::palette::read_triggers(ui, has_focus).with_picked(picked);
        state.open_requested_menus(ui.ctx(), id, text, language, triggers);
        // While the quick fix menu is open it owns the arrow keys.
        let mut fix = state.quick_fix.handle_keys(ui);
        state.find.handle_keys(keymap, ui, id, text, has_focus);
        let accept_key = state.completion.handle_keys(ui);
        let keys = if has_focus && !state.quick_fix.is_open() {
            state.keymap.handle(keymap, ui, id, text, state.view)
        } else {
            KeymapOutput::default()
        };
        let mut changed = keys.changed;
        let navigate_keys = match keys.command {
            Some(EditorCommand::GoToDefinition) => Some(false),
            Some(EditorCommand::FindReferences) => Some(true),
            _ => triggers.navigate, // (Vim's `gd` / `gD` arrive as commands, the rest as keys.)
        };
        let inserting = state.keymap.is_inserting(keymap);

        let ctrl_space = features.completion && inserting && triggers.completion;
        if features.format
            && triggers.format
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
            layout::cached(&mut state.layout, ui, buf.as_str(), language, underlines, search.as_ref())
        };
        // e.g., Vim Normal mode draws a block cursor: no blinking I-beam then.
        let hide_text_cursor = state.keymap.hides_text_cursor(keymap);
        // Gutter for the line numbers on the left (drawn after the text is laid out, below).
        let gutter = line_numbers.then(|| overlays::gutter_width(ui, text));
        let look = TextLook { desired_rows, dim_selection: keymap == Keymap::Vim, hide_text_cursor, gutter };
        let (gutter_left, output) = show_text_edit(ui, id, text, look, &mut layouter);

        let response = &output.response.response;
        changed |= response.changed();
        // egui reports the cursor only while the editor has the focus; a jump from outside (e.g., a
        // problems list, `set_cursor`) arrives one frame before the focus does, so read the state.
        let cursor_range = output
            .cursor_range
            .or_else(|| egui::TextEdit::load_state(ui.ctx(), id).and_then(|state| state.cursor.char_range()));
        let cursor_index = cursor_range.map(|range| range.primary.index.0);
        let mut cursor = cursor_index.map(|i| LinePos::from_char_index(text, i));

        // ── 3. Completion ────────────────────────────────────────────────────────
        if features.completion && inserting {
            update_completion(&mut state.completion, ui, language, text, cursor, (ctrl_space, response.changed()));
        } else {
            state.completion.close();
        }

        let anchor =
            cursor_index.map(|i| output.galley.pos_from_cursor(CCursor::new(i)).translate(output.galley_pos.to_vec2()));
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
        if let Some(fix) = fix.take() {
            let at = apply_quick_fix(text, &fix);
            set_cursor(ui.ctx(), id, at);
            changed = true;
        }
        if let Some(width) = gutter {
            let current = cursor.map(|c| c.line);
            overlays::line_numbers(ui, galley, galley_pos, gutter_left..gutter_left + width, current);
        }
        if let Some(ranges) = state.keymap.block_selection(keymap, text) {
            overlays::block_selection(ui, galley, galley_pos, &ranges);
        }
        if let Some(cursor) = anchor {
            scroll_to_cursor(ui, id, cursor, galley, keys.scroll);
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
            if (keys.show_hover || triggers.hover)
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

        let pointer = response.hover_pos().map(|pos| {
            let index = galley.cursor_from_pos(pos - galley_pos).index.0;
            LinePos::from_char_index(text, index)
        });

        // ── 5. Go to definition / find references ────────────────────────────────
        let keyboard = navigate_keys.and_then(|references| Some((cursor?, references)));
        let menu_labels = (labels.go_to_definition, labels.find_references);
        let navigation =
            resolve_navigation(response, language, text, pointer, keyboard, &mut state.menu_target, menu_labels);
        let action = picked.and_then(palette::Picked::host_action);

        CodeEditorOutput {
            changed,
            cursor,
            command: keys.command,
            response: response.clone(),
            pointer,
            navigation,
            action,
        }
    }
}

/// Scrolls for a cursor moved from outside the `TextEdit` (it only scrolls for its own moves):
/// - requested by [`set_cursor`] (e.g., a click in a problems list): centers it
/// - moved by the keymap (e.g., Vim `G`, `n`): just brings it into view
fn scroll_to_cursor(ui: &egui::Ui, id: egui::Id, cursor: egui::Rect, galley: &egui::Galley, scroll: Option<Scroll>) {
    if ui.data_mut(|d| d.remove_temp::<bool>(scroll_request_id(id))).unwrap_or(false) {
        ui.scroll_to_rect(cursor, Some(egui::Align::Center));
        return;
    }
    match scroll {
        // Keymap moves (Vim `gg`, `G`, `zz`, ...) jump at once: an animation is slow over a long
        // distance.
        Some(Scroll::IntoView) => ui.scroll_to_rect_animation(cursor, None, instant()),
        Some(Scroll::Align(align)) => ui.scroll_to_rect_animation(cursor, Some(align), instant()),
        Some(Scroll::Lines(lines)) => {
            let row_height = galley.rows.first().map_or(16.0, |row| row.rect().height());
            #[expect(clippy::cast_precision_loss, reason = "a few lines")]
            ui.scroll_with_delta(egui::vec2(0.0, -(lines as f32) * row_height));
        }
        None => {}
    }
}

/// Whether the editor has the keyboard.
///
/// egui drops a widget's focus on `Esc` before we run, but Vim needs that key (e.g., to leave
/// Visual mode): the focus is taken back then.
fn current_focus(ui: &egui::Ui, id: egui::Id, keymap: Keymap, had_focus: bool) -> bool {
    let has_focus = ui.memory(|m| m.has_focus(id));
    if !has_focus && had_focus && keymap == Keymap::Vim && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        ui.memory_mut(|m| m.request_focus(id));
        return true;
    }
    has_focus
}

/// How the `TextEdit` looks. See [`show_text_edit`].
#[derive(Debug, Clone, Copy)]
struct TextLook {
    desired_rows: usize,
    /// Vim selections often span many lines: a lighter color keeps the text readable.
    dim_selection: bool,
    /// e.g., Vim Normal mode draws a block cursor instead.
    hide_text_cursor: bool,
    /// Room to leave for the line numbers.
    gutter: Option<f32>,
}

/// Shows the `TextEdit`. Returns the gutter's left edge and the output.
fn show_text_edit(
    ui: &mut egui::Ui,
    id: egui::Id,
    text: &mut String,
    look: TextLook,
    layouter: &mut dyn FnMut(&egui::Ui, &dyn egui::TextBuffer, f32) -> std::sync::Arc<egui::Galley>,
) -> (f32, egui::text_edit::TextEditOutput) {
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
                .layouter(layouter)
                .show(ui)
        });
        (gutter_left, output.inner)
    })
    .inner
}

/// Refreshes the completion list. `(requested, typed)`: `Ctrl+Space` was pressed, the text changed.
fn update_completion(
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
fn apply_quick_fix(text: &mut String, fix: &QuickFix) -> usize {
    let line_start = LinePos { line: fix.line, column: 0 }.to_char_index(text);
    let start = text_pos::char_to_byte(text, line_start + fix.columns.start);
    let end = text_pos::char_to_byte(text, line_start + fix.columns.end);
    text.replace_range(start..end, &fix.new_text);
    line_start + fix.columns.start
}

/// Asks the language about what the keyboard, `Ctrl+click` or the right-click menu requested.
///
/// `keyboard`: `(cursor, references)` from `F12` / Vim `gd` and friends.
fn resolve_navigation<'a>(
    response: &egui::Response,
    language: &'a dyn Language,
    text: &str,
    pointer: Option<LinePos>,
    keyboard: Option<(LinePos, bool)>,
    menu_target: &mut Option<LinePos>,
    labels: (&str, &str),
) -> Option<Navigation<'a>> {
    let ctrl_click = response.clicked() && response.ctx.input(|i| i.modifiers.command);
    let at_pointer = pointer.filter(|_| ctrl_click).map(|pos| (pos, false));
    if response.secondary_clicked() {
        // Only over a symbol: elsewhere there is nothing to offer.
        *menu_target = pointer.filter(|pos| language.definition(text, pos.line, pos.column).is_some());
    }
    let from_menu = menu_target.and_then(|pos| navigation_menu(response, language, text, pos, labels));
    let (pos, references) = keyboard.or(at_pointer).or(from_menu)?;
    if references {
        language.references(text, pos.line, pos.column).map(Navigation::References)
    } else {
        language.definition(text, pos.line, pos.column).map(Navigation::Definition)
    }
}

/// The right-click menu for the symbol at `pos`. Returns `(pos, references)` if an item was clicked.
fn navigation_menu(
    response: &egui::Response,
    language: &dyn Language,
    text: &str,
    pos: LinePos,
    labels: (&str, &str),
) -> Option<(LinePos, bool)> {
    let mut request = None;
    response.context_menu(|ui| {
        if let Some(definition) = language.definition(text, pos.line, pos.column) {
            ui.weak(&*definition.document);
        }
        if ui.button(format!("{}  (F12)", labels.0)).clicked() {
            request = Some((pos, false));
            ui.close();
        }
        if ui.button(format!("{}  (Shift+F12)", labels.1)).clicked() {
            request = Some((pos, true));
            ui.close();
        }
    });
    request
}

/// The lines of `galley` **fully** inside the visible area (lines are never wrapped: row = line).
///
/// Half-visible rows do not count: Vim `L` on one would scroll the view to show it, so `H` / `L`
/// pressed in turn would creep down a line each time.
fn visible_lines(ui: &egui::Ui, galley: &egui::Galley, galley_pos: egui::Pos2) -> View {
    const EPS: f32 = 0.5; // rounding of the scroll offset
    let clip = ui.clip_rect();
    let rows = &galley.rows;
    let first = rows.iter().position(|r| galley_pos.y + r.rect().min.y >= clip.top() - EPS).unwrap_or(0);
    let last =
        rows.iter().rposition(|r| galley_pos.y + r.rect().max.y <= clip.bottom() + EPS).unwrap_or(first).max(first);
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
    state.cursor.set_char_range(Some(CCursorRange::two(CCursor::new(range.start), CCursor::new(range.end))));
    state.store(ctx, id);
    ctx.memory_mut(|mem| mem.request_focus(id));
}

/// No scroll animation.
fn instant() -> egui::style::ScrollAnimation {
    egui::style::ScrollAnimation::none()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quick_fix_replaces_columns_of_its_line() {
        let mut text = "first\nabc def".to_owned();
        let fix = QuickFix { title: String::new(), line: 1, columns: 4..7, new_text: "XY".to_owned() };
        // `first\n` is 6 chars, then `abc ` is 4.
        assert_eq!(apply_quick_fix(&mut text, &fix), 10);
        assert_eq!(text, "first\nabc XY");
    }
}
