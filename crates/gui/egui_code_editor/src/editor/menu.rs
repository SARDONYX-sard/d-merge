//! The right-click menu, and what it and `Ctrl+click` navigate to.

use crate::{
    EditorLabels, Language,
    cursor::set_selection,
    edit_ops,
    editor::Navigation,
    state::CodeEditorState,
    text_pos::LinePos,
    ui,
    ui::{
        context_menu,
        context_menu::{MenuAction, MenuContext},
    },
};

/// Asks the language about what the keyboard, `Ctrl+click` or the right-click menu requested.
///
/// `requested`: `(position, references)` from `F12`, Vim `gd`, the menu and the like.
pub(crate) fn resolve_navigation<'a>(
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

/// The right-click menu. `menu_target` remembers the symbol under the pointer when it opened.
pub(crate) fn context_menu(
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
pub(crate) fn run_menu_action(
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
