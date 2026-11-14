//! The top of the Changes view: the branch, and the commit box.
//!
//! ```text
//! ⎇ [main ▼] [+]                          another branch puts the work on it; `+` starts one
//! [message...................] [Commit]   `Ctrl+Enter` in the field too
//! ```

use egui_widgets::shadcn::{Button, ComboBox, Icon};
use gui_shared::i18n::{I18nKey, I18nMap};

use crate::patch::workspace::history::History;

/// What the branch row asked for.
#[derive(Debug)]
pub(super) enum BranchAsk {
    Switch(String),
    Create(String),
}

/// `⎇ [main ▼] [+]`, as VS Code's status of git. `new_branch`: the name being typed, while the `+`
/// field is open.
pub(super) fn branch_row(
    ui: &mut egui::Ui,
    i18n: &I18nMap,
    history: &History,
    new_branch: &mut Option<String>,
) -> Option<BranchAsk> {
    let current = history.branch_name();
    let mut asked = None;

    ui.horizontal(|ui| {
        ui.label("⎇").on_hover_text(i18n.t(I18nKey::SideBranchHover));
        ComboBox::from_id_salt("side_branch").selected_text(current).show_ui(ui, |ui| {
            for name in history.branch_names() {
                if ui.selectable_label(name == current, name).clicked() && name != current {
                    asked = Some(BranchAsk::Switch(name.to_owned()));
                }
            }
        });

        if let Some(created) = new_branch_field(ui, i18n, new_branch) {
            asked = Some(BranchAsk::Create(created));
        }
    });
    asked
}

/// `+`, then the field of the new branch's name: `Enter` makes it, leaving the field cancels it.
fn new_branch_field(ui: &mut egui::Ui, i18n: &I18nMap, new_branch: &mut Option<String>) -> Option<String> {
    let Some(name) = new_branch else {
        let plus = ui.add(Button::new("").icon(Icon::Plus).small()).on_hover_text(i18n.t(I18nKey::SideNewBranch));
        if plus.clicked() {
            *new_branch = Some(String::new());
        }
        return None;
    };

    let field = egui::TextEdit::singleline(name).hint_text(i18n.t(I18nKey::SideNewBranch)).desired_width(120.0);
    let field = ui.add(field);
    if !field.lost_focus() {
        field.request_focus();
        return None;
    }

    let entered = ui.input(|input| input.key_pressed(egui::Key::Enter));
    let name = std::mem::take(name);
    *new_branch = None;
    entered.then_some(name)
}

/// `[message............] [Commit]`. `can_commit`: there are changes. Returns whether to commit.
pub(super) fn commit_box(ui: &mut egui::Ui, i18n: &I18nMap, message: &mut String, has_changes: bool) -> bool {
    let can_commit = has_changes && !message.trim().is_empty();
    let mut commit = false;

    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let button = Button::new(i18n.t(I18nKey::SideCommit)).small().primary();
            commit |= ui.add_enabled(can_commit, button).on_hover_text(i18n.t(I18nKey::SideCommitHover)).clicked();

            let field = egui::TextEdit::singleline(message)
                .hint_text(i18n.t(I18nKey::SideCommitMessageHint))
                .desired_width(ui.available_width());
            let response = ui.add(field);
            let ctrl_enter = ui.input(|input| input.modifiers.command && input.key_pressed(egui::Key::Enter));
            commit |= can_commit && response.lost_focus() && ctrl_enter;
        });
    });
    commit
}
