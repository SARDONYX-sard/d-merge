use std::{fmt, path::PathBuf};

/// A configuration structure used to specify various directories and a status report callback.
///
/// The `Config` struct holds paths for resource and output directories, as well as an optional
/// callback for reporting the current status of a process. This can be used to track and
/// report status updates during an operation, such as applying patches or generating files.
#[derive(Default)]
pub struct Config {
    /// The directory containing the hkx templates you want to patch
    pub resource_dir: PathBuf,

    /// The directory where the output files will be saved.
    pub output_dir: PathBuf,

    /// An optional callback function that reports the current status of the process.
    ///
    /// This closure takes a `Status` enum and is invoked to report status updates.
    pub status_report: Option<Box<dyn Fn(Status) + Send + Sync>>,
}

impl Config {
    /// Creates a new `Config` instance with the specified resource directory, output directory,
    /// and an optional status reporting closure.
    pub fn new(
        resource_dir: PathBuf,
        output_dir: PathBuf,
        status_report: Option<Box<dyn Fn(Status) + Send + Sync>>,
    ) -> Self {
        Self {
            resource_dir,
            output_dir,
            status_report,
        }
    }

    /// Calls the status reporting closure with the provided status.
    ///
    /// This method allows us to easily invoke the status callback if it's provided.
    #[inline]
    pub fn report_status(&self, status: Status) {
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
}
