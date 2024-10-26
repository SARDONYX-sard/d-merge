use d_merge_core::mod_info::{GetModsInfo as _, ModInfo, ModsInfo};
use std::{collections::HashMap, path::Path};
use tauri::{Emitter as _, Window};

pub(crate) mod convert;

/// Early return with Err() and write log error.
macro_rules! bail {
    ($err:expr) => {{
        tracing::error!("{}", $err);
        return Err($err.to_string());
    }};
}

pub(super) use bail;

/// Measure the elapsed time and return the result of the given asynchronous function.
#[allow(unused)]
macro_rules! time {
    ($name:literal, $expr:expr) => {{
        let start = std::time::Instant::now();
        let res = $expr.or_else(|err| bail!(err));
        let elapsed = start.elapsed();
        tracing::info!(
            "{} time: {}.{}s.",
            $name,
            elapsed.as_secs(),
            elapsed.subsec_millis()
        );
        res
    }};
}

/// # Progress report for progress bar
///
/// - First: number of files/dirs explored
/// - After: working index
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Payload {
    /// - First: number of files/dirs explored
    /// - After: working index
    index: usize,
}

/// Closure that reports the number of files
macro_rules! sender {
    ($window:ident, $emit_name:literal) => {
        move |index: usize| {
            if let Err(err) = $window.emit($emit_name, Payload { index }) {
                tracing::error!("{}", err);
            };
        }
    };
}

/// # Progress report for progress bar
///
/// - First: number of files/dirs explored
/// - After: working index
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Settings {
    /// meshes, caches, settings
    out_dir: String,
    /// - Intended `./data` of `./data/Nemesis_Engine/mods/<id>/info.ini`
    mod_dir: String,
}

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
// Patch

#[tauri::command]
pub(crate) fn load_mods_info() -> Result<Vec<ModInfo>, String> {
    let dir = "../../dummy/mods";
    let pattern = format!("{dir}/*/info.ini");
    let info = ModsInfo::get_all(&pattern).or_else(|err| bail!(err))?;
    Ok(info.sort_to_vec_by_priority(HashMap::new()))
}

#[tauri::command]
pub(crate) fn load_activate_mods() -> Result<Vec<String>, String> {
    let mods = "
aaaaa
bcbi"
        .lines();
    Ok(mods.into_iter().map(|m| m.to_string()).collect())
}

#[tauri::command]
pub(crate) fn patch(window: Window, ids: Vec<String>) -> Result<(), String> {
    tracing::trace!("{:?}", ids);
    let _sender = sender!(window, "d_merge://progress/patch");
    Ok(())
}

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

#[tauri::command]
pub(crate) async fn change_log_level(log_level: &str) -> Result<(), String> {
    tracing::debug!("Selected log level: {log_level}");
    crate::log::change_level(log_level).or_else(|err| bail!(err))
}

/// Define our own `writeTextFile` api for tauri,
/// because there was a bug that contents were not written properly
/// (there was a case that the order of some data in contents was switched).
#[tauri::command]
pub(crate) async fn write_file(path: &Path, content: &str) -> Result<(), String> {
    std::fs::write(path, content).or_else(|err| bail!(err))
}
