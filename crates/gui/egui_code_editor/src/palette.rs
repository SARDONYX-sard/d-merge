//! Command palette logic (`Ctrl+Shift+P`, like VS Code / Monaco): run any command by typing its
//! name. Drawn by [`crate::ui::palette`].
//!
//! ```text
//!          ┌───────────────────────────────────┐
//!          │ [defi                           ] │    typed chars must appear in order: `defi`, `gtd`
//!          │ > Go to Definition            F12 │    Up / Down (Ctrl+N / P) choose, Enter runs,
//!          │   Find References       Shift+F12 │    Esc closes
//!          └───────────────────────────────────┘
//! ```
//! The list has the editor's own commands ([`Builtin`]) and the host's ([`EditorAction`], like
//! Monaco's `editor.addAction`). A host action only reports its id back
//! ([`crate::CodeEditorOutput::action`]): what it does is up to the host.

use crate::{EditorFeatures, Keymap};

/// A command the host adds to the palette, e.g. `EditorAction { id: "save", label: "Save" }`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditorAction<'a> {
    /// Reported in [`crate::CodeEditorOutput::action`] when it is run.
    pub id: &'a str,
    /// Shown in the list.
    pub label: &'a str,
}

/// Texts of the editor's own menus, for translation. The default is English.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditorLabels<'a> {
    pub go_to_definition: &'a str,
    pub find_references: &'a str,
    pub find: &'a str,
    pub go_to_line: &'a str,
    pub quick_fix: &'a str,
    pub format: &'a str,
    pub show_hover: &'a str,
    pub completion: &'a str,
    pub cut: &'a str,
    pub copy: &'a str,
    pub paste: &'a str,
    pub select_all: &'a str,
    pub command_palette: &'a str,
    /// The `↶` of an inline diff, on hover.
    pub revert_change: &'a str,
    /// The palette's input field, when empty.
    pub palette_hint: &'a str,
}

impl<'a> EditorLabels<'a> {
    /// The texts in the order of the fields, for a host that keeps them in a list.
    pub const fn from_texts(texts: [&'a str; 15]) -> Self {
        let [
            go_to_definition,
            find_references,
            find,
            go_to_line,
            quick_fix,
            format,
            show_hover,
            completion,
            cut,
            copy,
            paste,
            select_all,
            command_palette,
            revert_change,
            palette_hint,
        ] = texts;
        Self {
            go_to_definition,
            find_references,
            find,
            go_to_line,
            quick_fix,
            format,
            show_hover,
            completion,
            cut,
            copy,
            paste,
            select_all,
            command_palette,
            revert_change,
            palette_hint,
        }
    }
}

impl Default for EditorLabels<'_> {
    fn default() -> Self {
        Self {
            go_to_definition: "Go to Definition",
            find_references: "Find References",
            find: "Find",
            go_to_line: "Go to Line",
            quick_fix: "Quick Fix",
            format: "Format Document",
            show_hover: "Show Hover",
            completion: "Trigger Suggest",
            cut: "Cut",
            copy: "Copy",
            paste: "Paste",
            select_all: "Select All",
            command_palette: "Command Palette",
            revert_change: "Revert this change",
            palette_hint: "Type a command",
        }
    }
}

/// The editor's own commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Builtin {
    GoToDefinition,
    FindReferences,
    Find,
    GoToLine,
    QuickFix,
    Format,
    ShowHover,
    Completion,
}

impl Builtin {
    const ALL: [Self; 8] = [
        Self::GoToDefinition,
        Self::FindReferences,
        Self::Find,
        Self::GoToLine,
        Self::QuickFix,
        Self::Format,
        Self::ShowHover,
        Self::Completion,
    ];

    const fn label<'a>(self, labels: &EditorLabels<'a>) -> &'a str {
        match self {
            Self::GoToDefinition => labels.go_to_definition,
            Self::FindReferences => labels.find_references,
            Self::Find => labels.find,
            Self::GoToLine => labels.go_to_line,
            Self::QuickFix => labels.quick_fix,
            Self::Format => labels.format,
            Self::ShowHover => labels.show_hover,
            Self::Completion => labels.completion,
        }
    }

    const fn shortcut(self) -> &'static str {
        match self {
            Self::GoToDefinition => "F12",
            Self::FindReferences => "Shift+F12",
            Self::Find => "Ctrl+F",
            Self::GoToLine => "Ctrl+G",
            Self::QuickFix => "Ctrl+.",
            Self::Format => "Shift+Alt+F",
            Self::ShowHover => "",
            Self::Completion => "Ctrl+Space",
        }
    }

    /// Whether the command makes sense now. e.g., Vim has its own `/`, so no find bar there.
    const fn available(self, context: PaletteContext) -> bool {
        match self {
            Self::Find | Self::GoToLine => matches!(context.keymap, Keymap::Normal),
            Self::Format => context.features.format,
            Self::ShowHover => context.features.hover,
            Self::Completion => context.features.completion && context.inserting,
            Self::GoToDefinition | Self::FindReferences | Self::QuickFix => true,
        }
    }
}

/// What was run from the palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Picked<'a> {
    Builtin(Builtin),
    Host(&'a str),
}

impl<'a> Picked<'a> {
    /// The id of a host action.
    pub(crate) const fn host_action(self) -> Option<&'a str> {
        match self {
            Self::Host(action) => Some(action),
            Self::Builtin(_) => None,
        }
    }
}

/// The editor's situation, which decides the available commands.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PaletteContext {
    pub(crate) keymap: Keymap,
    pub(crate) features: EditorFeatures,
    /// In Vim's Insert mode, or a keymap without modes.
    pub(crate) inserting: bool,
}

/// One line of the list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Command<'a> {
    pub(crate) picked: Picked<'a>,
    pub(crate) label: &'a str,
    /// e.g., `Shift+F12`; `""` if it has none.
    pub(crate) shortcut: &'static str,
}

/// The commands available now that match `query`: the host's first.
pub(crate) fn commands<'a>(
    query: &str,
    context: PaletteContext,
    labels: &EditorLabels<'a>,
    actions: &'a [EditorAction<'a>],
) -> Vec<Command<'a>> {
    let host =
        actions.iter().map(|action| Command { picked: Picked::Host(action.id), label: action.label, shortcut: "" });
    let builtin = Builtin::ALL.into_iter().filter(|command| command.available(context)).map(|command| Command {
        picked: Picked::Builtin(command),
        label: command.label(labels),
        shortcut: command.shortcut(),
    });
    host.chain(builtin).filter(|command| matches_query(query, command.label)).collect()
}

/// Do the chars of `query` appear in `label`, in order? (ASCII case ignored.)
///
/// e.g., `gtd` and `defi` match `Go to Definition`; `dg` does not.
fn matches_query(query: &str, label: &str) -> bool {
    let mut label = label.chars();
    query.chars().filter(|c| !c.is_whitespace()).all(|q| label.any(|l| l.eq_ignore_ascii_case(&q)))
}

/// The palette's keys of one frame. Plain data: read in [`crate::ui::palette`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct PaletteKeys {
    pub(crate) up: bool,
    pub(crate) down: bool,
    pub(crate) enter: bool,
    pub(crate) escape: bool,
}

/// Palette state (part of [`crate::CodeEditorState`]).
#[derive(Debug, Default)]
pub(crate) struct Palette {
    pub(crate) open: bool,
    pub(crate) query: String,
    pub(crate) selected: usize,
    /// The palette's rect last frame: a click outside it closes it.
    pub(crate) rect: Option<egui::Rect>,
}

impl Palette {
    pub(crate) fn reopen(&mut self) {
        *self = Self { open: true, ..Self::default() };
    }

    /// Moves the selection (wrapping around); `Enter` returns the selected command.
    pub(crate) fn on_keys<'a>(&mut self, keys: PaletteKeys, commands: &[Command<'a>]) -> Option<Picked<'a>> {
        let last = commands.len().saturating_sub(1);
        self.selected = self.selected.min(last);
        if keys.up {
            self.selected = self.selected.checked_sub(1).unwrap_or(last);
        }
        if keys.down {
            self.selected = if self.selected >= last { 0 } else { self.selected + 1 };
        }
        if keys.enter { commands.get(self.selected).map(|command| command.picked) } else { None }
    }
}

/// One frame's requests from the keys or the palette, for what takes them the same way.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Triggers {
    pub(crate) quick_fix: bool,
    /// `Some(true)` = find references, `Some(false)` = go to definition.
    pub(crate) navigate: Option<bool>,
    pub(crate) format: bool,
    pub(crate) completion: bool,
    pub(crate) hover: bool,
    pub(crate) find: bool,
    pub(crate) go_to_line: bool,
}

impl Triggers {
    /// Adds what was picked in the palette to what the keys asked for.
    pub(crate) const fn with_picked(mut self, picked: Option<Picked<'_>>) -> Self {
        let Some(Picked::Builtin(command)) = picked else { return self };
        match command {
            Builtin::GoToDefinition => self.navigate = Some(false),
            Builtin::FindReferences => self.navigate = Some(true),
            Builtin::Find => self.find = true,
            Builtin::GoToLine => self.go_to_line = true,
            Builtin::QuickFix => self.quick_fix = true,
            Builtin::Format => self.format = true,
            Builtin::ShowHover => self.hover = true,
            Builtin::Completion => self.completion = true,
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const COMMANDS: [Command<'_>; 2] = [
        Command { picked: Picked::Host("a"), label: "A", shortcut: "" },
        Command { picked: Picked::Host("b"), label: "B", shortcut: "" },
    ];

    #[test]
    fn keys_move_the_selection_and_enter_picks() {
        let mut palette = Palette::default();
        let down = PaletteKeys { down: true, ..PaletteKeys::default() };
        let up = PaletteKeys { up: true, ..PaletteKeys::default() };
        let enter = PaletteKeys { enter: true, ..PaletteKeys::default() };

        assert_eq!(palette.on_keys(down, &COMMANDS), None);
        assert_eq!(palette.selected, 1);
        assert_eq!(palette.on_keys(enter, &COMMANDS), Some(Picked::Host("b")));
        palette.on_keys(down, &COMMANDS); // wraps around
        assert_eq!(palette.selected, 0);
        palette.on_keys(up, &COMMANDS);
        assert_eq!(palette.selected, 1);
        assert_eq!(palette.on_keys(enter, &[]), None); // nothing matches the query
    }

    #[test]
    fn lists_only_what_applies() {
        let labels = EditorLabels::default();
        let actions = [EditorAction { id: "save", label: "Save" }];
        let vim_normal = PaletteContext { keymap: Keymap::Vim, features: EditorFeatures::default(), inserting: false };

        let listed = commands("", vim_normal, &labels, &actions);
        assert_eq!(listed[0].picked, Picked::Host("save"));
        // Vim has its own search, and suggestions need Insert mode.
        assert!(!listed.iter().any(|c| c.picked == Picked::Builtin(Builtin::Find)));
        assert!(!listed.iter().any(|c| c.picked == Picked::Builtin(Builtin::Completion)));
        assert_eq!(commands("save", vim_normal, &labels, &actions).len(), 1);
    }

    #[test]
    fn picked_commands_become_triggers() {
        let picked = Some(Picked::Builtin(Builtin::FindReferences));
        assert_eq!(Triggers::default().with_picked(picked).navigate, Some(true));
        assert_eq!(Triggers::default().with_picked(Some(Picked::Host("x"))), Triggers::default());
    }

    #[test]
    fn query_chars_must_appear_in_order() {
        assert!(matches_query("", "Go to Definition"));
        assert!(matches_query("gtd", "Go to Definition"));
        assert!(matches_query("DEFI", "Go to Definition"));
        assert!(matches_query("go to", "Go to Definition"));
        assert!(!matches_query("dg", "Go to Definition"));
    }
}
