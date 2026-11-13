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
pub const FILE_COPY_PATH: &str = "file.copyPath";
pub const FILE_REVEAL: &str = "file.reveal";
pub const KEYBINDINGS_OPEN_JSON: &str = "keybindings.openJson";

/// The editor of the Normal keymap has the keyboard. (Vim has `Ctrl+W` / `Ctrl+S` of its own.)
const NORMAL_EDITOR: Option<&str> = Some("editorFocus && !vim");
/// The explorer has the keyboard, not a text field.
const EXPLORER: Option<&str> = Some("explorerFocus && !inputFocus");

/// Every command, in the order of the settings.
pub const COMMANDS: &[Command] = &[
    Command { id: TABS_NEXT, description: "Next tab", defaults: &[("ctrl+pagedown", None), ("alt+l", None)] },
    Command { id: TABS_PREVIOUS, description: "Previous tab", defaults: &[("ctrl+pageup", None), ("alt+h", None)] },
    Command { id: TABS_CLOSE, description: "Close the tab", defaults: &[("ctrl+w", NORMAL_EDITOR)] },
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
    Command { id: EDITOR_SAVE, description: "Save", defaults: &[("ctrl+s", None)] },
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
        defaults: &[("shift+alt+r", Some("!inputFocus"))],
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
