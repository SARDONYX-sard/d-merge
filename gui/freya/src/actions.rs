//! Operations triggered by the UI: mod list fetch, patch, reload of the settings files, etc.
//!
//! These take the [`AppState`] (all `Copy` handles), so they can be called from any event handler.

use std::{
    borrow::Cow,
    hash::{DefaultHasher, Hash as _, Hasher as _},
    path::Path,
    sync::{Arc, atomic::Ordering},
    time::{Duration, Instant},
};

use d_merge_gui_shared::{
    i18n::{I18nKey, I18nMap},
    mod_item::{self, ModItem, to_patches},
    patch::PatchProgress,
    settings::{BehaviorSettings, DataMode},
};
use freya::prelude::*;
use rayon::prelude::*;
use skyrim_data_dir::Runtime;

use crate::{
    state::{AppState, Notice, Shared},
    theme::colors,
};

// ── Notification ──────────────────────────────────────────────────────────────

pub(crate) fn notify(app: &AppState, text: impl Into<String>, color: impl Into<Option<Color>>) {
    let mut notify = app.notify;
    notify.set(Notice::new(text, color));
}

pub(crate) fn notify_error(app: &AppState, text: impl Into<String>) {
    notify(app, text, colors::RED);
}

pub(crate) fn clear_notification(app: &AppState) {
    notify(app, String::new(), None);
}

// ── Mod list ──────────────────────────────────────────────────────────────────

/// Stable row ID of a mod for the table. (The table identifies rows by `usize`.)
pub(crate) fn row_id(mod_id: &str) -> usize {
    let mut hasher = DefaultHasher::new();
    mod_id.hash(&mut hasher);
    hasher.finish() as usize
}

/// Re-create the table to show an order changed outside of it.
pub(crate) fn refresh_table(app: &AppState) {
    let mut generation = app.table_generation;
    *generation.write() += 1;
}

/// Refreshes the mod list in the background.
///
/// Any in-flight fetch is superseded: only the result of the last request is applied.
pub(crate) fn update_mod_list(app: &AppState, shared: &Shared) {
    let app = *app;
    tracing::debug!("`update_mod_list` has been called.");

    let (dir, use_vfs) = {
        let settings = app.settings.peek();
        (settings.current_skyrim_data_dir().to_owned(), settings.behavior.mode == DataMode::Vfs)
    };

    let mut fetching = app.fetching;
    let mut fetch_generation = app.fetch_generation;
    let mut mod_list_msg = app.mod_list_msg;

    fetching.set(true);
    *fetch_generation.write() += 1;
    let generation = *fetch_generation.peek();
    mod_list_msg
        .set(Notice::new(app.t_peek(I18nKey::ModsListFetchStateFetching), colors::LIGHT_BLUE));

    let runtime = shared.runtime.clone();
    let shared = shared.clone();
    let start_time = Instant::now();

    spawn_in_window(async move {
        // NOTE: `mod_info::get_all` is blocking.
        let result = runtime.spawn_blocking(move || mod_info::get_all(&dir, use_vfs)).await;

        if *app.fetch_generation.peek() != generation {
            return; // Outdated
        }
        let elapsed_secs = start_time.elapsed().as_secs_f32();

        let mut fetching = app.fetching;
        let mut last_fetch_was_empty = app.last_fetch_was_empty;
        let mut mod_list_msg = app.mod_list_msg;
        fetching.set(false);

        match result {
            Ok(Ok(mod_info)) if mod_info.is_empty() => {
                last_fetch_was_empty.set(true);
                mod_list_msg.set(Notice::new(
                    format!(
                        "{} ({elapsed_secs:.2} s)",
                        app.t_peek(I18nKey::ModsListFetchStateEmpty)
                    ),
                    None,
                ));
            }
            Ok(Ok(mod_info)) => {
                let mut settings = app.settings;
                {
                    let mut settings = settings.write();
                    let new_mods = mod_item::inherit_reorder_cast(settings.mod_list(), mod_info);
                    *settings.mod_list_mut() = new_mods;
                }
                last_fetch_was_empty.set(false);
                refresh_table(&app);

                mod_list_msg.set(Notice::new(
                    format!(
                        "{} ({elapsed_secs:.2} s)",
                        app.t_peek(I18nKey::ModsListFetchStateDone)
                    ),
                    colors::GREEN,
                ));

                if settings.peek().behavior.auto_run {
                    set_enabled(&app, None, true);
                    patch(&app, &shared);
                }
            }
            Ok(Err(e)) => {
                tracing::error!(%e, "mod_info::get_all error");
                mod_list_msg.set(Notice::new(
                    format!(
                        "{} ({elapsed_secs:.2} s)",
                        app.t_peek(I18nKey::ModsListFetchStateError)
                    ),
                    colors::RED,
                ));
            }
            Err(e) => {
                tracing::error!(%e, "mod list fetch task error");
                mod_list_msg.set(Notice::new(
                    format!(
                        "{} ({elapsed_secs:.2} s)",
                        app.t_peek(I18nKey::ModsListFetchStateError)
                    ),
                    colors::RED,
                ));
            }
        }
    });
}

/// Detects the Skyrim Data directory from the registry and refreshes the mod list if it changed.
pub(crate) fn update_vfs_skyrim_data_dir_by_reg(app: &AppState) {
    let target_runtime = app.settings.peek().behavior.target_runtime;

    match skyrim_data_dir::get_skyrim_data_dir(target_runtime) {
        Ok(dir) => {
            let new_path = dir.display().to_string();
            // NOTE: The mod list is refreshed by the effect watching the data directory.
            if app.settings.peek().vfs.skyrim_data_dir != new_path {
                let mut settings = app.settings;
                settings.write().vfs.skyrim_data_dir = new_path;
            }
        }
        Err(err) => {
            tracing::error!(%err);
            #[cfg(target_os = "windows")]
            let err_msg = app.t_peek(I18nKey::NotifyErrWindowsRegistryNotFound);
            #[cfg(not(target_os = "windows"))]
            let err_msg = app.t_peek(I18nKey::NotifyErrPlatformNotSupported);
            notify_error(app, err_msg);
        }
    }
}

/// Enables/disables the mods in `scope`(mod IDs). `None` means all mods.
pub(crate) fn set_enabled(app: &AppState, scope: Option<&[String]>, enabled: bool) {
    let mut settings = app.settings;
    let mut settings = settings.write();

    settings.mod_list_mut().par_iter_mut().for_each(|item| {
        if scope.is_none_or(|ids| ids.contains(&item.id)) {
            item.enabled = enabled;
        }
    });
}

/// Toggles one mod. With Shift held, the range from the anchor follows `visible_ids`(display order).
pub(crate) fn toggle_mod(app: &AppState, mod_id: &str, visible_ids: &[String]) {
    let mut settings = app.settings;
    let mut anchor = app.selection_anchor;
    let shift = *app.shift_held.peek();

    let enabled = {
        let mut settings = settings.write();
        let Some(item) = settings.mod_list_mut().iter_mut().find(|item| item.id == mod_id) else {
            return;
        };
        item.enabled = !item.enabled;
        item.enabled
    };

    let anchor_id = anchor.peek().clone();
    match anchor_id {
        Some(anchor_id) if shift => {
            let position = |id: &str| visible_ids.iter().position(|visible| visible == id);
            if let (Some(from), Some(to)) = (position(&anchor_id), position(mod_id)) {
                let range = &visible_ids[from.min(to)..=from.max(to)];
                set_enabled(app, Some(range), enabled);
            }
            anchor.set(None);
        }
        _ => anchor.set(Some(mod_id.to_string())),
    }
}

/// Applies the row order of the table(row IDs) to the mod list and renumbers the priorities.
///
/// The table only knows the rows it has shown. (e.g. it was created while filtering)
/// Unknown mods keep their places, and the known mods are reordered within the remaining places.
pub(crate) fn apply_row_order(app: &AppState, row_ids: &[usize]) {
    let mut settings = app.settings;
    let mut settings = settings.write();
    let mods = settings.mod_list_mut();

    let mut known: std::collections::HashMap<usize, ModItem> = std::collections::HashMap::new();
    for item in mods.iter() {
        let id = row_id(&item.id);
        if row_ids.contains(&id) {
            known.insert(id, item.clone());
        }
    }

    let mut reordered = row_ids.iter().filter_map(|id| known.remove(id));
    for item in mods.iter_mut() {
        if row_ids.contains(&row_id(&item.id))
            && let Some(next) = reordered.next()
        {
            *item = next;
        }
    }
    normalize_priorities(mods);
}

fn normalize_priorities(mods: &mut [ModItem]) {
    mods.par_iter_mut().enumerate().for_each(|(priority, item)| item.priority = priority);
}

/// Sort by type and ID, then renumber the priorities. In manual mode, duplicated mods are disabled.
pub(crate) fn normalize(app: &AppState) {
    {
        let mut settings = app.settings;
        let mut settings = settings.write();
        let is_manual = matches!(settings.behavior.mode, DataMode::Manual);

        mod_item::reorder_mods_priorities(settings.mod_list_mut());
        if is_manual {
            mod_item::dedup_mods_by_id(settings.mod_list_mut());
        }
    }
    refresh_table(app);
}

// ── Patch ─────────────────────────────────────────────────────────────────────

/// Builds the patch configuration from the current settings and starts `nemesis_merge::behavior_gen`.
pub(crate) fn patch(app: &AppState, shared: &Shared) {
    let app = *app;
    let start_time = Instant::now();
    shared.patch_status.clear();

    if app.settings.peek().behavior.auto_remove_meshes {
        remove_meshes_dir_all(&app);
    }

    let (patches, config) = {
        let settings = app.settings.peek();

        let (skyrim_data_dir, is_vfs, mod_list) = match settings.behavior.mode {
            DataMode::Vfs => (&settings.vfs.skyrim_data_dir, true, &settings.vfs.mod_list),
            DataMode::Manual => {
                (&settings.manual.skyrim_data_dir, false, &settings.manual.mod_list)
            }
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
            Runtime::Le => nemesis_merge::OutPutTarget::SkyrimLe,
            Runtime::Se | Runtime::Vr => nemesis_merge::OutPutTarget::SkyrimSe,
        };

        let patch_status = Arc::clone(&shared.patch_status);
        let config = nemesis_merge::Config {
            resource_dir: template_dir.to_string().into(),
            output_dir: settings.current_output_dir().to_owned().into(),
            output_target,
            status_report: Some(Box::new(move |status| apply_status(&patch_status, status))),
            parser_mode: (*parser_mode).into(),
            hack_options: Some(nemesis_merge::HackOptions::enable_all()),
            debug: nemesis_merge::DebugOptions {
                output_patch_json: enable_debug_output,
                output_merged_json: enable_debug_output,
                output_merged_xml: enable_debug_output,
            },
            skyrim_data_dir_glob: Some(skyrim_data_dir.clone()),
            generate_fnis_esp: *generate_fnis_esp,
            skeleton_arm_fix: *skeleton_arm_fix,
        };

        (patches, config)
    };

    shared.runtime.spawn(nemesis_merge::behavior_gen(patches, config));

    let mut patching = app.patching;
    patching.set(true);

    // Poll the progress written by the status-report callback(another thread).
    let patch_status = Arc::clone(&shared.patch_status);
    spawn_in_window(async move {
        loop {
            tokio::time::sleep(Duration::from_millis(100)).await;

            let text = patch_status.text(&app.i18n.peek(), start_time);
            if !text.is_empty() {
                notify(&app, text, status_color(&patch_status));
            }

            if matches!(patch_status.phase.load(Ordering::Relaxed), 6 | 7) {
                patch_status.phase.store(0, Ordering::Relaxed);
                break;
            }
        }

        let mut patching = app.patching;
        patching.set(false);
    });
}

/// Removes `<output_dir>/meshes` (and the debug cache) before patching.
///
/// Skipped with a warning when the output directory equals the Skyrim data
/// directory, because deleting `meshes/` there would destroy installed mods.
fn remove_meshes_dir_all(app: &AppState) {
    let (output_dir, is_dangerous) = {
        let settings = app.settings.peek();
        let output_dir = settings.current_output_dir().to_owned();
        let is_dangerous = nemesis_merge::cache_remover::is_dangerous_remove(
            &output_dir,
            settings.current_skyrim_data_dir(),
        );
        (output_dir, is_dangerous)
    };

    if is_dangerous {
        tracing::warn!(
            "0/6: `auto remove meshes` is enabled but output dir equals the Skyrim data \
             directory — skipping to avoid destroying installed mods."
        );
        return;
    }

    notify(
        app,
        format!("0/6: {} `{output_dir}/meshes`", app.t_peek(I18nKey::RemovingMeshesMessage)),
        None,
    );
    nemesis_merge::cache_remover::remove_meshes_dir_all(output_dir);
}

/// Writes a status of `nemesis_merge::behavior_gen` into the atomics. (Called from the patch thread)
fn apply_status(progress: &PatchProgress, status: nemesis_merge::Status) {
    use nemesis_merge::Status;

    let phase = match &status {
        Status::GeneratingFnisPatches { .. } => 1,
        Status::ReadingPatches { .. } => 2,
        Status::ParsingPatches { .. } => 3,
        Status::ApplyingPatches { .. } => 4,
        Status::GeneratingHkxFiles { .. } => 5,
        Status::Done => 6,
        Status::Error(_) => 7,
    };

    match status {
        Status::GeneratingFnisPatches { index, total }
        | Status::ReadingPatches { index, total }
        | Status::ParsingPatches { index, total }
        | Status::ApplyingPatches { index, total }
        | Status::GeneratingHkxFiles { index, total } => {
            progress.index.store(index, Ordering::Relaxed);
            progress.total.store(total, Ordering::Relaxed);
        }
        Status::Done => {}
        Status::Error(err) => *progress.error.write() = Some(err),
    }

    // NOTE: Store the phase last, so that the poller never sees a terminal phase without its error.
    progress.phase.store(phase, Ordering::Relaxed);
}

/// Color of the current patch phase.
fn status_color(progress: &PatchProgress) -> Color {
    match progress.snapshot().phase {
        1 => Color::from_rgb(120, 170, 255),
        2 => colors::LIGHT_BLUE,
        3 => Color::from_rgb(140, 200, 255),
        4 => Color::from_rgb(255, 170, 120),
        5 => Color::from_rgb(200, 140, 255),
        6 => colors::GREEN,
        7 => colors::RED,
        _ => Color::WHITE,
    }
}

// ── Settings files ────────────────────────────────────────────────────────────

/// Applies the log level immediately.
pub(crate) fn change_log_level(level: d_merge_gui_shared::log::LogLevel) {
    let _ = tracing_rotation::global::change_level(level.as_str());
}

/// Rotates the log file to the configured directory.
pub(crate) fn reload_log(app: &AppState) {
    let dir = app.settings.peek().log.dir_path.clone();

    if let Err(err) =
        tracing_rotation::global::change_log_path(&dir, d_merge_gui_shared::log::LOG_FILENAME)
    {
        tracing::error!(%err);
        notify_error(app, format!("Failed to reload log: {err}"));
    } else {
        crate::log::change_log_dir(&dir);
        tracing::info!("Log file rotated.");
        notify(app, "Log file rotated.", colors::GREEN);
    }
}

/// Reloads the i18n map from disk.
pub(crate) fn reload_i18n(app: &AppState) {
    let path = app.settings.peek().ui.i18n_path.clone();

    match I18nMap::load(path.as_str()) {
        Ok(i18n) => {
            let mut state = app.i18n;
            state.set(i18n);
            notify(app, format!("Reloaded {path}"), colors::GREEN);
        }
        Err(err) => notify_error(app, format!("Failed to reload: {err}")),
    }
}

/// Temporarily force English.
pub(crate) fn load_default_i18n(app: &AppState) {
    let mut state = app.i18n;
    state.set(I18nMap::new());
}

/// Writes the English translation file selected by the save dialog.
pub(crate) fn write_new_i18n(app: &AppState) {
    let i18n_path = app.settings.peek().ui.i18n_path.clone();
    let dir = parent_dir_or(&i18n_path, ".");

    let path = rfd::FileDialog::new()
        .set_directory(dir)
        .set_title("Save translation.json")
        .set_file_name("translation.json")
        .add_filter("translation", &["json"])
        .save_file();

    if let Some(path) = path {
        match I18nMap::save(&path) {
            Ok(()) => notify(app, format!("OK. Wrote {}", path.display()), colors::GREEN),
            Err(err) => notify_error(app, err.to_string()),
        }
    }
}

/// Applies the font settings.
pub(crate) fn reload_font(app: &AppState) {
    let font = app.settings.peek().ui.font.clone();
    let mut font_family = app.font_family;

    match crate::fonts::resolve_font_family(&font) {
        Ok(family) => font_family.set(family),
        Err(crate::fonts::FontError::Warn(msg)) => {
            tracing::warn!(msg);
            font_family.set(None);
            notify(app, msg, colors::YELLOW);
        }
        Err(crate::fonts::FontError::Error(msg)) => {
            tracing::error!(msg);
            font_family.set(None);
            notify_error(app, msg);
        }
    }
}

/// Shows the background image, reloading it from disk.
pub(crate) fn reload_background(app: &AppState) {
    let mut settings = app.settings;
    settings.write().ui.background_image.enabled = true;

    let mut generation = app.background_generation;
    *generation.write() += 1;
}

/// Returns the canonicalized parent directory of `path`, or `fallback`.
///
/// NOTE: Without this, rfd cannot set the dir correctly.
pub(crate) fn parent_dir_or<'a>(path: &'a str, fallback: &'a str) -> Cow<'a, Path> {
    Path::new(path).parent().map_or_else(
        || Cow::Borrowed(Path::new(fallback)),
        |p| p.canonicalize().map_or(Cow::Borrowed(p), Cow::Owned),
    )
}
