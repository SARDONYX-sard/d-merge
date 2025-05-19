#[tauri::command]
pub(crate) async fn open(path: &str, app: Option<&str>) -> Result<(), String> {
    match app {
        Some(app) => ::open::with_detached(path, app),
        None => ::open::that_detached(path),
    }
    .or_else(|err| crate::cmd::bail!(err))
}
