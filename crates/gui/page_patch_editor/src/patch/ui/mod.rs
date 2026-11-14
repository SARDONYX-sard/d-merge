//! Drawing and key reading of the patch tool. The logic lives next to this directory, in plain
//! modules that do not need an `egui::Ui`, so it can be unit tested.

pub(super) mod anim_data;
pub(super) mod anim_tree;
mod behaviors;
mod changes_view;
pub(super) mod class_picker;
pub(super) mod code_choice;
mod object_list;
pub(super) mod panels;
pub(super) mod repeat;
mod search;
pub(super) mod side_bar;
pub(super) mod start_screen;
pub(super) mod template_picker;
pub(super) mod top_bar;
mod tree;
pub(super) mod user_patch;
pub(super) mod vars_panel;
pub(super) mod windows;

/// How far the job is, in place of the page while it runs: see [`crate::patch::tool::job`].
pub(super) fn job_progress(ui: &mut egui::Ui, i18n: &gui_shared::i18n::I18nMap, job: &crate::patch::tool::job::Job) {
    ui.add_space(12.0);
    ui.label(format!("{}: {}", i18n.t(job.label), job.detail));
    let (done, total) = job.counts();
    let (ratio, text) = match job.progress() {
        Some(ratio) => (ratio, format!("{done} / {total}")),
        None => (0.0, done.to_string()),
    };
    ui.add(egui::ProgressBar::new(ratio).text(text).animate(true));
}
