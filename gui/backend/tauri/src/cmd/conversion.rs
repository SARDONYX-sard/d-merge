use super::sender;
use rayon::prelude::*;
use serde_hkx_for_gui::par_walk_dir::DirEntry;
use tauri::Window;

/// Convert hkx <-> xml
/// -
#[tauri::command]
pub(crate) async fn convert(
    window: Window,
    inputs: Vec<String>,
    output: Option<String>,
    format: &str,
    roots: Option<Vec<String>>,
) -> Result<(), String> {
    let status_sender = sender(window, "d_merge://progress/convert");
    let result = serde_hkx_for_gui::convert(inputs, output, format, roots, status_sender).await;

    result.map_err(|err| {
        let errs = err.to_string();
        tracing::error!("{errs}");
        errs
    })
}

/// Loads a directory structure from the specified path, filtering by allowed extensions.
///
/// # Errors
/// Returns an error message if the directory cannot be loaded or if there are issues reading the path.
#[tauri::command]
pub fn load_dir_node(dirs: Vec<String>) -> Result<Vec<DirEntry>, String> {
    serde_hkx_for_gui::par_walk_dir::load_dir_node(dirs).map_err(|errs| {
        let err = errs
            .par_iter()
            .map(|e| e.to_string())
            .collect::<Vec<String>>()
            .join("\n");

        tracing::error!("{err}");
        err
    })
}
