//! Paths shared across both execution modes.

use crate::{log::LogLevel, settings::ui::WindowGeometry};

/// Options that logging.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct LogSettings {
    /// Directory where rotating log files are written.
    ///
    /// Changes require an application restart to take effect (the log
    /// watcher is started once on the first frame).
    pub dir_path: String,

    /// Minimum severity level written to the rotating log file.
    ///
    /// Changes take effect immediately (no restart required) via
    /// `tracing_rotation::global::change_level`.
    pub level: LogLevel,

    pub window: WindowGeometry,
}

impl Default for LogSettings {
    fn default() -> Self {
        Self {
            dir_path: crate::log::LOG_DIR.into(),
            level: LogLevel::Info,
            window: WindowGeometry::default(),
        }
    }
}
