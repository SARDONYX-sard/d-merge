//! The editor's right-click menu, like Monaco's:
//!
//! ```text
//! #0106                       <- only over a symbol
//! Go to Definition      F12
//! Find References       Shift+F12
//! ─────────────────
//! Cut                   Ctrl+X   <- with a selection
//! Copy                  Ctrl+C   <- with a selection
//! Paste                 Ctrl+V
//! Select All            Ctrl+A
//! ─────────────────
//! Command Palette       Ctrl+Shift+P
//! ```

use crate::{EditorLabels, text_pos::LinePos};

/// What the menu asked for. Carried out by the editor, which owns the text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MenuAction {
    /// `references`: find the references instead of going to the definition.
    Navigate {
        pos: LinePos,
        references: bool,
    },
    Cut,
    Copy,
    Paste,
    SelectAll,
    CommandPalette,
}

/// What the menu depends on.
#[derive(Debug, Clone, Copy)]
pub(crate) struct MenuContext<'a> {
    /// The symbol that was right-clicked and its document, e.g. `#0106`.
    pub(crate) symbol: Option<(LinePos, &'a str)>,
    pub(crate) has_selection: bool,
}

/// One item: `label ...... shortcut`.
fn item(ui: &mut egui::Ui, enabled: bool, label: &str, shortcut: &str) -> bool {
    let button = egui::Button::new(label).shortcut_text(shortcut);
    let clicked = ui.add_enabled(enabled, button).clicked();
    if clicked {
        ui.close();
    }
    clicked
}

/// The menu's contents. Returns what was clicked.
pub(crate) fn contents(ui: &mut egui::Ui, labels: &EditorLabels<'_>, menu: MenuContext<'_>) -> Option<MenuAction> {
    let mut action = None;
    if let Some((pos, document)) = menu.symbol {
        ui.weak(document);
        if item(ui, true, labels.go_to_definition, "F12") {
            action = Some(MenuAction::Navigate { pos, references: false });
        }
        if item(ui, true, labels.find_references, "Shift+F12") {
            action = Some(MenuAction::Navigate { pos, references: true });
        }
        ui.separator();
    }
    let edits = [
        (MenuAction::Cut, menu.has_selection, labels.cut, "Ctrl+X"),
        (MenuAction::Copy, menu.has_selection, labels.copy, "Ctrl+C"),
        (MenuAction::Paste, true, labels.paste, "Ctrl+V"),
        (MenuAction::SelectAll, true, labels.select_all, "Ctrl+A"),
    ];
    for (edit, enabled, label, shortcut) in edits {
        if item(ui, enabled, label, shortcut) {
            action = Some(edit);
        }
    }
    ui.separator();
    if item(ui, true, labels.command_palette, "Ctrl+Shift+P") {
        action = Some(MenuAction::CommandPalette);
    }
    action
}
