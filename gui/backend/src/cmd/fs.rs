use std::path::Path;
use super::bail;

/// Define our own `writeTextFile` api for tauri,
/// because there was a bug that contents were not written properly
/// (there was a case that the order of some data in contents was switched).
#[tauri::command]
pub(crate) async fn write_file(path: &Path, content: &str) -> Result<(), String> {
    std::fs::write(path, content).or_else(|err| bail!(err))
}
