use crate::error::{NotFoundLogDirSnafu, Result};
use snafu::ResultExt as _;
use tauri::Manager as _;

/// Initializes logger.
///
/// # Errors
/// Double init
pub(crate) fn init(app: &tauri::App) -> Result<()> {
    let resolver = app.path();
    let log_dir = &resolver.app_log_dir().context(NotFoundLogDirSnafu)?;
    let log_name = format!("{}.log", app.package_info().name);

    Ok(tracing_rotation::init(log_dir, &log_name)?)
}
