use std::{fmt, path::PathBuf};

use nemesis_xml::hack::HackOptions;

/// A configuration structure used to specify various directories and a status report callback.
///
/// The `Config` struct holds paths for resource and output directories, as well as an optional
/// callback for reporting the current status of a process. This can be used to track and
/// report status updates during an operation, such as applying patches or generating files.
#[derive(Default)]
pub struct Config {
    /// The directory containing the hkx templates you want to patch
    ///
    /// Select the directory where mesh is located(e.g. `assets/templates`, then use `assets/templates/meshes`)
    pub resource_dir: PathBuf,

    /// The directory where the output files will be saved.
    pub output_dir: PathBuf,

    /// An optional callback function that reports the current status of the process.
    ///
    /// This closure takes a `Status` enum and is invoked to report status updates.
    pub status_report: Option<Box<dyn Fn(Status) + Send + Sync>>,

    /// Enables lenient parsing for known issues in unofficial or modded patches.
    ///
    /// This may fix common mistakes in community patches (e.g., misnamed fields),
    /// but can also hide real data errors.
    pub hack_options: Option<HackOptions>,
}

impl Config {
    /// Calls the status reporting closure with the provided status.
    ///
    /// This method allows us to easily invoke the status callback if it's provided.
    #[inline]
    pub fn on_report_status(&self, status: Status) {
        if let Some(f) = &self.status_report {
            f(status);
        }
    }
}

// Implements `Debug` for the `Config` struct, omitting the closure field as it cannot be debugged.
//
// This is useful for logging or debugging purposes, although the closure cannot be displayed.
//
// The `resource_dir` and `output_dir` fields are displayed, but the `status_report` closure
// is omitted.
impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Options")
            .field("resource_dir", &self.resource_dir)
            .field("output_dir", &self.output_dir)
            // Skip closure
            .finish()
    }
}

/// An enum representing various statuses during a process.
///
/// This enum is used to track and report the current state of an ongoing process, such as
/// reading templates, applying patches, generating files, or completing the task.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "ts_serde", serde(tag = "type", content = "message"))]
#[derive(Debug, Clone)]
pub enum Status {
    /// Status when reading templates and patches.
    ReadingTemplatesAndPatches,

    /// Status when applying patches.
    ApplyingPatches,

    /// Status when generating HKX files.
    GenerateHkxFiles,

    /// Status when the process is completed.
    Done,

    Error(String),
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReadingTemplatesAndPatches => write!(f, "[1/4] Reading templates and patches..."),
            Self::ApplyingPatches => write!(f, "[2/4] Applying patches..."),
            Self::GenerateHkxFiles => write!(f, "[3/4] Generating HKX files..."),
            Self::Done => write!(f, "[4/4] Done."),
            Self::Error(msg) => write!(f, "[Error] {msg}"),
        }
    }
}

#[cfg(test)]
pub(crate) fn new_color_status_reporter() -> Box<dyn Fn(Status) + Send + Sync> {
    Box::new(|status| {
        match &status {
            Status::ReadingTemplatesAndPatches => {
                println!("\x1b[36m{status}\x1b[0m"); // Cyan
            }
            Status::ApplyingPatches => {
                println!("\x1b[33m{status}\x1b[0m"); // Yellow
            }
            Status::GenerateHkxFiles => {
                println!("\x1b[34m{status}\x1b[0m"); // Blue
            }
            Status::Done => {
                println!("\x1b[32;1m{status}\x1b[0m"); // Bold Green
            }
            Status::Error(_) => {
                println!("\x1b[31;1m{status}\x1b[0m"); // Bold Red
            }
        }
    })
}

#[cfg(feature = "ts_serde")]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_status() {
        let status = Status::Error("Error message".to_string());
        let serialized = simd_json::to_string(&status).unwrap();
        assert_eq!(serialized, r#"{"type":"Error","message":"Error message"}"#);
    }
}
