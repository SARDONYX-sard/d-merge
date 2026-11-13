//! The commands keys can be bound to, and their bindings out of the box (the exe's defaults, as
//! VS Code's: the user file only changes them).
//!
//! An id is `area.verbNoun`, e.g. `tabs.close`. Whoever carries out a command matches its id: see
//! [`crate::Keybindings::dispatch`].

/// One command. e.g. `tabs.close`: `Close the tab`, `ctrl+w` while the editor of the Normal keymap
/// has the keyboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Command {
    pub id: &'static str,
    /// What it does, for the schema and the settings. e.g. `Close the tab`
    pub description: &'static str,
    /// `(keys, when)` it has out of the box.
    pub defaults: &'static [(&'static str, Option<&'static str>)],
}

pub const TABS_NEXT: &str = "tabs.next";
pub const TABS_PREVIOUS: &str = "tabs.previous";
pub const TABS_CLOSE: &str = "tabs.close";
pub const HISTORY_BACK: &str = "history.back";
pub const HISTORY_FORWARD: &str = "history.forward";
pub const SIDE_BAR_EXPLORER: &str = "sideBar.explorer";
pub const SIDE_BAR_SEARCH: &str = "sideBar.search";
pub const SIDE_BAR_CHANGES: &str = "sideBar.changes";
pub const PROBLEMS_TOGGLE: &str = "problems.toggle";
pub const EXPLORER_TOGGLE_VIEW: &str = "explorer.toggleView";
pub const EXPLORER_TOGGLE_CHANGED_ONLY: &str = "explorer.toggleChangedOnly";
pub const EXPLORER_TOGGLE_UNLINKED_ONLY: &str = "explorer.toggleUnlinkedOnly";
pub const EDITOR_SAVE: &str = "editor.save";
pub const EDITOR_FORMAT_AND_SAVE: &str = "editor.formatAndSave";
pub const EDITOR_TOGGLE_PREVIEW: &str = "editor.togglePreview";
pub const EDITOR_TOGGLE_INLINE_DIFF: &str = "editor.toggleInlineDiff";
pub const EXPLORER_FOCUS: &str = "explorer.focus";
pub const EDITOR_NEXT_CHANGE: &str = "editor.nextChange";
pub const EDITOR_PREVIOUS_CHANGE: &str = "editor.previousChange";
pub const FILE_COPY_PATH: &str = "file.copyPath";
pub const FILE_REVEAL: &str = "file.reveal";
pub const KEYBINDINGS_OPEN_JSON: &str = "keybindings.openJson";
pub const EDITOR_TRIGGER_SUGGEST: &str = "editor.triggerSuggest";
pub const EDITOR_FORMAT: &str = "editor.format";
pub const EDITOR_QUICK_FIX: &str = "editor.quickFix";
pub const EDITOR_COMMAND_PALETTE: &str = "editor.commandPalette";
pub const EDITOR_GO_TO_DEFINITION: &str = "editor.goToDefinition";
pub const EDITOR_FIND_REFERENCES: &str = "editor.findReferences";
pub const EDITOR_FIND: &str = "editor.find";
pub const EDITOR_GO_TO_LINE: &str = "editor.goToLine";
pub const EDITOR_DELETE_LINES: &str = "editor.deleteLines";
pub const EDITOR_MOVE_LINES_UP: &str = "editor.moveLinesUp";
pub const EDITOR_MOVE_LINES_DOWN: &str = "editor.moveLinesDown";
pub const EDITOR_COPY_LINES_UP: &str = "editor.copyLinesUp";
pub const EDITOR_COPY_LINES_DOWN: &str = "editor.copyLinesDown";
pub const EDITOR_INSERT_LINE_BELOW: &str = "editor.insertLineBelow";
pub const EDITOR_INSERT_LINE_ABOVE: &str = "editor.insertLineAbove";
pub const EDITOR_TOGGLE_COMMENT: &str = "editor.toggleComment";
pub const EXPLORER_DOWN: &str = "explorer.down";
pub const EXPLORER_UP: &str = "explorer.up";
pub const EXPLORER_FIRST: &str = "explorer.first";
pub const EXPLORER_LAST: &str = "explorer.last";
pub const EXPLORER_COLLAPSE: &str = "explorer.collapse";
pub const EXPLORER_EXPAND: &str = "explorer.expand";
pub const EXPLORER_TOGGLE: &str = "explorer.toggle";
pub const EXPLORER_LEAVE: &str = "explorer.leave";
pub const PROBLEMS_DOWN: &str = "problems.down";
pub const PROBLEMS_UP: &str = "problems.up";
pub const PROBLEMS_FIRST: &str = "problems.first";
pub const PROBLEMS_LAST: &str = "problems.last";
pub const PROBLEMS_GO: &str = "problems.go";
pub const PROBLEMS_LEAVE: &str = "problems.leave";

/// The editor of the Normal keymap has the keyboard. (Vim has `Ctrl+W` / `Ctrl+S` of its own.)
const NORMAL_EDITOR: Option<&str> = Some("editorFocus && !vim");
/// The explorer has the keyboard, not a text field.
const EXPLORER: Option<&str> = Some("explorerFocus && !inputFocus");
/// The code editor has the keyboard.
const EDITOR: Option<&str> = Some("editorFocus");
/// Vim's leader keys (`Space ...`): in Normal mode, with no command being typed.
const VIM_NORMAL: Option<&str> = Some("editorFocus && vim.mode == 'normal'");
/// The same, in the patch editor: what only it has (the tree, the inline diff).
const VIM_NORMAL_PATCH: Option<&str> = Some("editorFocus && vim.mode == 'normal' && page == 'patchEditor'");
/// The Problems panel has the keyboard.
const PROBLEMS: Option<&str> = Some("problemsFocus");

/// Every command, in the order of the settings.
pub const COMMANDS: &[Command] = &[
    Command {
        id: TABS_NEXT,
        description: "Next tab",
        defaults: &[("ctrl+pagedown", None), ("alt+l", None), ("g t", VIM_NORMAL)],
    },
    Command {
        id: TABS_PREVIOUS,
        description: "Previous tab",
        defaults: &[("ctrl+pageup", None), ("alt+h", None), ("g shift+t", VIM_NORMAL)],
    },
    Command {
        id: EDITOR_NEXT_CHANGE,
        description: "The next change (of the inline diff, or the preview)",
        defaults: &[("] c", VIM_NORMAL_PATCH)],
    },
    Command {
        id: EDITOR_PREVIOUS_CHANGE,
        description: "The previous change (of the inline diff, or the preview)",
        defaults: &[("[ c", VIM_NORMAL_PATCH)],
    },
    Command {
        id: TABS_CLOSE,
        description: "Close the tab",
        defaults: &[("ctrl+w", NORMAL_EDITOR), ("space c", VIM_NORMAL)],
    },
    Command {
        id: EXPLORER_FOCUS,
        description: "Move the keyboard to the object tree / list",
        defaults: &[("space e", VIM_NORMAL_PATCH)],
    },
    Command {
        id: EDITOR_TOGGLE_PREVIEW,
        description: "Show or hide the preview beside the editor",
        defaults: &[("space p", VIM_NORMAL)],
    },
    Command {
        id: EDITOR_TOGGLE_INLINE_DIFF,
        description: "Show or hide the changes inside the editor",
        defaults: &[("space d", VIM_NORMAL_PATCH)],
    },
    Command {
        id: EDITOR_FORMAT_AND_SAVE,
        description: "Format the document and save",
        defaults: &[("space w", VIM_NORMAL)],
    },
    Command {
        id: HISTORY_BACK,
        description: "Back to the object shown before",
        defaults: &[("alt+left", Some("!inputFocus"))],
    },
    Command { id: HISTORY_FORWARD, description: "Forward again", defaults: &[("alt+right", Some("!inputFocus"))] },
    Command { id: SIDE_BAR_EXPLORER, description: "Show the explorer", defaults: &[("ctrl+shift+e", None)] },
    Command { id: SIDE_BAR_SEARCH, description: "Show the search", defaults: &[("ctrl+shift+f", None)] },
    Command { id: SIDE_BAR_CHANGES, description: "Show the changes", defaults: &[("ctrl+shift+g", None)] },
    Command {
        id: PROBLEMS_TOGGLE,
        description: "Show the Problems panel with the keyboard, or hide it",
        defaults: &[("ctrl+shift+l", None)],
    },
    Command { id: EXPLORER_TOGGLE_VIEW, description: "Explorer: tree or flat list", defaults: &[("t", EXPLORER)] },
    Command {
        id: EXPLORER_TOGGLE_CHANGED_ONLY,
        description: "Explorer: changed objects only",
        defaults: &[("c", EXPLORER)],
    },
    Command {
        id: EXPLORER_TOGGLE_UNLINKED_ONLY,
        description: "Explorer: new objects nothing links to only",
        defaults: &[("u", EXPLORER)],
    },
    Command { id: EXPLORER_DOWN, description: "Explorer: next row", defaults: &[("j", EXPLORER), ("down", EXPLORER)] },
    Command { id: EXPLORER_UP, description: "Explorer: previous row", defaults: &[("k", EXPLORER), ("up", EXPLORER)] },
    Command { id: EXPLORER_FIRST, description: "Explorer: first row", defaults: &[("g", EXPLORER)] },
    Command { id: EXPLORER_LAST, description: "Explorer: last row", defaults: &[("shift+g", EXPLORER)] },
    Command {
        id: EXPLORER_COLLAPSE,
        description: "Explorer: close the node, or go to its parent",
        defaults: &[("h", EXPLORER), ("left", EXPLORER)],
    },
    Command {
        id: EXPLORER_EXPAND,
        description: "Explorer: open the node, or go into it",
        defaults: &[("l", EXPLORER), ("right", EXPLORER)],
    },
    Command { id: EXPLORER_TOGGLE, description: "Explorer: open or close the node", defaults: &[("o", EXPLORER)] },
    Command {
        id: EXPLORER_LEAVE,
        description: "Explorer: back to the editor",
        defaults: &[("enter", EXPLORER), ("escape", EXPLORER), ("space e", EXPLORER)],
    },
    Command {
        id: PROBLEMS_DOWN,
        description: "Problems: next problem",
        defaults: &[("j", PROBLEMS), ("down", PROBLEMS)],
    },
    Command {
        id: PROBLEMS_UP,
        description: "Problems: previous problem",
        defaults: &[("k", PROBLEMS), ("up", PROBLEMS)],
    },
    Command {
        id: PROBLEMS_FIRST,
        description: "Problems: first problem",
        defaults: &[("g", PROBLEMS), ("home", PROBLEMS)],
    },
    Command {
        id: PROBLEMS_LAST,
        description: "Problems: last problem",
        defaults: &[("shift+g", PROBLEMS), ("end", PROBLEMS)],
    },
    Command {
        id: PROBLEMS_GO,
        description: "Problems: go to the problem (the editor gets the keyboard)",
        defaults: &[("enter", PROBLEMS)],
    },
    Command { id: PROBLEMS_LEAVE, description: "Problems: back to the editor", defaults: &[("escape", PROBLEMS)] },
    Command { id: EDITOR_SAVE, description: "Save", defaults: &[("ctrl+s", None)] },
    Command {
        id: EDITOR_TRIGGER_SUGGEST,
        description: "Open the completion list",
        defaults: &[("ctrl+space", EDITOR)],
    },
    Command {
        id: EDITOR_FORMAT,
        description: "Format the document",
        defaults: &[("shift+alt+f", EDITOR), ("space f", VIM_NORMAL)],
    },
    Command { id: EDITOR_QUICK_FIX, description: "The quick fixes at the cursor", defaults: &[("ctrl+.", EDITOR)] },
    Command {
        id: EDITOR_COMMAND_PALETTE,
        description: "The editor's command palette",
        defaults: &[("ctrl+shift+p", EDITOR)],
    },
    Command {
        id: EDITOR_GO_TO_DEFINITION,
        description: "Go to the definition",
        defaults: &[("f12", EDITOR), ("g d", VIM_NORMAL)],
    },
    Command {
        id: EDITOR_FIND_REFERENCES,
        description: "Find the references",
        defaults: &[("shift+f12", EDITOR), ("g shift+d", VIM_NORMAL)],
    },
    Command {
        id: EDITOR_FIND,
        description: "The find bar (Normal keymap; Vim has /)",
        defaults: &[("ctrl+f", NORMAL_EDITOR)],
    },
    Command {
        id: EDITOR_GO_TO_LINE,
        description: "Go to a line (Normal keymap; Vim has :42)",
        defaults: &[("ctrl+g", NORMAL_EDITOR)],
    },
    Command { id: EDITOR_DELETE_LINES, description: "Delete the lines", defaults: &[("ctrl+shift+k", NORMAL_EDITOR)] },
    Command { id: EDITOR_MOVE_LINES_UP, description: "Move the lines up", defaults: &[("alt+up", NORMAL_EDITOR)] },
    Command {
        id: EDITOR_MOVE_LINES_DOWN,
        description: "Move the lines down",
        defaults: &[("alt+down", NORMAL_EDITOR)],
    },
    Command {
        id: EDITOR_COPY_LINES_UP,
        description: "Copy the lines up",
        defaults: &[("shift+alt+up", NORMAL_EDITOR)],
    },
    Command {
        id: EDITOR_COPY_LINES_DOWN,
        description: "Copy the lines down",
        defaults: &[("shift+alt+down", NORMAL_EDITOR)],
    },
    Command {
        id: EDITOR_INSERT_LINE_BELOW,
        description: "Insert a line below",
        defaults: &[("ctrl+enter", NORMAL_EDITOR)],
    },
    Command {
        id: EDITOR_INSERT_LINE_ABOVE,
        description: "Insert a line above",
        defaults: &[("ctrl+shift+enter", NORMAL_EDITOR)],
    },
    Command {
        id: EDITOR_TOGGLE_COMMENT,
        description: "Comment the lines out, or back in",
        defaults: &[("ctrl+/", EDITOR)],
    },
    Command {
        id: FILE_COPY_PATH,
        description: "Copy the path of the selected object's file (VS Code's Copy Path)",
        defaults: &[("shift+alt+c", Some("!inputFocus"))],
    },
    Command {
        id: KEYBINDINGS_OPEN_JSON,
        description: "Open keybindings.json in the editor (VS Code's Preferences: Open Keyboard Shortcuts (JSON))",
        defaults: &[],
    },
    Command {
        id: FILE_REVEAL,
        description: "Show the selected object's file in the file manager (VS Code's Reveal in File Explorer)",
        defaults: &[("shift+alt+r", Some("!inputFocus")), ("space shift+v", VIM_NORMAL)],
    },
];

/// The command of `id`, if there is one.
#[must_use]
pub fn command(id: &str) -> Option<&'static Command> {
    COMMANDS.iter().find(|command| command.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each id once, and every default readable: a typo here would only show at run time.
    #[test]
    fn the_commands_are_well_formed() {
        for (at, command) in COMMANDS.iter().enumerate() {
            assert!(!COMMANDS[..at].iter().any(|other| other.id == command.id), "{} twice", command.id);
            for (keys, when) in command.defaults {
                crate::key::parse_keys(keys).unwrap();
                if let Some(when) = when {
                    crate::when::When::parse(when).unwrap();
                }
            }
        }
    }
}
