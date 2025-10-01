use nemesis_merge::Status as RustStatus;
use pyo3::prelude::*;

/// A Python-compatible wrapper for the internal `Status` enum.
///
/// This enum mirrors the internal Rust `Status` used to report progress during behavior generation.
/// It is used for communicating status updates back to Python callbacks.
///
/// Do not construct this directly. It is automatically created from internal status values.
#[pyclass]
#[derive(Debug, Clone)]
pub enum Status {
    /// Status when generating FNIS patches.
    GeneratingFnisPatches {
        /// 0 based index
        index: usize,
        total: usize,
    },

    /// Indicates the system is reading patches and templates.
    ReadingPatches {
        /// The index of the currently processed file (0-based).
        index: usize,
        /// The total number of files to process.
        total: usize,
    },

    /// Indicates patches are being parsed.
    ParsingPatches { index: usize, total: usize },

    /// Indicates patches are being applied.
    ApplyingPatches { index: usize, total: usize },

    /// Indicates `.hkx` files are being generated.
    GeneratingHkxFiles { index: usize, total: usize },

    /// Indicates the behavior generation is complete.
    Done(),

    /// Indicates an error occurred during processing.
    Error(String),
}

impl From<RustStatus> for Status {
    #[inline]
    fn from(status: RustStatus) -> Self {
        match status {
            RustStatus::GeneratingFnisPatches { index, total } => {
                Status::GeneratingFnisPatches { index, total }
            }
            RustStatus::ReadingPatches { index, total } => Status::ReadingPatches { index, total },
            RustStatus::ParsingPatches { index, total } => Status::ParsingPatches { index, total },
            RustStatus::ApplyingPatches { index, total } => {
                Status::ApplyingPatches { index, total }
            }
            RustStatus::GeneratingHkxFiles { index, total } => {
                Status::GeneratingHkxFiles { index, total }
            }
            RustStatus::Done => Status::Done(),
            RustStatus::Error(msg) => Status::Error(msg),
        }
    }
}

impl core::fmt::Display for Status {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::GeneratingFnisPatches { index, total } => {
                write!(f, "[1/6] Generating FNIS patches...({index}/{total})")
            }
            Self::ReadingPatches { index, total } => {
                write!(f, "[2/6] Reading templates and patches...({index}/{total})")
            }
            Self::ParsingPatches { index, total } => {
                write!(f, "[3/6] Parsing patches...({index}/{total})")
            }
            Self::ApplyingPatches { index, total } => {
                write!(f, "[4/6] Applying patches...({index}/{total})")
            }
            Self::GeneratingHkxFiles { index, total } => {
                write!(f, "[5/6] Generating .hkx files...({index}/{total})")
            }
            Self::Done() => write!(f, "[6/6] Done."),
            Self::Error(msg) => write!(f, "[Error] {msg}"),
        }
    }
}

#[pymethods]
impl Status {
    /// Returns a human-readable string representation of the status.
    fn __str__(&self) -> String {
        format!("{self}")
    }

    /// Returns a developer-friendly string representation of the status.
    fn __repr__(&self) -> String {
        format!("{self:?}")
    }
}

/// Wraps a Python callback into a Rust-compatible status reporter.
///
/// Converts a Python callable that accepts a `StatusWrapper` into a
/// `Box<dyn Fn(Status)>` that can be used in Rust async tasks.
pub fn wrap_status_callback(
    callback: Option<Py<PyAny>>,
) -> Option<Box<dyn Fn(RustStatus) + Send + Sync + 'static>> {
    callback.map(|cb| {
        Box::new(move |status: RustStatus| {
            let wrapper = Status::from(status);
            Python::with_gil(|py| {
                if let Err(e) = cb.call1(py, (wrapper,)) {
                    e.print(py);
                }
            });
        }) as Box<dyn Fn(RustStatus) + Send + Sync + 'static>
    })
}
