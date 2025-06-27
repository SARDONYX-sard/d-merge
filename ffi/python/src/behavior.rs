use nemesis_merge::behavior_gen;
use pyo3::prelude::*;

use crate::py_config::PyConfig;

#[pyfunction]
pub fn behavior_gen_py(
    py: Python,
    nemesis_paths: Vec<String>,
    py_config: PyConfig,
) -> PyResult<Bound<PyAny>> {
    let paths = nemesis_paths
        .iter()
        .map(std::path::PathBuf::from)
        .collect::<Vec<_>>();
    let config = py_config.into();

    pyo3_async_runtimes::tokio::future_into_py(py, async move {
        behavior_gen(paths, config)
            .await
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("{e}")))?;
        Ok(Python::with_gil(|py| py.None()))
    })
}
