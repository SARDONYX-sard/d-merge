//! What is shown while no work is open, like VS Code's welcome page.
//!
//! ```text
//! Patch Editor
//! [New] [Open an existing mod...] [User Patch] [Open work data]
//!
//! Open Recent
//! mymod  D:/mods/MyMod
//! para   D:/mods/Paraglider
//! ```

use egui_widgets::{
    PathRow,
    notify::Notification,
    shadcn::{Button, ComboBox, Icon, TextEdit},
};
use gui_shared::i18n::{I18nKey, I18nMap};

use super::top_bar::{PATH_ACTIONS, recent_list};
use crate::patch::{
    PatchTool,
    workspace::{TemplateScope, Workspace},
};

impl PatchTool {
    pub(in crate::patch) fn start_screen(&mut self, ui: &mut egui::Ui, i18n: &I18nMap, notify: &mut Notification) {
        ui.add_space(12.0);
        ui.heading(i18n.t(I18nKey::PagePatchEditor));
        ui.weak(i18n.t(I18nKey::NemesisStartHint));
        ui.add_space(8.0);
        let has_output = !self.patch_output.is_empty();
        if !has_output {
            ui.colored_label(ui.visuals().warn_fg_color, i18n.t(I18nKey::NemesisNeedPatchOutput));
            ui.add_space(8.0);
        }
        // A work is kept in that directory.
        ui.add_enabled_ui(has_output, |ui| {
            self.start_buttons(ui, i18n, notify);
            self.new_work_form(ui, i18n, notify);
        });
        ui.add_space(12.0);
        ui.strong(i18n.t(I18nKey::NemesisOpenRecent));
        let picked = ui.add_enabled_ui(has_output, |ui| recent_list(ui, &self.recent, i18n)).inner;
        if let Some(work) = picked {
            self.open_recent(&work, notify, i18n);
        }
    }

    fn start_buttons(&mut self, ui: &mut egui::Ui, i18n: &I18nMap, notify: &mut Notification) {
        ui.horizontal(|ui| {
            let new = Button::new(i18n.t(I18nKey::NemesisNewWork)).icon(Icon::Plus).primary();
            if ui.add(new).on_hover_text(i18n.t(I18nKey::NemesisNewWorkHover)).clicked() {
                self.new_work.get_or_insert_default();
            }
            let open = Button::new(i18n.t(I18nKey::NemesisOpenMod)).icon(Icon::FolderSearch);
            if ui.add(open).on_hover_text(i18n.t(I18nKey::NemesisOpenModHover)).clicked() {
                self.pick_mod(notify, i18n);
            }
            self.open_cache_button(ui, i18n, notify);
        });
    }

    /// The mod code and the output of a new work.
    fn new_work_form(&mut self, ui: &mut egui::Ui, i18n: &I18nMap, notify: &mut Notification) {
        let Some(form) = &mut self.new_work else { return };
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            let hover = i18n.t(I18nKey::NemesisModCodeHover);
            ui.label(i18n.t(I18nKey::NemesisModCodeLabel)).on_hover_text(hover);
            let code = TextEdit::singleline(&mut form.code).hint_text(i18n.t(I18nKey::NemesisModCodeHint));
            ui.add(code.desired_width(90.0)).on_hover_text(hover);
        });
        let output = PathRow::new(i18n.t(I18nKey::NemesisOutputLabel), i18n.t(I18nKey::SelectButton), PATH_ACTIONS)
            .help(i18n.t(I18nKey::NemesisOutputHover))
            .show(ui, &mut form.output);
        if let Some(err) = output.open_error {
            notify.error(err);
        }
        scope_box(ui, &mut form.scope, i18n);

        let problem = form.problem(|code| Workspace::exists(&self.cache_root, code));
        let (mut create, mut cancel) = (false, false);
        ui.horizontal(|ui| {
            create = ui.add_enabled(problem.is_none(), Button::new(i18n.t(I18nKey::NemesisCreate)).primary()).clicked();
            cancel = ui.add(Button::new(i18n.t(I18nKey::CancelButton))).clicked();
            // Nothing to complain about before anything is typed.
            if let Some(problem) = problem.filter(|_| !form.code.is_empty()) {
                ui.colored_label(ui.visuals().warn_fg_color, i18n.t(problem));
            }
        });
        if create {
            let code = form.code.trim().to_owned();
            let output = form.output.trim().to_owned();
            let scope = form.scope;
            self.create_work(code, output, scope);
            ui.ctx().request_repaint();
        } else if cancel {
            self.new_work = None;
        }
    }
}

/// `Templates [Nemesis v]`: which templates the new work can patch. It can't be changed later.
fn scope_box(ui: &mut egui::Ui, scope: &mut TemplateScope, i18n: &I18nMap) {
    ui.horizontal(|ui| {
        let hover = i18n.t(I18nKey::NemesisScopeHover);
        ui.label(i18n.t(I18nKey::NemesisScopeLabel)).on_hover_text(hover);
        let mut selected = TemplateScope::ALL.iter().position(|(known, _)| known == scope).unwrap_or(0);
        let response = ComboBox::from_id_salt("nemesis_new_work_scope").show_index(
            ui,
            &mut selected,
            TemplateScope::ALL.len(),
            |index| TemplateScope::ALL[index].1,
        );
        if response.on_hover_text(hover).changed() {
            *scope = TemplateScope::ALL[selected].0;
        }
    });
}
