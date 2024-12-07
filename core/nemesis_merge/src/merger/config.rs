use std::{fmt, path::PathBuf};

#[derive(Default)]
pub struct Config {
    pub resource_dir: PathBuf,
    pub output_dir: PathBuf,
    pub status_report: Option<Box<dyn Fn(Status) + Send + Sync>>,
}

impl Config {
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

    /// Wrapper function to easily execute closures.
    #[inline]
    pub fn report_status(&self, status: Status) {
        if let Some(f) = &self.status_report {
            f(status);
        }
    }
}

// Closure cannot be debugged directly, so it is omitted here
impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Options")
            .field("resource_dir", &self.resource_dir)
            .field("output_dir", &self.output_dir)
            // Skip closure
            .finish()
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub enum Status {
    ReadingTemplatesAndPatches,
    ApplyingPatches,
    GenerateHkxFiles,
    Done,
}
