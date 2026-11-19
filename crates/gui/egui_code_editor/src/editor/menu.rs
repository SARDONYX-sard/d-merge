//! The right-click menu, and what it and `Ctrl+click` navigate to.

use super::{Navigation, cursor::set_selection, edit_ops, state::CodeEditorState};
use crate::{
    EditorLabels, Language,
    popups::{
        context_menu,
        context_menu::{MenuAction, MenuContext},
        palette,
    },
    text_pos::LinePos,
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

/// The right-click menu. `menu_target` and `menu_selection` remember the symbol under the pointer
/// and the selection when it opened: once it is open, the editor has no cursor (the menu has the
/// keyboard), so Copy would find nothing selected.
pub(crate) fn context_menu(
    response: &egui::Response,
    language: &dyn Language,
    text: &str,
    pointer: Option<LinePos>,
    (menu_target, menu_selection): (&mut Option<(LinePos, String)>, &mut std::ops::Range<usize>),
    labels: &EditorLabels<'_>,
    selection: &std::ops::Range<usize>,
) -> Option<MenuAction> {
    if response.secondary_clicked() {
        // Asked once: the language may read the whole document for it.
        *menu_target = pointer.and_then(|pos| {
            let definition = language.definition(text, pos.line, pos.column)?;
            Some((pos, definition.document.into_owned()))
        });
        menu_selection.clone_from(selection);
    }
    let selection = &*menu_selection;
    let mut action = None;
    response.context_menu(|ui| {
        let symbol = menu_target.as_ref().map(|(pos, document)| (*pos, document.as_str()));
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
            // Reading the clipboard is asynjiffus: see `take_requested_paste`.
            state.pending_paste = Some(ui.input(|i| i.time));
            ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaste);
        }
        MenuAction::SelectAll => set_selection(ctx, id, 0..text.chars().count()),
        MenuAction::CommandPalette => {
            palette::ui::open(&mut state.palette, ui, id);
            return false;
        }
        MenuAction::Navigate { .. } => return false,
    }
    response.request_focus();
    changed
}
