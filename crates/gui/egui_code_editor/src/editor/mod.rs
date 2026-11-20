//! The widget: a frame of it goes through the steps in the modules here. See the crate docs.

pub(crate) mod comment;
pub(crate) mod cursor;
mod decorations;
pub(crate) mod edit_ops;
pub(crate) mod hole;
mod input;
pub(crate) mod layout;
mod menu;
mod minimap;
pub(crate) mod overlays;
mod popups;
pub(crate) mod state;
mod text;

use egui::text::CCursor;

use self::{
    cursor::editor_id,
    decorations::{draw_line_decorations, draw_text_background, scroll_to_cursor, visible_lines},
    input::{apply_format, read_input, tab_key_follows_indent},
    menu::{context_menu, resolve_navigation, run_menu_action},
    popups::{accept_completion, run_quick_fix, show_hover, show_signature, show_which_key, update_completion_popup},
    state::CodeEditorState,
    text::{line_bottom, show_text, sorted_chars},
};
use crate::{
    Diagnostic, EditorAction, EditorLabels, EditorRequest, InlineDiff, Keymap, Language, Location, PeekAsk, PeekInput,
    ReferenceList, References, Severity,
    command::{EditorCommand, EditorFeatures},
    popups::{context_menu::MenuAction, palette},
    text_pos::LinePos,
};

/// A resolved `F12` / `Shift+F12` / Vim `gd` / `gD` / `Ctrl+click` / right-click request:
/// the host opens the place, or lists the references.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Navigation<'a> {
    Definition(Location<'a>),
    References(References<'a>),
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
    /// The id of the [`EditorAction`] whose `⚙` was clicked in the palette: the host lets its keys
    /// be changed, as VS Code's palette does.
    pub configure: Option<&'a str>,
    /// The editor's own command whose `⚙` was clicked: see [`CodeEditor::request_keys`].
    pub configure_request: Option<EditorRequest>,
    /// The `↶` of this [`InlineDiff::hunks`] entry was clicked: the host puts it back.
    pub inline_revert: Option<usize>,
    /// What the references' peek asked: see [`CodeEditor::references`].
    pub peek: Option<PeekAsk>,
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
    requests: Option<&'a [EditorRequest]>,
    request_keys: Option<&'a [(EditorRequest, String)]>,
    annotations: &'a [String],
    /// The width of the minimap; none: not shown. See [`Self::minimap`].
    minimap: Option<f32>,
    background: egui::Color32,
    references: Option<(&'a mut Option<ReferenceList>, &'a PeekInput<'a>)>,
}

impl std::fmt::Debug for CodeEditor<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CodeEditor").field("id", &self.id).field("keymap", &self.keymap).finish_non_exhaustive()
    }
}

impl<'a> CodeEditor<'a> {
    /// Default of [`Self::minimap_width`].
    pub const MINIMAP_WIDTH: f32 = 90.0;

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
            requests: None,
            request_keys: None,
            annotations: &[],
            minimap: None,
            background: egui::Color32::TRANSPARENT,
            references: None,
        }
    }

    /// The references found, as VS Code's peek: inside the editor, under the line they were asked
    /// from, pushing the lines below it down. `list` is `None` once it has closed; what it asked is
    /// [`CodeEditorOutput::peek`]. See [`ReferenceList`].
    #[inline]
    pub const fn references(mut self, list: &'a mut Option<ReferenceList>, input: &'a PeekInput<'a>) -> Self {
        self.references = Some((list, input));
        self
    }

    /// The minimap at the right of the view, as VS Code's (default: not shown): the whole text
    /// shrunk, the part in view under a slider; a click or a drag on it scrolls there.
    #[inline]
    pub const fn minimap(mut self, show: bool) -> Self {
        self.minimap = if show { Some(Self::MINIMAP_WIDTH) } else { None };
        self
    }

    /// The minimap's width, if shown (default [`Self::MINIMAP_WIDTH`]).
    #[inline]
    pub const fn minimap_width(mut self, width: f32) -> Self {
        if self.minimap.is_some() {
            self.minimap = Some(width);
        }
        self
    }

    /// The color behind the text. Default: none, so the page's own shows, as VS Code's editor is on
    /// its window's: e.g., in a window with a see-through color, a color of its own would be a second
    /// layer over it. e.g., `ui.visuals().text_edit_bg_color()` looks as egui's text fields.
    #[inline]
    pub const fn background(mut self, color: egui::Color32) -> Self {
        self.background = color;
        self
    }

    /// A note per line in a column left of the line numbers, as GitLens' blame: e.g.
    /// `#2 fix the speed`. A run of lines with the same note shows it once, on its first line, with
    /// a line above the run. Missing ones are none.
    #[inline]
    pub const fn line_annotations(mut self, annotations: &'a [String]) -> Self {
        self.annotations = annotations;
        self
    }

    /// The host's keys of the editor's commands, as its palette shows them (e.g.
    /// `(Format, "shift+alt+f")`; `""`: none), each with a `⚙` that asks the host to change them
    /// ([`CodeEditorOutput::configure_request`]). For a host that gives its [`Self::requests`].
    #[inline]
    pub const fn request_keys(mut self, keys: &'a [(EditorRequest, String)]) -> Self {
        self.request_keys = Some(keys);
        self
    }

    /// The editor's commands the host's keys asked for this frame (see [`EditorRequest`]): the
    /// editor then reads none of its own keys for them, so that the host's key bindings decide.
    #[inline]
    pub const fn requests(mut self, requests: &'a [EditorRequest]) -> Self {
        self.requests = Some(requests);
        self
    }

    /// What the host acts on, so Vim's list of keys offers them. e.g., a host without a tree has no
    /// [`EditorCommand::ToggleExplorer`].
    #[inline]
    pub const fn host_commands(mut self, host_commands: &'a [EditorCommand]) -> Self {
        self.host_commands = host_commands;
        self
    }

    /// Everything but changing the text: the cursor, selecting, copying, hover, go to definition.
    /// `changed` stays `false`. Keymaps edit the text directly, so what they did is undone at the end
    /// of the frame.
    #[inline]
    pub const fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    #[inline]
    pub const fn language(mut self, language: &'a dyn Language) -> Self {
        self.language = language;
        self
    }

    /// Problems to underline (and show on hover).
    #[inline]
    pub const fn diagnostics(mut self, diagnostics: &'a [Diagnostic]) -> Self {
        self.diagnostics = diagnostics;
        self
    }

    #[inline]
    pub const fn features(mut self, features: EditorFeatures) -> Self {
        self.features = features;
        self
    }

    /// Key binding style. Default: [`Keymap::Normal`].
    #[inline]
    pub const fn keymap(mut self, keymap: Keymap) -> Self {
        self.keymap = keymap;
        self
    }

    /// Shows the changes inside the text, like Monaco's inline diff. See [`InlineDiff`].
    #[inline]
    pub const fn inline_diff(mut self, diff: Option<&'a InlineDiff>) -> Self {
        self.inline_diff = diff;
        self
    }

    /// Translated texts of the right-click menu and the command palette.
    #[inline]
    pub const fn labels(mut self, labels: EditorLabels<'a>) -> Self {
        self.labels = labels;
        self
    }

    /// The host's commands in the palette (`Ctrl+Shift+P`). See [`CodeEditorOutput::action`].
    #[inline]
    pub const fn actions(mut self, actions: &'a [EditorAction<'a>]) -> Self {
        self.actions = actions;
        self
    }

    /// Show `·` / `→` for spaces / tabs and tint the indent levels (default `true`).
    #[inline]
    pub const fn whitespace(mut self, show: bool) -> Self {
        self.whitespace = show;
        self
    }

    /// Show the line number gutter (default `true`).
    #[inline]
    pub const fn line_numbers(mut self, show: bool) -> Self {
        self.line_numbers = show;
        self
    }

    /// Minimum height in text rows (default 30).
    #[inline]
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
            requests,
            request_keys,
            annotations,
            minimap,
            background,
            mut references,
        } = self;
        if read_only {
            // In a buffer that stays: this is every frame.
            text.clone_into(&mut state.read_only_text);
        }
        // The references' peek floats under the cursor's line, over the lines below it.
        let peek_line = references.as_mut().and_then(|(list, _)| list.as_mut()).map(|list| {
            list.anchor(|| {
                let state = egui::TextEdit::load_state(ui.ctx(), id);
                let at = state.and_then(|state| state.cursor.char_range()).map_or(0, |range| range.primary.index.0);
                LinePos::from_char_index(text, at).line
            })
        });
        let frame = EditorFrame {
            id,
            language,
            underlines: if features.diagnostics { diagnostics } else { &[] },
            features,
            keymap,
            inline_diff,
            requests,
            request_keys,
            background,
        };

        state.indent = language.indent();
        state.keymap.vim_options_mut().indent = state.indent;
        state.keymap.set_comment(language.comment());
        let input = read_input(frame, ui, state, text, &labels, actions);
        // A line found in the palette, or `:42`: the cursor goes there.
        if let Some(palette::Picked::Jump { line, column }) = input.picked {
            let at = crate::text_pos::LinePos { line, column }.to_char_index(text);
            crate::editor::cursor::set_cursor(ui.ctx(), id, at);
        }
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
        // The notes first, then the line numbers, in the one gutter.
        let notes_width = overlays::annotations_width(ui, annotations);
        let gutter = (line_numbers || notes_width > 0.0)
            .then(|| notes_width + if line_numbers { overlays::gutter_width(ui, text) } else { 0.0 });
        // What the editor draws from here on: the references' peek punches a hole in it.
        let first_shape = ui.ctx().graphics_mut(|graphics| graphics.entry(ui.layer_id()).next_idx().0);
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
            gutter.map(|width| gutter_left + notes_width..gutter_left + width),
            cursor,
        );
        overlays::annotations(ui, placed, gutter_left..gutter_left + notes_width, annotations);
        if let Some(width) = minimap {
            minimap::show(ui, (id, &mut state.minimap), placed, (ui.clip_rect().intersect(response.rect), width));
        }
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
        // The references' peek last, as VS Code's: over all the editor draws (the minimap, the hints,
        // the cursor); its popups are above it, in layers of their own.
        let peek = references.and_then(|(list, input)| {
            let top = placed.1.y + line_bottom(placed.0, peek_line?);
            // Left of the minimap, which stays in view beside it.
            let right = ui.clip_rect().right() - minimap.unwrap_or(0.0);
            ReferenceList::show_inline(ui, list, input, (top, right, first_shape))
        });
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
        let menu_state = (&mut state.menu_target, &mut state.menu_selection);
        let menu_action = context_menu(response, language, text, pointer, menu_state, &labels, &selection);
        let menu_selection = state.menu_selection.clone();
        changed |= run_menu_action(menu_action, ui, id, state, text, &menu_selection, response);
        let keyboard = navigate_keys.and_then(|references| Some((cursor?, references)));
        let from_menu = match menu_action {
            Some(MenuAction::Navigate { pos, references }) => Some((pos, references)),
            _ => None,
        };
        let navigation = resolve_navigation(response, language, text, pointer, keyboard.or(from_menu));

        if read_only {
            if *text != state.read_only_text {
                text.clone_from(&state.read_only_text);
            }
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
            configure: input.picked.and_then(palette::Picked::configure),
            configure_request: input.picked.and_then(palette::Picked::configure_request),
            inline_revert,
            peek,
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
    /// See [`CodeEditor::requests`].
    requests: Option<&'a [EditorRequest]>,
    /// See [`CodeEditor::request_keys`].
    request_keys: Option<&'a [(EditorRequest, String)]>,
    /// See [`CodeEditor::background`].
    background: egui::Color32,
}

#[cfg(test)]
mod tests;
