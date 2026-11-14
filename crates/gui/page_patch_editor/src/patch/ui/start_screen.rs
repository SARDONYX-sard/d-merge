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

use super::top_bar::{BarAsk, PATH_ACTIONS, recent_list};
use crate::patch::{
    new_work::NewWork,
    tool::work_state::Work,
    ui::panels::Panels,
    workspace::{TemplateScope, Workspace},
};

/// See the module docs. Returns what was asked: see [`super::top_bar::apply`].
pub(in crate::patch) fn start_screen(
    ui: &mut egui::Ui,
    i18n: &I18nMap,
    (work, panels): (&Work, &mut Panels),
    notify: &mut Notification,
) -> Vec<BarAsk> {
    ui.add_space(12.0);
    ui.heading(i18n.t(I18nKey::PagePatchEditor));
    ui.weak(i18n.t(I18nKey::NemesisStartHint));
    ui.add_space(8.0);
    let has_output = !work.place.patch_output.is_empty();
    if !has_output {
        ui.colored_label(ui.visuals().warn_fg_color, i18n.t(I18nKey::NemesisNeedPatchOutput));
        ui.add_space(8.0);
    }

    // A work is kept in that directory.
    let mut asks = Vec::new();
    ui.add_enabled_ui(has_output, |ui| {
        asks.extend(start_buttons(ui, i18n, work, &mut panels.new_work));
        asks.extend(new_work_form(ui, i18n, work, &mut panels.new_work, notify));
    });

    ui.add_space(12.0);
    ui.strong(i18n.t(I18nKey::NemesisOpenRecent));
    let picked = ui.add_enabled_ui(has_output, |ui| recent_list(ui, &work.recent, i18n)).inner;
    asks.extend(picked.map(BarAsk::OpenRecent));
    asks
}

/// `[New] [Open an existing mod...] [Open work data]`. `New` opens the form of `new_work`.
fn start_buttons(ui: &mut egui::Ui, i18n: &I18nMap, work: &Work, new_work: &mut Option<NewWork>) -> Option<BarAsk> {
    let mut asked = None;
    ui.horizontal(|ui| {
        let new = Button::new(i18n.t(I18nKey::NemesisNewWork)).icon(Icon::Plus).primary();
        if ui.add(new).on_hover_text(i18n.t(I18nKey::NemesisNewWorkHover)).clicked() {
            new_work.get_or_insert_default();
        }
        let open = Button::new(i18n.t(I18nKey::NemesisOpenMod)).icon(Icon::FolderSearch);
        if ui.add(open).on_hover_text(i18n.t(I18nKey::NemesisOpenModHover)).clicked() {
            asked = Some(BarAsk::OpenMod);
        }
        if super::top_bar::cache_button(ui, i18n, &work.place.cache_dir()) {
            asked = Some(BarAsk::OpenCache);
        }
    });
    asked
}

/// The mod code and the output of a new work, while its form is open (`new_work`).
fn new_work_form(
    ui: &mut egui::Ui,
    i18n: &I18nMap,
    work: &Work,
    new_work: &mut Option<NewWork>,
    notify: &mut Notification,
) -> Option<BarAsk> {
    let form = new_work.as_mut()?;
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        let hover = i18n.t(I18nKey::NemesisModCodeHover);
        ui.label(i18n.t(I18nKey::NemesisModCodeLabel)).on_hover_text(hover);
        let code = TextEdit::singleline(&mut form.code).hint_text(i18n.t(I18nKey::NemesisModCodeHint));
        ui.add(code.desired_width(90.0)).on_hover_text(hover);
    });
    let output = PathRow::new(i18n.t(I18nKey::OutputLabel), i18n.t(I18nKey::SelectButton), PATH_ACTIONS)
        .help(i18n.t(I18nKey::NemesisOutputHover))
        .show(ui, &mut form.output);
    if let Some(err) = output.open_error {
        notify.error(err);
    }
    scope_box(ui, &mut form.scope, i18n);

    let problem = form.problem(|code| Workspace::exists(&work.place.cache_root, code));
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
        ui.ctx().request_repaint();
        return new_work.take().map(BarAsk::CreateWork);
    }
    if cancel {
        *new_work = None;
    }
    None
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
