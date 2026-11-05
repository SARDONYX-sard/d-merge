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
mod edit_ops;
mod find;
mod hover;
mod inline_diff;
pub mod keymap;
pub mod language;
pub mod languages;
mod layout;
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
    ui::{
        context_menu::{self, MenuAction, MenuContext},
        overlays,
        palette::PaletteInput,
    },
};
pub use self::{
    inline_diff::{CharChange, DiffHunk, InlineDiff},
    keymap::{Keymap, KeymapState, StatusLine, StatusMode, VimOptions},
    language::{
        CompletionItem, CompletionList, Diagnostic, Indent, Language, Location, QuickFix, References, Severity,
        SignatureHelp,
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
    /// Vim `<Space>l`, `gt`, `:tabn`: the host's next tab.
    NextTab,
    /// Vim `<Space>h`, `gT`, `:tabp`: the host's previous tab.
    PreviousTab,
    /// Vim `<Space>c`, `:tabc`, `:q`: close the host's tab.
    CloseTab,
    /// Vim `<Space>p`, `:preview`: show or hide what the host shows beside the editor.
    TogglePreview,
    /// Vim `<Space>d`, `:diff`: show or hide the changes inside the editor.
    ToggleInlineDiff,
}

/// A resolved `F12` / `Shift+F12` / Vim `gd` / `gD` / `Ctrl+click` / right-click request:
/// the host opens the place, or lists the references.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Navigation<'a> {
    Definition(Location<'a>),
    References(References<'a>),
}

impl EditorCommand {
    const ALL: [Self; 11] = [
        Self::Save,
        Self::GoToDefinition,
        Self::FindReferences,
        Self::JumpBack,
        Self::JumpForward,
        Self::ToggleExplorer,
        Self::NextTab,
        Self::PreviousTab,
        Self::CloseTab,
        Self::TogglePreview,
        Self::ToggleInlineDiff,
    ];

    const fn action_id(self) -> &'static str {
        match self {
            Self::Save => "save",
            Self::GoToDefinition => "go_to_definition",
            Self::FindReferences => "find_references",
            Self::JumpBack => "jump_back",
            Self::JumpForward => "jump_forward",
            Self::ToggleExplorer => "toggle_explorer",
            Self::NextTab => "next_tab",
            Self::PreviousTab => "previous_tab",
            Self::CloseTab => "close_tab",
            Self::TogglePreview => "toggle_preview",
            Self::ToggleInlineDiff => "toggle_inline_diff",
        }
    }

    fn from_action_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|command| command.action_id() == id)
    }

    /// This command in the palette (`Ctrl+Shift+P`). Picking it comes back as
    /// [`CodeEditorOutput::command`], the same as its key.
    pub const fn action(self, label: &str) -> EditorAction<'_> {
        EditorAction { id: self.action_id(), label }
    }
}

/// `Ctrl+PageDown`: `Some(true)`, the next tab. `Ctrl+PageUp`: `Some(false)`, the previous one. As in
/// VS Code, in every keymap. For a host with tabs to ask before showing its editor.
pub fn tab_switch_keys(ui: &egui::Ui) -> Option<bool> {
    use egui::{Key, Modifiers};
    ui.input_mut(|input| {
        if input.consume_key(Modifiers::COMMAND, Key::PageDown) {
            Some(true)
        } else if input.consume_key(Modifiers::COMMAND, Key::PageUp) {
            Some(false)
        } else {
            None
        }
    })
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
    /// The symbol the right-click menu was opened on (the pointer moves while it is open).
    menu_target: Option<LinePos>,
    /// The selection the right button was pressed on, while that button is down.
    held_selection: Option<CCursorRange>,
    /// What one level of indentation is, for the status bar (drawn before the editor).
    indent: Indent,
    /// The Vim key that waits for another, and since when.
    which_key_waiting: ui::which_key::Waiting,
    /// `(errors, warnings)` for the status bar.
    status_problems: (usize, usize),
    /// The cursor and how many chars are selected, for the status bar (drawn before the editor).
    status_cursor: Option<(LinePos, usize)>,
    /// The menu's Paste asked the OS for the clipboard, at this time (seconds). The text arrives as
    /// an event in a later frame.
    pending_paste: Option<f64>,
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
    /// The `↶` of this [`InlineDiff::hunks`] entry was clicked: the host puts it back.
    pub inline_revert: Option<usize>,
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
    inline_diff: Option<&'a InlineDiff>,
    read_only: bool,
    host_commands: &'a [EditorCommand],
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
            inline_diff: None,
            read_only: false,
            host_commands: &[],
        }
    }

    /// What the host acts on, so Vim's list of keys offers them. e.g., a host without a tree has no
    /// [`EditorCommand::ToggleExplorer`].
    pub const fn host_commands(mut self, host_commands: &'a [EditorCommand]) -> Self {
        self.host_commands = host_commands;
        self
    }

    /// Everything but changing the text: the cursor, selecting, copying, hover, go to definition.
    /// `changed` stays `false`. Keymaps edit the text directly, so what they did is undone at the end
    /// of the frame.
    pub const fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
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

    /// Shows the changes inside the text, like Monaco's inline diff. See [`InlineDiff`].
    pub const fn inline_diff(mut self, diff: Option<&'a InlineDiff>) -> Self {
        self.inline_diff = diff;
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

    /// One frame; see the crate docs for the order of the steps and why.
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
            inline_diff,
            read_only,
            host_commands,
        } = self;
        let original = read_only.then(|| text.clone());
        let frame = EditorFrame {
            id,
            language,
            underlines: if features.diagnostics { diagnostics } else { &[] },
            features,
            keymap,
            inline_diff,
        };

        state.indent = language.indent();
        state.keymap.vim_options_mut().indent = state.indent;
        let input = read_input(frame, ui, state, text, &labels, actions);
        tab_key_follows_indent(ui, id, state.indent);
        let mut changed = input.keys.changed | input.pasted | {
            let asked = input.triggers.format || input.keys.format;
            apply_format(frame, ui, text, asked)
        };
        let navigate_keys = match input.keys.command {
            Some(EditorCommand::GoToDefinition) => Some(false),
            Some(EditorCommand::FindReferences) => Some(true),
            _ => input.triggers.navigate, // (Vim's `gd` / `gD` arrive as commands, the rest as keys.)
        };
        let inserting = state.keymap.is_inserting(keymap);

        // Gutter for the line numbers on the left (drawn after the text is laid out, below).
        let gutter = line_numbers.then(|| overlays::gutter_width(ui, text));
        let (gutter_left, output) = show_text(frame, ui, state, text, desired_rows, gutter);
        let response = &output.response.response;
        let placed = (&*output.galley, output.galley_pos);
        changed |= response.changed();

        // egui reports the cursor only while the editor has the focus; a jump from outside (e.g., a
        // problems list, `set_cursor`) arrives one frame before the focus does, so read the state.
        let cursor_range = output
            .cursor_range
            .or_else(|| egui::TextEdit::load_state(ui.ctx(), id).and_then(|state| state.cursor.char_range()));
        let cursor_index = cursor_range.map(|range| range.primary.index.0);
        let mut cursor = cursor_index.map(|i| LinePos::from_char_index(text, i));
        let anchor = cursor_index.map(|i| placed.0.pos_from_cursor(CCursor::new(i)).translate(placed.1.to_vec2()));

        let completion_request = (input.triggers.completion, response.changed());
        update_completion_popup(frame, ui, state, text, cursor, inserting, completion_request);
        changed |= accept_completion(ui, id, state, text, anchor, input.accept_key, &mut cursor, response);

        let inline_revert =
            inline_diff.and_then(|diff| overlays::inline_diff(ui, placed.0, placed.1, diff, id, labels.revert_change));
        draw_text_background(ui, placed, cursor.map(|cursor| cursor.line), whitespace);
        changed |= run_quick_fix(ui.ctx(), id, state, text, anchor, input.fix);
        draw_line_decorations(
            ui,
            state,
            keymap,
            text,
            placed,
            gutter.map(|width| gutter_left..gutter_left + width),
            cursor,
        );
        if let Some(cursor) = anchor {
            scroll_to_cursor(ui, id, cursor, placed.0, input.keys.scroll);
        }
        let visible = response.rect.intersect(ui.clip_rect());
        state.find.ui(ui, id, visible, text, placed);
        show_which_key(ui, id, state, keymap, visible, host_commands);
        state.view = visible_lines(ui, placed.0, placed.1);
        state.had_focus = response.has_focus();
        if let Some(index) = state.keymap.block_cursor(keymap, cursor_index, text) {
            overlays::block_cursor(ui, placed.0, placed.1, index);
        }
        if features.inlay_hints {
            overlays::inlay_hints(ui, placed.0, placed.1, language, text);
        }
        let open_hover = cursor_index.filter(|_| input.keys.show_hover || input.triggers.hover); // e.g., Vim `K`
        show_hover(frame, ui, state, text, response, placed, open_hover);
        if inserting && !state.completion.is_open() && response.has_focus() {
            show_signature(frame, ui, text, cursor, anchor);
        }

        let pointer = response.hover_pos().map(|pos| {
            let index = placed.0.cursor_from_pos(pos - placed.1).index.0;
            LinePos::from_char_index(text, index)
        });
        let selection = cursor_range.map_or(0..0, |range| sorted_chars(&range));
        state.status_cursor = cursor.map(|cursor| (cursor, selection.len()));
        let errors = frame.underlines.iter().filter(|problem| problem.severity == Severity::Error).count();
        state.status_problems = (errors, frame.underlines.len() - errors);
        let menu_action = context_menu(response, language, text, pointer, &mut state.menu_target, &labels, &selection);
        changed |= run_menu_action(menu_action, ui, id, state, text, &selection, response);
        let keyboard = navigate_keys.and_then(|references| Some((cursor?, references)));
        let from_menu = match menu_action {
            Some(MenuAction::Navigate { pos, references }) => Some((pos, references)),
            _ => None,
        };
        let navigation = resolve_navigation(response, language, text, pointer, keyboard.or(from_menu));

        if let Some(original) = original {
            *text = original;
            changed = false;
        }

        CodeEditorOutput {
            changed,
            cursor,
            // A command of the palette asks for the same as its key.
            command: input.keys.command.or_else(|| {
                input.picked.and_then(palette::Picked::host_action).and_then(EditorCommand::from_action_id)
            }),
            response: response.clone(),
            pointer,
            navigation,
            action: input.picked.and_then(palette::Picked::host_action),
            inline_revert,
        }
    }
}

/// What every step of a frame needs. Does not change during the frame.
#[derive(Clone, Copy)]
struct EditorFrame<'a> {
    id: egui::Id,
    language: &'a dyn Language,
    /// The problems to underline (none with [`EditorFeatures::diagnostics`] off).
    underlines: &'a [Diagnostic],
    features: EditorFeatures,
    keymap: Keymap,
    inline_diff: Option<&'a InlineDiff>,
}

/// What the keys of this frame asked for. See [`read_input`].
struct FrameInput<'a> {
    picked: Option<palette::Picked<'a>>,
    triggers: palette::Triggers,
    /// `Enter` / `Tab` in the quick fix menu.
    fix: Option<QuickFix>,
    /// `Enter` / `Tab` in the completion popup.
    accept_key: bool,
    /// The text changed by the menu's Paste.
    pasted: bool,
    keys: KeymapOutput,
}

/// Step 1: the keys owned by the popups and the keymap. Must run before the `TextEdit`.
fn read_input<'a>(
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
fn apply_format(frame: EditorFrame<'_>, ui: &egui::Ui, text: &mut String, asked: bool) -> bool {
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
fn keep_cursor_line(ctx: &egui::Context, id: egui::Id, text: &str, cursor: LinePos) {
    let mut edit = egui::TextEdit::load_state(ctx, id).unwrap_or_default();
    let index = cursor.to_char_index(text);
    edit.cursor.set_char_range(Some(CCursorRange::one(CCursor::new(index))));
    edit.store(ctx, id);
}

/// egui's `TextEdit` always inserts a tab character for `Tab`: where the language indents with spaces,
/// it gets those instead. What the keymap or a popup wanted the key for is gone from the input by now.
fn tab_key_follows_indent(ui: &egui::Ui, id: egui::Id, indent: Indent) {
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

/// The list of what can follow a Vim key that waits for another, once it has waited for a moment.
fn show_which_key(
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

/// Step 2: the `TextEdit`, with the syntax colors, underlines and search matches.
fn show_text(
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
fn keep_selection_for_menu(
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
const fn selection_covers(selection: &CCursorRange, at: usize) -> bool {
    let range = sorted_chars(selection);
    range.start < range.end && range.start <= at && at <= range.end
}

/// Step 3: opens, updates or closes the completion list.
///
/// `(requested, typed)`: `Ctrl+Space` was pressed, the text changed.
fn update_completion_popup(
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
fn accept_completion(
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
fn run_quick_fix(
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

/// The line numbers, and the rectangle of Vim's Visual block.
fn draw_line_decorations(
    ui: &egui::Ui,
    state: &CodeEditorState,
    keymap: Keymap,
    text: &str,
    (galley, galley_pos): (&egui::Galley, egui::Pos2),
    gutter: Option<std::ops::Range<f32>>,
    cursor: Option<LinePos>,
) {
    if let Some(x_range) = gutter {
        overlays::line_numbers(ui, galley, galley_pos, x_range, cursor.map(|cursor| cursor.line));
    }
    if let Some(ranges) = state.keymap.block_selection(keymap, text) {
        overlays::block_selection(ui, galley, galley_pos, &ranges);
    }
}

/// The hover popup (diagnostics and docs). `open_at`: opens it there, not at the pointer.
fn show_hover(
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
fn show_signature(
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

/// What goes behind the text: the cursor line's tint and the whitespace marks.
fn draw_text_background(
    ui: &egui::Ui,
    (galley, galley_pos): (&egui::Galley, egui::Pos2),
    cursor_line: Option<usize>,
    whitespace: bool,
) {
    if let Some(line) = cursor_line {
        overlays::current_line(ui, galley, galley_pos, line);
    }
    if whitespace {
        overlays::whitespace(ui, galley, galley_pos);
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
/// `requested`: `(position, references)` from `F12`, Vim `gd`, the menu and the like.
fn resolve_navigation<'a>(
    response: &egui::Response,
    language: &'a dyn Language,
    text: &str,
    pointer: Option<LinePos>,
    requested: Option<(LinePos, bool)>,
) -> Option<Navigation<'a>> {
    let ctrl_click = response.clicked() && response.ctx.input(|i| i.modifiers.command);
    let at_pointer = pointer.filter(|_| ctrl_click).map(|pos| (pos, false));
    let (pos, references) = requested.or(at_pointer)?;
    if references {
        language.references(text, pos.line, pos.column).map(Navigation::References)
    } else {
        language.definition(text, pos.line, pos.column).map(Navigation::Definition)
    }
}

/// The char range of a selection, start first.
const fn sorted_chars(range: &CCursorRange) -> std::ops::Range<usize> {
    let (a, b) = (range.primary.index.0, range.secondary.index.0);
    if a <= b { a..b } else { b..a }
}

/// The right-click menu. `menu_target` remembers the symbol under the pointer when it opened.
fn context_menu(
    response: &egui::Response,
    language: &dyn Language,
    text: &str,
    pointer: Option<LinePos>,
    menu_target: &mut Option<LinePos>,
    labels: &EditorLabels<'_>,
    selection: &std::ops::Range<usize>,
) -> Option<MenuAction> {
    if response.secondary_clicked() {
        *menu_target = pointer.filter(|pos| language.definition(text, pos.line, pos.column).is_some());
    }
    let mut action = None;
    response.context_menu(|ui| {
        let definition = menu_target.and_then(|pos| Some((pos, language.definition(text, pos.line, pos.column)?)));
        let symbol = definition.as_ref().map(|(pos, definition)| (*pos, &*definition.document));
        let menu = MenuContext { symbol, has_selection: !selection.is_empty() };
        action = context_menu::contents(ui, labels, menu);
    });
    action
}

/// Carries out the editing items of the menu. Returns whether the text changed.
fn run_menu_action(
    action: Option<MenuAction>,
    ui: &egui::Ui,
    id: egui::Id,
    state: &mut CodeEditorState,
    text: &mut String,
    selection: &std::ops::Range<usize>,
    response: &egui::Response,
) -> bool {
    let Some(action) = action else { return false };
    let ctx = ui.ctx();
    let mut changed = false;
    match action {
        MenuAction::Copy | MenuAction::Cut => {
            if let Some(selected) = edit_ops::selected_text(text, selection) {
                ctx.copy_text(selected.to_owned());
            }
            if action == MenuAction::Cut {
                let at = edit_ops::replace_chars(text, selection, "");
                set_selection(ctx, id, at..at);
                changed = true;
            }
        }
        MenuAction::Paste => {
            // Reading the clipboard is asynchronous: see `take_requested_paste`.
            state.pending_paste = Some(ui.input(|i| i.time));
            ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaste);
        }
        MenuAction::SelectAll => set_selection(ctx, id, 0..text.chars().count()),
        MenuAction::CommandPalette => {
            ui::palette::open(&mut state.palette, ui, id);
            return false;
        }
        MenuAction::Navigate { .. } => return false,
    }
    response.request_focus();
    changed
}

/// The clipboard text the menu's Paste asked for, put in at the selection. Returns whether it came.
///
/// It is taken before the keymap: Vim's Normal mode reads a paste event as `Ctrl+V`.
fn take_requested_paste(ui: &egui::Ui, id: egui::Id, state: &mut CodeEditorState, text: &mut String) -> bool {
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

    /// Shows an editor for pointer events, one frame at a time.
    struct Frames {
        ctx: egui::Context,
        text: String,
        state: CodeEditorState,
        id: egui::Id,
        rect: egui::Rect,
        keymap: Keymap,
        command: Option<EditorCommand>,
        language: &'static dyn Language,
    }

    /// A language that indents with tabs.
    struct Tabbed;
    impl Language for Tabbed {
        fn indent(&self) -> Indent {
            Indent::Tab(4)
        }
    }

    /// A language whose form is easy to tell.
    struct Upper;
    impl Language for Upper {
        fn format(&self, text: &str) -> Option<String> {
            Some(text.to_uppercase())
        }
    }

    impl Frames {
        fn new(text: &str) -> Self {
            let mut frames = Self {
                ctx: egui::Context::default(),
                text: text.to_owned(),
                state: CodeEditorState::default(),
                id: egui::Id::NULL,
                rect: egui::Rect::NOTHING,
                keymap: Keymap::Normal,
                command: None,
                language: &Upper,
            };
            frames.frame(Vec::new());
            frames.ctx.memory_mut(|memory| memory.request_focus(frames.id));
            frames.frame(Vec::new());
            frames
        }

        fn frame(&mut self, events: Vec<egui::Event>) {
            let raw = egui::RawInput { events, ..Default::default() };
            let (text, state, id, rect) = (&mut self.text, &mut self.state, &mut self.id, &mut self.rect);
            let (keymap, command, language) = (self.keymap, &mut self.command, self.language);
            let mut output = self.ctx.run_ui(raw, |ui| {
                let shown = CodeEditor::new("editor", text).language(language).keymap(keymap).show(ui, state);
                *command = shown.command;
                let response = shown.response;
                *id = response.id;
                *rect = response.rect;
            });
            output.textures_delta.clear();
        }

        fn select(&self, from: usize, to: usize) {
            let mut edit = egui::TextEdit::load_state(&self.ctx, self.id).unwrap_or_default();
            edit.cursor.set_char_range(Some(CCursorRange::two(CCursor::new(from), CCursor::new(to))));
            edit.store(&self.ctx, self.id);
        }

        fn selection(&self) -> std::ops::Range<usize> {
            let edit = egui::TextEdit::load_state(&self.ctx, self.id).unwrap();
            sorted_chars(&edit.cursor.char_range().unwrap())
        }

        fn move_to(&mut self, pos: egui::Pos2) {
            self.frame(vec![egui::Event::PointerMoved(pos)]);
        }

        fn button(&mut self, button: egui::PointerButton, pos: egui::Pos2, pressed: bool) {
            let modifiers = egui::Modifiers::NONE;
            self.frame(vec![egui::Event::PointerButton { pos, button, pressed, modifiers }]);
        }

        /// A point on the 0-based `line`, a little in from the start of the text.
        fn on_line(&self, line: usize) -> egui::Pos2 {
            let font = egui::TextStyle::Monospace.resolve(&self.ctx.global_style());
            let row = self.ctx.fonts_mut(|fonts| fonts.row_height(&font));
            self.rect.left_top() + egui::vec2(20.0, row.mul_add(line as f32, row / 2.0) + 2.0)
        }
    }

    #[test]
    fn the_right_button_on_the_selection_keeps_it() {
        let mut frames = Frames::new("abcdefghijkl\nsecond line");
        frames.select(0, 12);
        let on_it = frames.on_line(0);
        frames.move_to(on_it);

        frames.button(egui::PointerButton::Secondary, on_it, true);
        assert_eq!(frames.selection(), 0..12, "on the press");
        frames.frame(Vec::new());
        assert_eq!(frames.selection(), 0..12, "while it is down");
        frames.button(egui::PointerButton::Secondary, on_it, false);
        assert_eq!(frames.selection(), 0..12, "on the release");
        frames.frame(Vec::new());
        assert_eq!(frames.selection(), 0..12, "after it");
        assert_eq!(frames.state.held_selection, None);
    }

    #[test]
    fn the_right_button_elsewhere_moves_the_cursor() {
        let mut frames = Frames::new("abcdefghijkl\nsecond line");
        frames.select(0, 12);
        let elsewhere = frames.on_line(1);
        frames.move_to(elsewhere);

        frames.button(egui::PointerButton::Secondary, elsewhere, true);
        let selection = frames.selection();
        assert!(selection.is_empty() && selection.start > 12, "{selection:?}");
    }

    /// A language with the same hover over all of its text.
    struct Hovered(&'static str);
    impl Language for Hovered {
        fn hover_at(&self, _text: &str, _line: usize, _column: usize) -> Option<String> {
            Some(self.0.to_owned())
        }
    }

    /// Presses a key the way a window system reports it: the key, then the text it typed.
    fn press(frames: &mut Frames, key: egui::Key, modifiers: egui::Modifiers, typed: &str) {
        let mut events = vec![egui::Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers }];
        if !typed.is_empty() {
            events.push(egui::Event::Text(typed.to_owned()));
        }
        frames.frame(events);
    }

    fn press_shift_k(frames: &mut Frames) {
        press(frames, egui::Key::K, egui::Modifiers::SHIFT, "K");
    }

    fn press_j(frames: &mut Frames) {
        press(frames, egui::Key::J, egui::Modifiers::NONE, "j");
    }

    /// `K` twice, in Vim.
    fn in_hover_of(doc: &'static str) -> Frames {
        // (A leak is fine in a test: a language has to live as long as the frames.)
        let language: &'static Hovered = Box::leak(Box::new(Hovered(doc)));
        let mut frames = Frames::new("first\nsecond");
        frames.language = language;
        frames.keymap = Keymap::Vim;
        frames.frame(Vec::new());
        press_shift_k(&mut frames);
        press_shift_k(&mut frames);
        frames
    }

    fn open_hover(frames: &Frames) -> &crate::hover::Open {
        frames.state.hover.open.as_ref().expect("the popup is open")
    }

    #[test]
    fn k_twice_with_nothing_to_do_in_the_hover_leaves_the_keyboard_in_the_editor() {
        let mut frames = in_hover_of("plain text");
        assert!(!open_hover(&frames).focused);

        press_j(&mut frames);
        assert!(frames.state.hover.open.is_none(), "the next key closes it");
    }

    #[test]
    fn j_and_k_scroll_a_long_hover() {
        let doc: &'static str = "a line\n\n".repeat(100).leak();
        let mut frames = in_hover_of(doc);
        assert!(open_hover(&frames).focused, "there is more text to see");
        assert_eq!(open_hover(&frames).scroll_offset, 0.0);

        press_j(&mut frames);
        press_j(&mut frames);
        frames.frame(Vec::new());
        let scrolled = open_hover(&frames).scroll_offset;
        assert!(scrolled > 0.0, "{scrolled}");

        press(&mut frames, egui::Key::K, egui::Modifiers::NONE, "k");
        frames.frame(Vec::new());
        assert!(open_hover(&frames).scroll_offset < scrolled);
        assert!(frames.state.hover.open.is_some(), "not closed by them");
        assert_eq!(frames.text, "first\nsecond", "nor do they reach the editor");
    }

    #[test]
    fn tab_goes_through_the_links_of_a_hover() {
        let mut frames = in_hover_of("[one](https://example.com/1) and [two](https://example.com/2)");
        assert!(open_hover(&frames).focused);
        assert_eq!(open_hover(&frames).link, None, "on the popup, until asked");

        press_tab(&mut frames);
        assert_eq!(open_hover(&frames).link, Some(0));
        press_tab(&mut frames);
        assert_eq!(open_hover(&frames).link, Some(1));
        press(&mut frames, egui::Key::Tab, egui::Modifiers::SHIFT, "");
        assert_eq!(open_hover(&frames).link, Some(0));
        assert!(!frames.ctx.memory(|memory| memory.has_focus(frames.id)), "the keyboard is not in the editor");
    }

    #[test]
    fn q_leaves_the_hover_for_the_editor() {
        let mut frames = in_hover_of("[one](https://example.com/1)");
        press(&mut frames, egui::Key::Q, egui::Modifiers::NONE, "q");
        assert!(frames.state.hover.open.is_none());
        assert!(frames.ctx.memory(|memory| memory.has_focus(frames.id)));
        let status = frames.state.keymap.status(Keymap::Vim).unwrap();
        assert!(!status.text.contains("recording"), "the `q` is not Vim's: {}", status.text);
    }

    fn press_tab(frames: &mut Frames) {
        frames.frame(vec![egui::Event::Key {
            key: egui::Key::Tab,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }]);
    }

    fn shift_right_in_vim(frames: &mut Frames) {
        frames.keymap = Keymap::Vim;
        frames.frame(vec![egui::Event::Text(">".to_owned())]);
        frames.frame(vec![egui::Event::Text(">".to_owned())]);
    }

    #[test]
    fn indents_as_the_language_does() {
        let mut frames = Frames::new("a");
        press_tab(&mut frames);
        assert_eq!(frames.text, "a    ", "at the cursor, which is at the end");
        assert_eq!(frames.state.indent.to_string(), "Spaces: 4");

        let mut frames = Frames::new("a");
        frames.language = &Tabbed;
        frames.frame(Vec::new());
        press_tab(&mut frames);
        assert_eq!(frames.text, "a\t");
        assert_eq!(frames.state.indent.to_string(), "Tab: 4");
    }

    #[test]
    fn vim_shifts_as_the_language_does() {
        let mut frames = Frames::new("a");
        shift_right_in_vim(&mut frames);
        assert_eq!(frames.text, "    a");

        let mut frames = Frames::new("a");
        frames.language = &Tabbed;
        frames.frame(Vec::new());
        shift_right_in_vim(&mut frames);
        assert_eq!(frames.text, "\ta");
    }

    #[test]
    fn a_command_of_the_palette_is_the_command() {
        for command in EditorCommand::ALL {
            assert_eq!(EditorCommand::from_action_id(command.action("label").id), Some(command));
        }
        assert_eq!(EditorCommand::from_action_id("of_the_host"), None);
    }

    #[test]
    fn a_key_that_waits_starts_the_list_of_keys() {
        let mut frames = Frames::new("first");
        frames.keymap = Keymap::Vim;
        frames.frame(vec![egui::Event::Text(" ".to_owned())]);
        assert!(matches!(frames.state.which_key_waiting, Some((keymap::Prefix::Leader, _))));

        frames.frame(vec![egui::Event::Text("z".to_owned())]);
        assert_eq!(frames.state.which_key_waiting, None, "not a command: the keys are dropped");

        frames.frame(vec![egui::Event::Text("g".to_owned())]);
        assert!(matches!(frames.state.which_key_waiting, Some((keymap::Prefix::G, _))));
    }

    #[test]
    fn vim_space_w_formats_then_saves() {
        let mut frames = Frames::new("first\nsecond");
        frames.keymap = Keymap::Vim;
        frames.frame(vec![egui::Event::Text(" ".to_owned())]);
        assert_eq!(frames.text, "first\nsecond", "a leader key, waiting");

        frames.frame(vec![egui::Event::Text("w".to_owned())]);
        assert_eq!(frames.text, "FIRST\nSECOND");
        assert_eq!(frames.command, Some(EditorCommand::Save), "the host saves what is formatted");
    }

    #[test]
    fn vim_space_f_only_formats() {
        let mut frames = Frames::new("first");
        frames.keymap = Keymap::Vim;
        frames.frame(vec![egui::Event::Text(" ".to_owned())]);
        frames.frame(vec![egui::Event::Text("f".to_owned())]);
        assert_eq!(frames.text, "FIRST");
        assert_eq!(frames.command, None);
    }

    #[test]
    fn selecting_with_the_mouse_asks_for_no_hover() {
        let mut frames = Frames::new("abcdefghijkl");
        let start = frames.on_line(0);
        frames.move_to(start);
        frames.frame(Vec::new());
        assert!(frames.state.hover.resting.is_some(), "resting on a char waits for the hover");

        frames.button(egui::PointerButton::Primary, start, true);
        let dragged = start + egui::vec2(30.0, 0.0);
        frames.move_to(dragged);
        frames.frame(Vec::new());
        assert_eq!(frames.state.hover.resting, None, "dragging");

        frames.button(egui::PointerButton::Primary, dragged, false);
        frames.frame(Vec::new());
        assert_eq!(frames.state.hover.resting, None, "where the drag ended");

        frames.move_to(dragged + egui::vec2(10.0, 0.0));
        assert!(frames.state.hover.resting.is_some(), "moved on");
    }
}
