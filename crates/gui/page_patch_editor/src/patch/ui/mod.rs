//! Drawing and key reading of the patch tool. The logic lives next to this directory, in plain
//! modules that do not need an `egui::Ui`, so it can be unit tested.

pub(super) mod anim_data;
pub(super) mod bulk;
pub(super) mod class_picker;
pub(super) mod code_choice;
pub(super) mod panels;
pub(super) mod ref_mods;
pub(super) mod repeat;
pub(super) mod start_screen;
pub(super) mod template_picker;
pub(super) mod top_bar;
pub(super) mod user_patch;
pub(super) mod windows;

/// How far the job is, in place of the page while it runs: see [`crate::patch::tool::job`].
///
/// ```text
/// ◌ Reading the work: upatch  1615        the total not known: what was read so far
/// ◌ Renaming the mod code: oldc -> newc
/// [██████████          120 / 300        ]  the total known: a bar, its count inside it
/// ```
///
/// The spinner is beside the words, not in the bar: egui draws an animated bar's spinner over its text.
pub(super) fn job_progress(ui: &mut egui::Ui, i18n: &gui_shared::i18n::I18nMap, job: &crate::patch::tool::job::Job) {
    ui.add_space(12.0);
    let (done, total) = job.counts();
    let ratio = job.progress();

    ui.horizontal(|ui| {
        ui.add(egui::Spinner::new());
        ui.label(format!("{}: {}", i18n.t(job.label), job.detail));
        if ratio.is_none() && done > 0 {
            ui.weak(egui::RichText::new(done.to_string()).monospace());
        }
    });
    if let Some(ratio) = ratio {
        ui.add(egui::ProgressBar::new(ratio).text(format!("{done} / {total}")));
    }
}
