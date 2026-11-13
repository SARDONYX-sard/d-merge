//! What a page with a code editor does with the key bindings each frame, in one place: the commands
//! of the keys, the keys its command palette shows, and VS Code's `⚙` that records new keys.
//!
//! ```text
//! page frame:  let keys = key_ui.frame(ctx, bindings, &context, texts);   // before the widgets
//!              palette: EditorAction::new(id, label).with_keys(key_ui.palette_keys(id)?)
//!              output.configure == Some(id) -> key_ui.configure(id)        // the recorder opens
//! ```

use crate::{
    KeyContext, Keybindings,
    commands::{
        EDITOR_SAVE, FILE_REVEAL, HISTORY_BACK, HISTORY_FORWARD, KEYBINDINGS_OPEN_JSON, TABS_CLOSE, TABS_NEXT,
        TABS_PREVIOUS,
    },
    json_editor::JsonEditor,
    recorder::{KeyRecorder, Recorded, RecorderTexts},
};

/// The palette action of VS Code's "Preferences: Open Keyboard Shortcuts (JSON)".
pub const OPEN_JSON_ACTION: &str = "open_keybindings_json";

/// The editor's palette actions that are commands here: `(action id, command)`. The action ids are
/// those of `egui_code_editor::EditorCommand` and the hosts' own (`go_back`).
pub const EDITOR_ACTIONS: [(&str, &str); 8] = [
    ("next_tab", TABS_NEXT),
    ("previous_tab", TABS_PREVIOUS),
    ("close_tab", TABS_CLOSE),
    ("reveal_file", FILE_REVEAL),
    ("save", EDITOR_SAVE),
    ("go_back", HISTORY_BACK),
    ("go_forward", HISTORY_FORWARD),
    (OPEN_JSON_ACTION, KEYBINDINGS_OPEN_JSON),
];

/// Takes the commands `map` knows out of `commands`, in the order they came, as what `map` makes
/// them. e.g. `[explorer.down, tabs.next, explorer.down]` -> `[Next, Next]`, `[tabs.next]` left
pub fn take_mapped<T>(commands: &mut Vec<&'static str>, map: impl Fn(&str) -> Option<T>) -> Vec<T> {
    let mut taken = Vec::new();
    commands.retain(|command| match map(command) {
        Some(value) => {
            taken.push(value);
            false
        }
        None => true,
    });
    taken
}

/// The code editor's request of a command (see `egui_code_editor::EditorRequest`): the editor's
/// own keys are the key bindings' then. e.g. `editor.toggleComment` -> `ToggleComment`
#[must_use]
pub fn editor_request(command: &str) -> Option<egui_code_editor::EditorRequest> {
    use egui_code_editor::EditorRequest as Request;

    use crate::commands::{
        EDITOR_COMMAND_PALETTE, EDITOR_COPY_LINES_DOWN, EDITOR_COPY_LINES_UP, EDITOR_DELETE_LINES, EDITOR_FIND,
        EDITOR_FIND_REFERENCES, EDITOR_FORMAT, EDITOR_GO_TO_DEFINITION, EDITOR_GO_TO_LINE, EDITOR_INSERT_LINE_ABOVE,
        EDITOR_INSERT_LINE_BELOW, EDITOR_MOVE_LINES_DOWN, EDITOR_MOVE_LINES_UP, EDITOR_QUICK_FIX,
        EDITOR_TOGGLE_COMMENT, EDITOR_TRIGGER_SUGGEST,
    };
    Some(match command {
        EDITOR_TRIGGER_SUGGEST => Request::Completion,
        EDITOR_FORMAT => Request::Format,
        EDITOR_QUICK_FIX => Request::QuickFix,
        EDITOR_COMMAND_PALETTE => Request::CommandPalette,
        EDITOR_GO_TO_DEFINITION => Request::GoToDefinition,
        EDITOR_FIND_REFERENCES => Request::FindReferences,
        EDITOR_FIND => Request::Find,
        EDITOR_GO_TO_LINE => Request::GoToLine,
        EDITOR_DELETE_LINES => Request::DeleteLines,
        EDITOR_MOVE_LINES_UP => Request::MoveLinesUp,
        EDITOR_MOVE_LINES_DOWN => Request::MoveLinesDown,
        EDITOR_COPY_LINES_UP => Request::CopyLinesUp,
        EDITOR_COPY_LINES_DOWN => Request::CopyLinesDown,
        EDITOR_INSERT_LINE_BELOW => Request::InsertLineBelow,
        EDITOR_INSERT_LINE_ABOVE => Request::InsertLineAbove,
        EDITOR_TOGGLE_COMMENT => Request::ToggleComment,
        _ => return None,
    })
}

/// The Problems panel's key of a command. e.g. `problems.down` -> `Next`
#[must_use]
pub fn problems_key(command: &str) -> Option<egui_code_editor::ProblemsKey> {
    use egui_code_editor::ProblemsKey;

    use crate::commands::{PROBLEMS_DOWN, PROBLEMS_FIRST, PROBLEMS_GO, PROBLEMS_LAST, PROBLEMS_LEAVE, PROBLEMS_UP};
    Some(match command {
        PROBLEMS_DOWN => ProblemsKey::Next,
        PROBLEMS_UP => ProblemsKey::Previous,
        PROBLEMS_FIRST => ProblemsKey::First,
        PROBLEMS_LAST => ProblemsKey::Last,
        PROBLEMS_GO => ProblemsKey::Go,
        PROBLEMS_LEAVE => ProblemsKey::Leave,
        _ => return None,
    })
}

/// The command of a palette action, if it is one. e.g. `close_tab` -> `tabs.close`
#[must_use]
pub fn command_of_action(action: &str) -> Option<&'static str> {
    EDITOR_ACTIONS.iter().find(|(id, _)| *id == action).map(|(_, command)| *command)
}

/// One frame's outcome. See [`KeyUi::frame`].
#[derive(Debug, Default)]
pub struct FrameKeys {
    /// The commands the keys asked for.
    pub commands: Vec<&'static str>,
    /// The recorder saved new keys (`Ok`) or could not (`Err`).
    pub saved: Option<Result<(), String>>,
}

/// Where the JSON opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum JsonAt {
    Top,
    /// At the entry of this command.
    Command(&'static str),
}

/// Keep one per page.
#[derive(Debug, Default)]
pub struct KeyUi {
    recorder: KeyRecorder,
    /// The keys of [`EDITOR_ACTIONS`] as the palette shows them, for `cached_for`.
    palette: Vec<(&'static str, String)>,
    /// The bindings and the context the palette's keys are of: computed again only when they change.
    cached_for: Option<(u64, KeyContext)>,
    /// The context the recorder was opened in: the binding it replaces is the one working there.
    record_context: KeyContext,
    json: JsonEditor,
    /// The JSON is to be opened next frame.
    json_asked: Option<JsonAt>,
}

impl KeyUi {
    /// Call before the widgets. While the recorder is open it has all the keys (no command then) and
    /// draws its window; else the keys of a binding become its command.
    pub fn frame(
        &mut self,
        ctx: &egui::Context,
        bindings: &mut Keybindings,
        context: &KeyContext,
        texts: RecorderTexts<'_>,
    ) -> FrameKeys {
        let mut out = FrameKeys::default();
        if let Some(at) = self.json_asked.take()
            && let Some(path) = bindings.path()
        {
            let command = match at {
                JsonAt::Top => None,
                JsonAt::Command(command) => Some(command),
            };
            self.json.open(path, command);
        }
        if self.recorder.is_open() {
            if let Some(Recorded::Keys(command, keys)) = self.recorder.take_keys(ctx) {
                out.saved = Some(bindings.bind(command, &keys, &self.record_context));
            }
            self.json_asked = self.recorder.show(ctx, bindings, texts).map(JsonAt::Command);
        } else if !self.json.has_keyboard(ctx) {
            // Typed in the JSON's editor, the keys are its own.
            out.commands = bindings.dispatch(ctx, context);
            if let Some(at) = out.commands.iter().position(|command| *command == KEYBINDINGS_OPEN_JSON) {
                out.commands.swap_remove(at);
                self.json_asked = Some(JsonAt::Top);
            }
        }
        let keymap = if context.vim { egui_code_editor::Keymap::Vim } else { egui_code_editor::Keymap::Normal };
        if let Some(saved) = self.json.show(ctx, bindings, keymap, &texts) {
            out.saved = Some(saved);
        }
        // The palette is of the editor: its keys as the editor with the keyboard sees them.
        let editor = KeyContext { editor_focus: true, input_focus: false, explorer_focus: false, ..*context };
        if self.cached_for != Some((bindings.generation(), editor)) {
            self.cached_for = Some((bindings.generation(), editor));
            self.palette = EDITOR_ACTIONS
                .iter()
                .map(|(action, command)| (*action, bindings.keys_of(command, &editor).unwrap_or_default()))
                .collect();
            self.record_context = editor;
        }
        out
    }

    /// `action` as the palette lists it: its keys and `⚙` if it is a command here, and found by
    /// its English name (`english`, when the label is translated) and its command id too.
    ///
    /// e.g. `close_tab` "タブを閉じる" -> keys `ctrl+w`, found by `Close the tab` and `tabs.close`
    #[must_use]
    pub fn palette_action<'a>(
        &'a self,
        action: egui_code_editor::EditorAction<'a>,
        english: &'a str,
    ) -> egui_code_editor::EditorAction<'a> {
        let command = command_of_action(action.id).unwrap_or_default();
        let action = action.with_aliases([english, command]);
        match self.palette_keys(action.id) {
            Some(keys) => action.with_keys(keys),
            None => action,
        }
    }

    /// The keys a palette action shows (`""`: none yet). `None`: it is no command, so it has no `⚙`.
    #[must_use]
    pub fn palette_keys(&self, action: &str) -> Option<&str> {
        self.palette.iter().find(|(id, _)| *id == action).map(|(_, keys)| keys.as_str())
    }

    /// A palette action that is the key bindings' own: `true` if it was, and is done.
    ///
    /// e.g. `open_keybindings_json` opens the JSON.
    pub fn run_action(&mut self, action: &str) -> bool {
        let own = action == OPEN_JSON_ACTION;
        if own {
            self.json_asked = Some(JsonAt::Top);
        }
        own
    }

    /// The `⚙` of palette action `action`: the recorder opens for its command.
    pub fn configure(&mut self, action: &str) {
        if let Some(command) = command_of_action(action) {
            self.configure_command(command);
        }
    }

    /// The recorder opens for `command` (an id of [`crate::COMMANDS`]).
    pub fn configure_command(&mut self, command: &str) {
        self.recorder.start(command);
    }
}
