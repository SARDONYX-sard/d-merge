use nemesis_merge::{behavior_gen as behavior_gen_rs, global_logger::global_logger};
use pyo3::prelude::*;

use crate::{py_config::Config, status::wrap_status_callback};
use nemesis_merge::Config as RustConfig;

/// Generates behaviors by merging templates and patches asynchronously.
///
/// # Arguments
/// * `nemesis_paths` - List of input paths.
/// * `config` - Merge configuration.
/// * `status_report` - Optional Python callback for reporting progress.
#[pyfunction]
pub fn behavior_gen(
    py: Python,
    nemesis_paths: Vec<String>,
    mut config: Config,
    status_report: Option<Py<PyAny>>,
) -> PyResult<Bound<PyAny>> {
    let paths = nemesis_paths
        .iter()
        .map(std::path::PathBuf::from)
        .collect::<Vec<_>>();

    // Setup logger if needed
    if let Some(path) = config.log_path.take() {
        global_logger(path, config.log_level.take().unwrap_or_default())
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("{e}")))?;
    }

    let mut rust_config: RustConfig = config.into();

    // Attach status callback
    rust_config.status_report = wrap_status_callback(status_report);

    // Launch async behavior generation
    pyo3_async_runtimes::tokio::future_into_py(py, async move {
        behavior_gen_rs(paths, rust_config)
            .await
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("{e}")))?;
        Ok(Python::with_gil(|py| py.None()))
    })
}
