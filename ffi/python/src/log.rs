use pyo3::prelude::*;
use tracing::{debug, error, info, trace, warn};

#[pyo3_stub_gen::derive::gen_stub_pyfunction]
#[pyo3::pyfunction]
/// Initializes the logger with a specified directory and log file name.
///
/// # Errors
/// An error occurs when initialization is attempted twice.
///
/// # Examples
///
/// ```python
/// from d_merge_python import logger_init
///
/// logger_init("./test/logs", "d_merge_python.log")
/// ```
pub fn logger_init(log_dir: String, log_name: String) -> PyResult<()> {
    tracing_rotation::init(log_dir, &log_name)
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
}

#[pyo3_stub_gen::derive::gen_stub_pyfunction]
#[pyo3::pyfunction]
/// Changes the current logging level.
///
/// # Errors
/// If logger uninitialized.
///
/// # Examples
///
/// ```python
/// from d_merge_python import change_log_level
///
/// change_log_level("debug");
/// ```
///
/// # Note
/// - If unknown log level. fallback to `error`.(And write log warn)
pub fn change_log_level(
    #[gen_stub(override_type(type_repr="typing.Literal[\"trace\", \"debug\", \"info\", \"warn\", \"error\"]", imports=("typing")))]
    level: String,
) -> PyResult<()> {
    tracing_rotation::change_level(&level)
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
}

#[pyo3_stub_gen::derive::gen_stub_pyfunction]
#[pyo3::pyfunction]
/// Logs a message at the TRACE level.
///
/// # Examples
///
/// ```python
/// from d_merge_python import log_trace
///
/// log_trace('This is a trace message');
/// ```
pub fn log_trace(msg: String) {
    trace!("{msg}");
}

#[pyo3_stub_gen::derive::gen_stub_pyfunction]
#[pyo3::pyfunction]
/// Logs a message at the DEBUG level.
///
/// # Examples
///
/// ```python
/// from d_merge_python import log_debug
///
/// log_debug('This is a debug message');
/// ```
pub fn log_debug(msg: String) {
    debug!("{msg}");
}

#[pyo3_stub_gen::derive::gen_stub_pyfunction]
#[pyo3::pyfunction]
/// Logs a message at the INFO level.
///
/// # Examples
///
/// ```python
/// from d_merge_python import log_info
///
/// log_info('This is an info message');
/// ```
pub fn log_info(msg: String) {
    info!("{msg}");
}

#[pyo3_stub_gen::derive::gen_stub_pyfunction]
#[pyo3::pyfunction]
/// Logs a message at the WARN level.
///
/// # Examples
///
/// ```python
/// from d_merge_python import log_warn
///
/// log_warn('This is a warning message');
/// ```
pub fn log_warn(msg: String) {
    warn!("{msg}");
}

#[pyo3_stub_gen::derive::gen_stub_pyfunction]
#[pyo3::pyfunction]
/// Logs a message at the ERROR level.
///
/// # Examples
///
/// ```python
/// from d_merge_python import log_error
///
/// log_error('This is an error message');
/// ```
pub fn log_error(msg: String) {
    error!("{msg}");
}
