//! Runs the merge (`behavior_merge::behavior_gen`) on the async runtime and reports its progress.
//!
//! The merge calls `status_report` from its own threads. That writes into [`PatchProgress`]
//! (atomics, so the UI never waits for a lock), and [`PatchRun::poll`] shows it every frame.

use std::sync::{Arc, atomic::Ordering};

use behavior_merge::PatchMaps;
use egui_widgets::notify::{Notification, NotificationColor};
use gui_shared::{
    i18n::{I18nKey, I18nMap},
    patch::{PatchProgress, phase},
    settings::pages::patch::{
        PatchSettings,
        behavior::{BehaviorSettings, DataMode, ParserMode},
        mod_item::ModItem,
    },
};
use mod_info::ModType;
use rayon::{iter::Either, prelude::*};
use skyrim_data_dir::Runtime;

/// One merge at a time; see the module docs.
#[derive(Debug, Default)]
pub(crate) struct PatchRun {
    /// When the current merge started. `None` when none is running.
    started: Option<std::time::Instant>,
    progress: Arc<PatchProgress>,
}

impl PatchRun {
    /// Starts a merge of the enabled mods of the current mode.
    pub(crate) fn start(
        &mut self,
        ctx: &egui::Context,
        settings: &PatchSettings,
        notify: &mut Notification,
        i18n: &I18nMap,
        runtime: &tokio::runtime::Runtime,
    ) {
        self.started = Some(std::time::Instant::now());
        self.progress.clear();

        if settings.behavior.auto_remove_meshes {
            remove_meshes_dir_all(settings, notify, i18n);
        }

        let (skyrim_data_dir, is_vfs, mod_list) = match settings.behavior.mode {
            DataMode::Vfs => (&settings.vfs.skyrim_data_dir, true, &settings.vfs.mod_list),
            DataMode::Manual => (&settings.manual.skyrim_data_dir, false, &settings.manual.mod_list),
        };
        let patches = to_patches(skyrim_data_dir, is_vfs, mod_list);

        let BehaviorSettings {
            target_runtime,
            enable_debug_output,
            generate_fnis_esp,
            skeleton_arm_fix,
            template_dir,
            parser_mode,
            ..
        } = &settings.behavior;
        let enable_debug_output = *enable_debug_output;
        let output_target = match target_runtime {
            Runtime::Le | Runtime::Enderal => behavior_merge::OutPutTarget::SkyrimLe,
            Runtime::Se | Runtime::EnderalSe | Runtime::Vr => behavior_merge::OutPutTarget::SkyrimSe,
        };
        let progress = Arc::clone(&self.progress);
        let ctx = ctx.clone();
        let config = behavior_merge::Config {
            resource_dir: template_dir.to_string().into(),
            output_dir: settings.current_output_dir().to_owned().into(),
            output_target,
            status_report: Some(Box::new(move |status| progress.apply(status, &ctx))),
            parser_mode: to_nemesis_parser_mode(*parser_mode),
            hack_options: Some(behavior_merge::HackOptions::enable_all()),
            debug: behavior_merge::DebugOptions {
                output_patch_json: enable_debug_output,
                output_merged_json: enable_debug_output,
                output_merged_xml: enable_debug_output,
            },
            skyrim_data_dir_glob: Some(skyrim_data_dir.clone()),
            generate_fnis_esp: *generate_fnis_esp,
            skeleton_arm_fix: *skeleton_arm_fix,
        };

        runtime.spawn(behavior_merge::behavior_gen(patches, config));
    }

    /// Shows the progress in the notification bar. Call once per frame.
    /// With no merge running, it only checks an `Option`.
    pub(crate) fn poll(&mut self, notify: &mut Notification, i18n: &I18nMap) {
        let Some(started) = self.started else { return };

        let text = self.progress.text(i18n, started);
        if !text.is_empty() {
            notify.set(text, self.progress.color());
        }
        if matches!(self.progress.phase.load(Ordering::Relaxed), phase::DONE | phase::ERROR) {
            self.progress.phase.store(phase::IDLE, Ordering::Relaxed);
            self.started = None;
        }
    }
}

/// `auto_remove_meshes`: deletes the old output, unless it is the game's own data directory.
fn remove_meshes_dir_all(settings: &PatchSettings, notify: &mut Notification, i18n: &I18nMap) {
    let output_dir = settings.current_output_dir();

    if behavior_merge::cache_remover::is_dangerous_remove(output_dir, settings.current_skyrim_data_dir()) {
        tracing::warn!(
            "0/6: `auto remove meshes` is enabled but output dir equals the Skyrim data \
             directory — skipping to avoid destroying installed mods."
        );
        return;
    }

    notify.info(format!("0/6: {} `{output_dir}/meshes`", i18n.t(I18nKey::RemovingMeshesMessage)));
    behavior_merge::cache_remover::remove_meshes_dir_all(output_dir);
}

trait EguiDisplay {
    fn apply(&self, status: behavior_merge::Status, ctx: &egui::Context);
    fn color(&self) -> egui::Color32;
}

impl EguiDisplay for Arc<gui_shared::patch::PatchProgress> {
    /// Applies a new patch-generation status emitted by
    /// `behavior_merge::behavior_gen`.
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
    fn apply(&self, status: behavior_merge::Status, ctx: &egui::Context) {
        let old_phase = self.phase.load(Ordering::Relaxed);

        let new_phase = match &status {
            behavior_merge::Status::GeneratingFnisPatches { .. } => phase::GENERATING_FNIS_PATCHES,
            behavior_merge::Status::ReadingPatches { .. } => phase::READING_PATCHES,
            behavior_merge::Status::ParsingPatches { .. } => phase::PARSING_PATCHES,
            behavior_merge::Status::ApplyingPatches { .. } => phase::APPLYING_PATCHES,
            behavior_merge::Status::GeneratingHkxFiles { .. } => phase::GENERATING_HKX_FILES,
            behavior_merge::Status::Done => phase::DONE,
            behavior_merge::Status::Error(_) => phase::ERROR,
        };

        self.phase.store(new_phase, Ordering::Relaxed);

        match status {
            behavior_merge::Status::GeneratingFnisPatches { index, total }
            | behavior_merge::Status::ReadingPatches { index, total }
            | behavior_merge::Status::ParsingPatches { index, total }
            | behavior_merge::Status::ApplyingPatches { index, total }
            | behavior_merge::Status::GeneratingHkxFiles { index, total } => {
                self.index.store(index, Ordering::Relaxed);
                self.total.store(total, Ordering::Relaxed);
            }

            behavior_merge::Status::Done => {}
            behavior_merge::Status::Error(err) => *self.error.write() = Some(err),
        }

        // NOTE: Request a UI repaint when the phase changes or reaches a terminal state.
        // - If we do not do this, notifications that the patch has been applied may be significantly delayed.
        // - Comparing the status with the `old` phase is also important; without this, the UI will freeze.
        if old_phase != new_phase || matches!(new_phase, phase::DONE | phase::ERROR) {
            ctx.request_repaint();
        }
    }

    /// Returns the current UI color associated with the active patch phase.
    fn color(&self) -> egui::Color32 {
        let snapshot = self.snapshot();

        match snapshot.phase {
            phase::GENERATING_FNIS_PATCHES => egui::Color32::from_rgb(120, 170, 255),
            phase::READING_PATCHES => NotificationColor::RIGHT_BLUE,
            phase::PARSING_PATCHES => egui::Color32::from_rgb(140, 200, 255),
            phase::APPLYING_PATCHES => egui::Color32::from_rgb(255, 170, 120),
            phase::GENERATING_HKX_FILES => egui::Color32::from_rgb(200, 140, 255),
            phase::DONE => egui::Color32::GREEN,
            phase::ERROR => egui::Color32::RED,
            _ => egui::Color32::WHITE,
        }
    }
}

/// Settings enum -> `behavior_merge` enum.
///
/// Lives here (not in `gui_shared`) so that the settings crate does not depend on
/// the heavy `behavior_merge`.
const fn to_nemesis_parser_mode(mode: ParserMode) -> behavior_merge::ParserMode {
    match mode {
        ParserMode::Strict => behavior_merge::ParserMode::Strict,
        ParserMode::Lenient => behavior_merge::ParserMode::Lenient,
    }
}

/// Converts the enabled mods into `behavior_merge`'s input.
///
/// IDs become what `behavior_merge` expects:
/// - Nemesis/FNIS (manual): e.g. `aaaa`
/// - Nemesis (vfs): e.g. `<skyrim data dir>/Nemesis_Engine/mod/aaaa`
/// - FNIS: always the bare id
fn to_patches(skyrim_data_dir: &str, is_vfs: bool, mod_infos: &[ModItem]) -> PatchMaps {
    let (nemesis_entries, fnis_entries) = mod_infos.par_iter().filter(|item| item.enabled).partition_map(|mod_item| {
        let ModItem { id, priority, mod_type, .. } = mod_item;
        let priority = *priority;

        match mod_type {
            ModType::Nemesis => {
                let id = if is_vfs { format!("{skyrim_data_dir}/Nemesis_Engine/mod/{id}") } else { id.clone() };
                Either::Left((id, priority))
            }
            ModType::NemesisExt => {
                let id = if is_vfs { format!("{skyrim_data_dir}/Nemesis_EngineExt/mod/{id}") } else { id.clone() };
                Either::Left((id, priority))
            }
            ModType::Fnis => Either::Right((id.clone(), priority)),
        }
    });
    PatchMaps { nemesis_entries, fnis_entries }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str, mod_type: ModType, enabled: bool) -> ModItem {
        ModItem { enabled, id: id.to_owned(), name: String::new(), mod_type, site: String::new(), priority: 1 }
    }

    #[test]
    fn only_enabled_mods_are_merged_with_the_ids_the_merger_expects() {
        let mods = [
            item("aaaa", ModType::Nemesis, true),
            item("bbbb", ModType::NemesisExt, true),
            item("cccc", ModType::Fnis, true),
            item("off", ModType::Nemesis, false),
        ];

        let vfs = format!("{:?}", to_patches("D:/Data", true, &mods));
        assert!(vfs.contains("D:/Data/Nemesis_Engine/mod/aaaa"), "{vfs}");
        assert!(vfs.contains("D:/Data/Nemesis_EngineExt/mod/bbbb"), "{vfs}");
        assert!(!vfs.contains("off"), "{vfs}");
        assert!(!vfs.contains("Data/Nemesis_Engine/mod/cccc"), "{vfs}"); // FNIS: the bare id

        let manual = format!("{:?}", to_patches("D:/Data", false, &mods));
        assert!(!manual.contains("D:/Data"), "{manual}");
    }
}
