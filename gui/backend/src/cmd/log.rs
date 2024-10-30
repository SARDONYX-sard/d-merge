use super::bail;

#[tauri::command]
pub(crate) async fn change_log_level(log_level: &str) -> Result<(), String> {
    tracing::debug!("Selected log level: {log_level}");
    crate::log::change_level(log_level).or_else(|err| bail!(err))
}
