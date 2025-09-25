use std::collections::HashMap;

use nemesis_merge::{
    behavior_gen as behavior_gen_rs, global_logger::global_logger, PatchMaps as RustPatchMaps,
};
use pyo3::prelude::*;

use crate::{py_config::Config, status::wrap_status_callback};
use nemesis_merge::Config as RustConfig;

#[pyclass]
#[derive(Debug, Clone, Default)]
pub struct PatchMaps {
    /// Nemesis patch path
    /// - key: path until mod_code(e.g. `<skyrim_data_dir>/meshes/Nemesis_Engine/mod/slide`)
    /// - value: priority
    pub nemesis_entries: PriorityMap,
    /// FNIS patch path
    /// - key: path until namespace(e.g. `<skyrim_data_dir>/path/Meshes/actors/character/animations/FNISFlyer`)
    /// - value: priority
    pub fnis_entries: PriorityMap,
}

#[pymethods]
impl PatchMaps {
    /// Create a new Config object.
    ///
    /// All fields must be provided.
    #[new]
    fn new(nemesis_entries: PriorityMap, fnis_entries: PriorityMap) -> Self {
        Self {
            nemesis_entries,
            fnis_entries,
        }
    }
}

// Node.js unsupported usize. So we use u32
type PriorityMap = HashMap<String, u32>;

#[inline]
fn into_rust_priority_map(map: PatchMaps) -> RustPatchMaps {
    RustPatchMaps {
        nemesis_entries: map
            .nemesis_entries
            .into_iter()
            .map(|(k, v)| (k, v as usize))
            .collect(),
        fnis_entries: map
            .fnis_entries
            .into_iter()
            .map(|(k, v)| (k, v as usize))
            .collect(),
    }
}

/// Generates behaviors by merging templates and patches asynchronously.
///
/// # Arguments
/// * `nemesis_paths` - List of input paths.
/// * `config` - Merge configuration.
/// * `status_report` - Optional Python callback for reporting progress.
#[pyfunction]
pub fn behavior_gen(
    py: Python,
    patches: PatchMaps,
    mut config: Config,
    status_report: Option<Py<PyAny>>,
) -> PyResult<Bound<PyAny>> {
    let patches = into_rust_priority_map(patches);

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
        behavior_gen_rs(patches, rust_config)
            .await
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("{e}")))?;
        Ok(Python::with_gil(|py| py.None()))
    })
}
