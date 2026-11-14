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

/// A command the host adds to the palette, e.g. `EditorAction::new("save", "Save")`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditorAction<'a> {
    /// Reported in [`crate::CodeEditorOutput::action`] when it is run.
    pub id: &'a str,
    /// Shown in the list.
    pub label: &'a str,
    /// Its keys as the host's key bindings have them, e.g. `ctrl+w`; `""` for none. `Some`: they can
    /// be changed, as in VS Code's palette: its `⚙` comes back as [`crate::CodeEditorOutput::configure`].
    pub keys: Option<&'a str>,
    /// More it is found by in the palette, besides its label and id: e.g. its English name when the
    /// label is translated, and the host's command id. `""`: none.
    pub aliases: [&'a str; 2],
}

impl<'a> EditorAction<'a> {
    #[inline]
    pub const fn new(id: &'a str, label: &'a str) -> Self {
        Self { id, label, keys: None, aliases: ["", ""] }
    }

    /// More it is found by. See [`Self::aliases`]. e.g. `["Close the tab", "tabs.close"]`
    #[inline]
    #[must_use]
    pub const fn with_aliases(mut self, aliases: [&'a str; 2]) -> Self {
        self.aliases = aliases;
        self
    }

    /// Its keys, and that they can be changed. See [`Self::keys`].
    #[inline]
    #[must_use]
    pub const fn with_keys(mut self, keys: &'a str) -> Self {
        self.keys = Some(keys);
        self
    }
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
    /// The `⚙` of a palette row, on hover.
    pub configure_keybinding: &'a str,
}

impl<'a> EditorLabels<'a> {
    /// The texts in the order of the fields, for a host that keeps them in a list.
    pub const fn from_texts(texts: [&'a str; 16]) -> Self {
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
            configure_keybinding,
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
            configure_keybinding,
        }
    }
}

impl Default for EditorLabels<'_> {
    #[inline]
    fn default() -> Self {
        Self::ENGLISH
    }
}

impl EditorLabels<'_> {
    /// The English texts, the default: also what the palette finds a translated command by.
    pub const ENGLISH: EditorLabels<'static> = EditorLabels {
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
        palette_hint: "Search the text (> for commands, : to go to a line)",
        configure_keybinding: "Configure Keybinding",
    };
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

    /// The host's request of it (see [`crate::CodeEditor::request_keys`]). `None`: Hover, by the
    /// keymap's own key (Vim `K`).
    pub(crate) const fn request(self) -> Option<crate::EditorRequest> {
        use crate::EditorRequest as Request;
        Some(match self {
            Self::GoToDefinition => Request::GoToDefinition,
            Self::FindReferences => Request::FindReferences,
            Self::Find => Request::Find,
            Self::GoToLine => Request::GoToLine,
            Self::QuickFix => Request::QuickFix,
            Self::Format => Request::Format,
            Self::Completion => Request::Completion,
            Self::ShowHover => return None,
        })
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
    /// The `⚙` of a host action: change its keys.
    Configure(&'a str),
    /// The `⚙` of one of the editor's commands whose keys are the host's: change them.
    ConfigureBuiltin(Builtin),
    /// A place of the text: a line found, or `:42`. 0-based, the column in chars.
    Jump {
        line: usize,
        column: usize,
    },
}

impl<'a> Picked<'a> {
    /// The id of a host action.
    pub(crate) const fn host_action(self) -> Option<&'a str> {
        match self {
            Self::Host(action) => Some(action),
            _ => None,
        }
    }

    /// The editor's command whose keys are to be changed, as the host asks for it.
    pub(crate) const fn configure_request(self) -> Option<crate::EditorRequest> {
        match self {
            Self::ConfigureBuiltin(command) => command.request(),
            _ => None,
        }
    }

    /// The id of the host action whose keys are to be changed.
    pub(crate) const fn configure(self) -> Option<&'a str> {
        match self {
            Self::Configure(action) => Some(action),
            _ => None,
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
    pub(crate) shortcut: &'a str,
    /// Its keys can be changed: a `⚙` on its row.
    pub(crate) configurable: bool,
    /// What else it is found by: e.g. its id, its English name. See [`EditorAction::aliases`].
    pub(crate) aliases: [&'a str; 3],
}

/// What the palette lists, by what its query starts with, as VS Code's quick open.
///
/// | query   | lists                                     |
/// | ------- | ----------------------------------------- |
/// | `>fmt`  | the commands matching `fmt`               |
/// | `:42`   | line 42, to go to                         |
/// | `speed` | the lines of the text with `speed` in it  |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mode {
    Commands,
    Line,
    Find,
}

/// The mode of `query` and what follows its mark. e.g. `>fmt` -> `(Commands, "fmt")`
pub(crate) fn mode(query: &str) -> (Mode, &str) {
    if let Some(rest) = query.strip_prefix('>') {
        (Mode::Commands, rest)
    } else if let Some(rest) = query.strip_prefix(':') {
        (Mode::Line, rest)
    } else {
        (Mode::Find, query)
    }
}

/// The 0-based line of `:42`'s `42`, kept in the text. `None`: not a number.
pub(crate) fn line_of(rest: &str, text: &str) -> Option<usize> {
    let line: usize = rest.trim().parse().ok()?;
    Some(line.saturating_sub(1).min(text.split('\n').count().saturating_sub(1)))
}

/// The most lines the find mode lists.
const MAX_FOUND: usize = 200;

/// The lines of `text` with `query` in them (ASCII case ignored), as `(line, column, the line)`, the
/// first [`MAX_FOUND`]. Nothing for an empty query.
///
/// e.g. `a\n  Speed\nb`, `speed` -> `[(1, 2, "  Speed")]`
pub(crate) fn find_lines<'t>(text: &'t str, query: &str) -> Vec<(usize, usize, &'t str)> {
    if query.trim().is_empty() {
        return Vec::new();
    }
    let needle = query.to_ascii_lowercase();
    let mut found = Vec::new();
    for (line_no, line) in text.split('\n').enumerate() {
        let lower = line.to_ascii_lowercase();
        if let Some(at) = lower.find(&needle) {
            found.push((line_no, line[..at].chars().count(), line));
            if found.len() == MAX_FOUND {
                break;
            }
        }
    }
    found
}

/// A request of the host with its keys, e.g. `(Format, "shift+alt+f")`: see
/// [`crate::CodeEditor::request_keys`].
pub(crate) type RequestKeys = (crate::EditorRequest, String);

/// The commands available now that match `query`: the host's first.
pub(crate) fn commands<'a>(
    query: &str,
    context: PaletteContext,
    labels: &EditorLabels<'a>,
    (actions, request_keys): (&'a [EditorAction<'a>], Option<&'a [RequestKeys]>),
) -> Vec<Command<'a>> {
    // The editor's commands with the host's keys, if it has them: those, and a `⚙`.
    let host_keys = |command: Builtin| {
        let request = command.request()?;
        let keys = request_keys?.iter().find(|(known, _)| *known == request).map_or("", |(_, keys)| keys.as_str());
        Some(keys)
    };
    let host = actions.iter().map(|action| Command {
        picked: Picked::Host(action.id),
        label: action.label,
        shortcut: action.keys.unwrap_or_default(),
        configurable: action.keys.is_some(),
        aliases: [action.id, action.aliases[0], action.aliases[1]],
    });
    let builtin = Builtin::ALL.into_iter().filter(|command| command.available(context)).map(|command| Command {
        picked: Picked::Builtin(command),
        label: command.label(labels),
        shortcut: host_keys(command).unwrap_or_else(|| command.shortcut()),
        configurable: host_keys(command).is_some(),
        aliases: [command.label(&EditorLabels::ENGLISH), "", ""],
    });
    // As VS Code's: by the label shown, or the English one, or the id.
    let found = |command: &Command<'_>| {
        std::iter::once(command.label).chain(command.aliases).any(|text| !text.is_empty() && matches_query(query, text))
    };
    host.chain(builtin).filter(|command| query.trim().is_empty() || found(command)).collect()
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
    /// Just opened: its field takes the keyboard when it is drawn. (A focus asked for by id before
    /// the field exists is lost: the field's own response asks, as the find bar's does.)
    pub(crate) focus_field: bool,
}

impl Palette {
    pub(crate) fn reopen(&mut self) {
        // As VS Code's `Ctrl+Shift+P`: `>` there, the commands. Taken away, the text is searched.
        *self = Self { open: true, focus_field: true, query: ">".to_owned(), ..Self::default() };
    }

    /// Moves the selection (wrapping around); `Enter` returns the selected command.
    pub(crate) fn on_keys<'a>(&mut self, keys: PaletteKeys, commands: &[Picked<'a>]) -> Option<Picked<'a>> {
        let last = commands.len().saturating_sub(1);
        self.selected = self.selected.min(last);
        if keys.up {
            self.selected = self.selected.checked_sub(1).unwrap_or(last);
        }
        if keys.down {
            self.selected = if self.selected >= last { 0 } else { self.selected + 1 };
        }
        if keys.enter { commands.get(self.selected).copied() } else { None }
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

    /// As VS Code's quick open: `>` the commands, `:` a line, else the text searched.
    #[test]
    fn the_query_chooses_what_is_listed() {
        assert_eq!(mode(">fmt"), (Mode::Commands, "fmt"));
        assert_eq!(mode(":42"), (Mode::Line, "42"));
        assert_eq!(mode("speed"), (Mode::Find, "speed"));
        let text = "a\n  Speed\nb speed";
        assert_eq!(find_lines(text, "speed"), [(1, 2, "  Speed"), (2, 2, "b speed")]);
        assert!(find_lines(text, " ").is_empty(), "nothing to look for");
        assert_eq!(line_of("2", text), Some(1));
        assert_eq!(line_of("99", text), Some(2), "kept in the text");
        assert_eq!(line_of("x", text), None);
    }

    /// A translated command is also found by its English name and its ids, as in VS Code.
    #[test]
    fn a_command_is_found_by_its_english_name_and_ids() {
        let context = PaletteContext { keymap: Keymap::Normal, features: EditorFeatures::default(), inserting: true };
        let japanese = EditorLabels { format: "ドキュメントのフォーマット", ..EditorLabels::ENGLISH };
        let actions = [EditorAction::new("close_tab", "タブを閉じる").with_aliases(["Close the tab", "tabs.close"])];
        let found = |query: &str| {
            commands(query, context, &japanese, (&actions, None)).iter().map(|c| c.label).collect::<Vec<_>>()
        };
        for query in ["タブ", "close the", "tabs.close", "close_tab"] {
            assert_eq!(found(query), ["タブを閉じる"], "{query}");
        }
        assert_eq!(found("format doc"), ["ドキュメントのフォーマット"], "a builtin by its English name");
    }

    const COMMANDS: [Command<'_>; 2] = [
        Command { picked: Picked::Host("a"), label: "A", shortcut: "", configurable: false, aliases: ["", "", ""] },
        Command { picked: Picked::Host("b"), label: "B", shortcut: "", configurable: false, aliases: ["", "", ""] },
    ];

    #[test]
    fn keys_move_the_selection_and_enter_picks() {
        let mut palette = Palette::default();
        let down = PaletteKeys { down: true, ..PaletteKeys::default() };
        let up = PaletteKeys { up: true, ..PaletteKeys::default() };
        let enter = PaletteKeys { enter: true, ..PaletteKeys::default() };

        assert_eq!(palette.on_keys(down, &COMMANDS.map(|c| c.picked)), None);
        assert_eq!(palette.selected, 1);
        assert_eq!(palette.on_keys(enter, &COMMANDS.map(|c| c.picked)), Some(Picked::Host("b")));
        palette.on_keys(down, &COMMANDS.map(|c| c.picked)); // wraps around
        assert_eq!(palette.selected, 0);
        palette.on_keys(up, &COMMANDS.map(|c| c.picked));
        assert_eq!(palette.selected, 1);
        assert_eq!(palette.on_keys(enter, &[]), None); // nothing matches the query
    }

    #[test]
    fn lists_only_what_applies() {
        let labels = EditorLabels::default();
        let actions = [EditorAction::new("save", "Save")];
        let vim_normal = PaletteContext { keymap: Keymap::Vim, features: EditorFeatures::default(), inserting: false };

        let listed = commands("", vim_normal, &labels, (&actions, None));
        assert_eq!(listed[0].picked, Picked::Host("save"));
        // Vim has its own search, and suggestions need Insert mode.
        assert!(!listed.iter().any(|c| c.picked == Picked::Builtin(Builtin::Find)));
        assert!(!listed.iter().any(|c| c.picked == Picked::Builtin(Builtin::Completion)));
        assert_eq!(commands("save", vim_normal, &labels, (&actions, None)).len(), 1);
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
