use std::sync::Arc;

use pyo3::prelude::*;
use rayon::prelude::*;
use serde_hkx_for_gui::status::{Payload as RustPayload, Status as RustStatus};
use serde_hkx_for_gui::DirEntry as RustDirEntry;

#[pyo3_stub_gen::derive::gen_stub_pyclass]
#[pyo3::pyclass]
/// Represents a node in the directory structure.
#[derive(Debug, Clone)]
pub struct DirEntry {
    /// Relative or absolute path
    #[pyo3(get)]
    pub id: String,
    /// The name of the entry (file or directory).
    #[pyo3(get)]
    pub label: String,
    /// The sub-entries contained within this directory, if applicable.
    /// This will be `None` if the entry is a file.
    #[pyo3(get)]
    pub children: Option<Vec<DirEntry>>,
}

impl From<RustDirEntry> for DirEntry {
    fn from(entry: RustDirEntry) -> Self {
        DirEntry {
            id: entry.path,
            label: entry.name,
            children: entry
                .children
                .map(|children| children.into_par_iter().map(Into::into).collect()),
        }
    }
}

#[pyo3_stub_gen::derive::gen_stub_pyfunction]
#[pyo3::pyfunction]
/// Loads a directory structure from the specified path, filtering by allowed extensions.
///
/// # Errors
/// Returns an error message if the directory cannot be loaded or if there are issues reading the path.
pub fn load_dir_node(dirs: Vec<String>) -> pyo3::PyResult<Vec<DirEntry>> {
    let dir_entries = serde_hkx_for_gui::load_dir_node(dirs).map_err(|errs| {
        let err = errs
            .par_iter()
            .map(|e| e.to_string())
            .collect::<Vec<String>>()
            .join("\n");
        pyo3::exceptions::PyRuntimeError::new_err(err)
    })?;

    Ok(dir_entries.into_par_iter().map(Into::into).collect())
}

#[pyo3_stub_gen::derive::gen_stub_pyfunction]
#[gen_stub(override_return_type(type_repr="typing.Awaitable[None]", imports=("typing")))]
#[pyo3::pyfunction]
/// Converts between HKX and XML (or other supported formats) asynchronously.
/// - `inputs`: input paths
/// - `output`: output path
/// - `format`: "amd64" | "win32" | "xml" | "json". otherwise error.
/// - `roots`: inputs multiple
/// - `progress`: status report function
///
/// # Errors
/// - Failed to convert.
/// - `FormatParse` - The provided output format string could not be parsed.
pub fn convert(
    py: Python,

    inputs: Vec<String>,
    output: Option<String>,
    format: String,
    roots: Option<Vec<String>>,
    #[gen_stub(override_type(type_repr="typing.Optional[collections.abc.Callable[[Payload], None]]", imports=("collections.abc", "typing")))]
    progress: Option<Py<PyAny>>,
) -> PyResult<Bound<PyAny>> {
    pyo3_async_runtimes::tokio::future_into_py(py, async move {
        let progress = progress.map(|cb| {
            let arc_cb = Arc::new(move |payload: RustPayload| {
                Python::attach(|py| {
                    if let Err(e) = cb.call1(py, (Payload::from(payload),)) {
                        e.print(py);
                    }
                });
            });

            // Return a closure that clones the Arc and calls it
            move |payload: RustPayload| {
                let arc_cb = arc_cb.clone();
                (arc_cb)(payload);
            }
        });

        match progress {
            Some(progress) => {
                serde_hkx_for_gui::convert(inputs, output, &format, roots, progress).await
            }
            None => serde_hkx_for_gui::convert(inputs, output, &format, roots, |_| {}).await,
        }
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    })
}

#[pyo3_stub_gen::derive::gen_stub_pyclass_enum]
#[pyo3::pyclass]
/// Represents the progress status of a conversion task.
///
/// The numeric representation (`u8`) is serialized and deserialized directly,
/// which is convenient for frontend communication.
#[derive(Debug, Clone)]
pub enum SerdeHkxStatus {
    /// Task is pending and has not started yet.: 0
    Pending,

    /// Task is currently being processed.: 1
    Processing,

    /// Task completed successfully.: 2
    Done,

    /// Task encountered an error during processing.: 3
    Error,
}

#[pyo3_stub_gen::derive::gen_stub_pyclass]
#[pyo3::pyclass]
/// Payload for progress reporting
#[derive(Debug, Clone)]
pub struct Payload {
    /// Hashed identifier of the file path.
    ///
    /// Using a hash ensures that the frontend can track tasks reliably,
    /// even if items are removed or reordered.
    ///
    /// - conversion input path to `djb2` hashed -> id
    #[pyo3(get, set)]
    pub path_id: u32,

    /// Current progress status of this task.
    #[pyo3(get, set)]
    pub status: SerdeHkxStatus,
}

// Rust Payload → JsPayload
impl From<RustPayload> for Payload {
    fn from(p: RustPayload) -> Self {
        Payload {
            path_id: p.path_id,
            status: match p.status {
                RustStatus::Pending => SerdeHkxStatus::Pending,
                RustStatus::Processing => SerdeHkxStatus::Processing,
                RustStatus::Done => SerdeHkxStatus::Done,
                RustStatus::Error => SerdeHkxStatus::Error,
            },
        }
    }
}
