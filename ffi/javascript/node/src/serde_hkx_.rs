use std::sync::Arc;

use napi::threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode};
use napi_derive::napi;
use rayon::prelude::*;
use serde_hkx_for_gui::status::{Payload as RustPayload, Status as RustStatus};
use serde_hkx_for_gui::DirEntry as RustDirEntry;

/// Represents a node in the directory structure.
#[napi_derive::napi(object)]
pub struct DirEntry {
    /// Relative or absolute path
    pub id: String,
    /// The name of the entry (file or directory).
    pub label: String,
    /// The sub-entries contained within this directory, if applicable.
    /// This will be `None` if the entry is a file.
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

#[napi]
pub async fn load_dir_node(dirs: Vec<String>) -> napi::Result<Vec<DirEntry>> {
    let dir_entries = serde_hkx_for_gui::load_dir_node(dirs).map_err(|errs| {
        let err = errs
            .par_iter()
            .map(|e| e.to_string())
            .collect::<Vec<String>>()
            .join("\n");
        napi::Error::from_reason(err)
    })?;

    Ok(dir_entries.into_par_iter().map(Into::into).collect())
}

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
#[napi(
    ts_args_type = "inputs: string[], output: string, format: \"amd64\" | \"win32\" | \"xml\" | \"json\", roots: string[] | undefined, progress: (payload: Payload) => void"
)]
pub async fn convert(
    inputs: Vec<String>,
    output: Option<String>,
    format: String,
    roots: Option<Vec<String>>,
    progress: ThreadsafeFunction<Payload>,
) -> napi::Result<()> {
    let progress = Arc::new(progress);

    serde_hkx_for_gui::convert(inputs, output, &format, roots, move |payload| {
        let _ = progress.call(Ok(payload.into()), ThreadsafeFunctionCallMode::NonBlocking);
    })
    .await
    .map_err(|e| napi::Error::from_reason(e.to_string()))?;

    Ok(())
}

/// Represents the progress status of a conversion task.
///
/// The numeric representation (`u8`) is serialized and deserialized directly,
/// which is convenient for frontend communication.
#[napi]
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

/// Payload for progress reporting
#[napi(object)]
pub struct Payload {
    /// Hashed identifier of the file path.
    ///
    /// Using a hash ensures that the frontend can track tasks reliably,
    /// even if items are removed or reordered.
    ///
    /// - conversion input path to `djb2` hashed -> id
    pub path_id: u32,

    /// Current progress status of this task.
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
