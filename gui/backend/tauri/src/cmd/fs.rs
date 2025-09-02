use super::bail;
use std::path::Path;

/// Define our own `writeTextFile` api for tauri,
/// because there was a bug that contents were not written properly
/// (there was a case that the order of some data in contents was switched).
#[tauri::command]
pub(crate) async fn write_file(path: &Path, content: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .or_else(|err| bail!(err))?;
    }
    tokio::fs::write(path, content)
        .await
        .or_else(|err| bail!(err))
}
