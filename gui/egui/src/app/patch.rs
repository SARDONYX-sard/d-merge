//! Behavior-generation (patch) runner.
//!
//! # Responsibilities
//! - [`App::patch`] — assembles [`nemesis_merge::Config`] from current
//!   settings and spawns the async task on [`App::async_rt`].
//! - [`App::poll_patch_result`] — called every frame; reads the latest
//!   [`nemesis_merge::Status`] and forwards it to the notification bar.
//! - [`App::remove_meshes_dir_all`] — optional pre-patch cleanup of the
//!   output `meshes/` directory, with a safety guard against accidental
//!   deletion inside the Skyrim data directory.
//!
//! # Async model
//! `nemesis_merge::behavior_gen` is a `Future` driven by the dedicated
//! `tokio` runtime stored in [`App::async_rt`].  Progress is reported
//! through a `status_report` callback that writes into
//! `Arc<RwLock<Option<nemesis_merge::Status>>>`.  The UI thread reads this
//! every frame via `try_read` — never blocking.

use std::sync::{Arc, atomic::Ordering};

use d_merge_gui_shared::{
    i18n::I18nKey,
    mod_item::to_patches,
    settings::{BehaviorSettings, DataMode},
};
use skyrim_data_dir::Runtime;

use crate::app::App;

impl App {
    /// Assembles patch configuration from current settings and starts
    /// `nemesis_merge::behavior_gen` on the async runtime.
    ///
    /// If [`AppSettings::auto_remove_meshes`] is set, the output `meshes/`
    /// directory is removed first (see [`App::remove_meshes_dir_all`]).
    pub(crate) fn patch(&mut self, ctx: &egui::Context) {
        self.patch_start_time = Some(std::time::Instant::now());
        self.patch_status.clear();

        if self.settings.behavior.auto_remove_meshes {
            self.remove_meshes_dir_all();
        }

        let (skyrim_data_dir, is_vfs, mod_list) = match self.settings.behavior.mode {
            DataMode::Vfs => {
                (&self.settings.vfs.skyrim_data_dir, true, &self.settings.vfs.mod_list)
            }
            DataMode::Manual => {
                (&self.settings.manual.skyrim_data_dir, false, &self.settings.manual.mod_list)
            }
        };
        let patches = to_patches(skyrim_data_dir, is_vfs, mod_list);

        let BehaviorSettings {
            target_runtime,
            enable_debug_output,
            generate_fnis_esp,
            template_dir,
            ..
        } = &self.settings.behavior;
        let enable_debug_output = *enable_debug_output;
        let output_target = match target_runtime {
            Runtime::Le => nemesis_merge::OutPutTarget::SkyrimLe,
            Runtime::Se | Runtime::Vr => nemesis_merge::OutPutTarget::SkyrimSe,
        };

        let patch_status = Arc::clone(&self.patch_status);

        let debug = nemesis_merge::DebugOptions {
            output_patch_json: enable_debug_output,
            output_merged_json: enable_debug_output,
            output_merged_xml: enable_debug_output,
        };

        let ctx = ctx.clone();
        let config = nemesis_merge::Config {
            resource_dir: template_dir.to_string().into(),
            output_dir: self.settings.current_output_dir().to_owned().into(),
            output_target,
            status_report: Some(Box::new(move |status| {
                patch_status.apply(status, &ctx);
            })),
            hack_options: Some(nemesis_merge::HackOptions::enable_all()),
            debug,
            skyrim_data_dir_glob: Some(skyrim_data_dir.clone()),
            generate_fnis_esp: *generate_fnis_esp,
        };

        self.async_rt.spawn(nemesis_merge::behavior_gen(patches, config));
    }

    /// Polls the patch status written by the `status_report` callback and
    /// forwards it to the notification bar.
    ///
    /// Must be called once per frame from [`eframe::App::update`].
    /// Short-circuits immediately when no patch is in progress
    /// (`patch_start_time` is `None`), keeping the happy-path cost to a
    /// single `Option` check.
    pub(crate) fn poll_patch_result(&mut self) {
        // Check start_time first to avoid an unnecessary RwLock read every frame.
        let Some(start_time) = self.patch_start_time else {
            return;
        };

        let text = self.patch_status.text(&self.i18n, start_time);
        if !text.is_empty() {
            self.set_colored_notify(text, self.patch_status.color());
        }

        if matches!(self.patch_status.phase.load(Ordering::Relaxed), 6 | 7) {
            self.patch_status.phase.store(0, Ordering::Relaxed);
            self.patch_start_time = None;
        }
    }

    /// Removes `<output_dir>/meshes` (and the debug cache) before patching.
    ///
    /// Skipped with a warning when the output directory equals the Skyrim data
    /// directory, because deleting `meshes/` there would destroy installed mods.
    fn remove_meshes_dir_all(&mut self) {
        let output_dir = self.settings.current_output_dir().to_owned();
        let skyrim_data_dir = self.settings.current_skyrim_data_dir();

        if nemesis_merge::cache_remover::is_dangerous_remove(&output_dir, skyrim_data_dir) {
            tracing::warn!(
                "0/6: `auto remove meshes` is enabled but output dir equals the Skyrim data \
                 directory — skipping to avoid destroying installed mods."
            );
            return;
        }

        self.notify_info(format!(
            "0/6: {} `{output_dir}/meshes`",
            self.t(I18nKey::RemovingMeshesMessage)
        ));
        nemesis_merge::cache_remover::remove_meshes_dir_all(output_dir);
    }
}

pub(crate) const EGUI_RIGHT_BLUE: egui::Color32 = egui::Color32::from_rgb(120, 220, 255);

pub(crate) trait EguiDisplay {
    fn apply(&self, status: nemesis_merge::Status, ctx: &egui::Context);
    fn color(&self) -> egui::Color32;
}

impl EguiDisplay for Arc<d_merge_gui_shared::patch::PatchProgress> {
    /// Applies a new patch-generation status emitted by
    /// `nemesis_merge::behavior_gen`.
    ///
    /// High-frequency progress updates are written into atomics so the egui
    /// render loop can poll them without lock contention.
    ///
    /// Terminal states:
    /// - `Done`
    /// - `Error`
    ///
    /// are stored separately because they occur rarely compared to normal
    /// progress updates.
    fn apply(&self, status: nemesis_merge::Status, ctx: &egui::Context) {
        let old_phase = self.phase.load(Ordering::Relaxed);

        let new_phase = match &status {
            nemesis_merge::Status::GeneratingFnisPatches { .. } => 1,
            nemesis_merge::Status::ReadingPatches { .. } => 2,
            nemesis_merge::Status::ParsingPatches { .. } => 3,
            nemesis_merge::Status::ApplyingPatches { .. } => 4,
            nemesis_merge::Status::GeneratingHkxFiles { .. } => 5,
            nemesis_merge::Status::Done => 6,
            nemesis_merge::Status::Error(_) => 7,
        };

        self.phase.store(new_phase, Ordering::Relaxed);

        match status {
            nemesis_merge::Status::GeneratingFnisPatches { index, total }
            | nemesis_merge::Status::ReadingPatches { index, total }
            | nemesis_merge::Status::ParsingPatches { index, total }
            | nemesis_merge::Status::ApplyingPatches { index, total }
            | nemesis_merge::Status::GeneratingHkxFiles { index, total } => {
                self.index.store(index, Ordering::Relaxed);
                self.total.store(total, Ordering::Relaxed);
            }

            nemesis_merge::Status::Done => {}
            nemesis_merge::Status::Error(err) => *self.error.write() = Some(err),
        }

        // NOTE: Request a UI repaint when the phase changes or reaches a terminal state.
        // - If we do not do this, notifications that the patch has been applied may be significantly delayed.
        // - Comparing the status with the `old` phase is also important; without this, the UI will freeze.
        if old_phase != new_phase || matches!(new_phase, 6 | 7) {
            ctx.request_repaint();
        }
    }

    /// Returns the current UI color associated with the active patch phase.
    fn color(&self) -> egui::Color32 {
        let snapshot = self.snapshot();

        match snapshot.phase {
            1 => egui::Color32::from_rgb(120, 170, 255),
            2 => super::patch::EGUI_RIGHT_BLUE,
            3 => egui::Color32::from_rgb(140, 200, 255),
            4 => egui::Color32::from_rgb(255, 170, 120),
            5 => egui::Color32::from_rgb(200, 140, 255),
            6 => egui::Color32::GREEN,
            7 => egui::Color32::RED,
            _ => egui::Color32::WHITE,
        }
    }
}
