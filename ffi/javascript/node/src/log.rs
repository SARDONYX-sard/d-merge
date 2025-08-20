use napi::bindgen_prelude::*;
use napi_derive::napi;
use tracing::{debug, error, info, trace, warn};

/// Initializes the logger with a specified directory and log file name.
///
/// # Examples
/// ```ts
/// import path from 'node:path';
///
/// const logDir = path.join(__dirname, 'logs');
/// loggerInit(logDir, 'node_ffi_test.log');
/// ```
///
/// # Errors
/// An error occurs when initialization is attempted twice.
#[napi]
pub fn logger_init(log_dir: String, log_name: String) -> Result<()> {
    tracing_rotation::init(log_dir, &log_name).map_err(|e| napi::Error::from_reason(e.to_string()))
}

/// Changes the current logging level.
///
/// # Errors
/// If logger uninitialized.
///
/// # Examples
/// ```ts
/// changeLogLevel('debug'); // Supported: "trace" | "debug" | "info" | "warn" | "error"
/// ```
///
/// # Note
/// - If unknown log level. fallback to `error`.(And write log warn)
/// - `level` - Logging level as a string. Supported values: `"trace"`, `"debug"`, `"info"`, `"warn"`, `"error"`.
// #[napi(ts_args_type = "level: \"trace\" | \"debug\" | \"info\" | \"warn\" | \"error\"")]
#[napi]
pub fn change_log_level(level: String) -> Result<()> {
    tracing_rotation::change_level(&level).map_err(|e| napi::Error::from_reason(e.to_string()))
}

/// Logs a message at the TRACE level.
///
/// # Examples
/// ```ts
/// logTrace('This is a trace message');
/// ```
#[napi]
pub fn log_trace(msg: String) {
    trace!("{}", msg);
}

/// Logs a message at the DEBUG level.
///
/// # Examples
/// ```ts
/// logDebug('This is a debug message');
/// ```
#[napi]
pub fn log_debug(msg: String) {
    debug!("{}", msg);
}

/// Logs a message at the INFO level.
///
/// # Examples
/// ```ts
/// logInfo('This is an info message');
/// ```
#[napi]
pub fn log_info(msg: String) {
    info!("{}", msg);
}

/// Logs a message at the WARN level.
///
/// # Examples
/// ```ts
/// logWarn('This is a warning message');
/// ```
#[napi]
pub fn log_warn(msg: String) {
    warn!("{}", msg);
}

/// Logs a message at the ERROR level.
///
/// # Examples
/// ```ts
/// logError('This is an error message');
/// ```
#[napi]
pub fn log_error(msg: String) {
    error!("{}", msg);
}
