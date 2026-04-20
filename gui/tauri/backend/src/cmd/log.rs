#[tauri::command]
pub(crate) async fn change_log_level(log_level: &str) -> Result<(), String> {
    tracing::debug!("Selected log level: {log_level}");
    tracing_rotation::change_level(log_level).or_else(|err| super::bail!(err))
}

#[tauri::command]
pub(crate) fn log(level: &str, message: String) {
    match level {
        _ if level.eq_ignore_ascii_case("trace") => tracing::trace!("{message}"),
        _ if level.eq_ignore_ascii_case("debug") => tracing::debug!("{message}"),
        _ if level.eq_ignore_ascii_case("info") => tracing::info!("{message}"),
        _ if level.eq_ignore_ascii_case("warn") => tracing::warn!("{message}"),
        _ if level.eq_ignore_ascii_case("error") => tracing::error!("{message}"),
        _ => tracing::warn!("Unknown log level: {level} - {message}"),
    }
}
