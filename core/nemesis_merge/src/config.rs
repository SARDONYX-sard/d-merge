use std::{fmt, path::PathBuf};

/// A configuration structure used to specify various directories and a status report callback.
///
/// The `Config` struct holds paths for input resources and output directories, along with optional
/// settings for debugging and compatibility. It is used to control behavior during operations such as
/// patching HKX templates, merging JSON data, and generating final outputs.
#[derive(Default)]
pub struct Config {
    /// The directory containing the HKX templates you want to patch.
    ///
    /// Typically this is a directory like `assets/templates`. The actual patch target directory
    /// should be a subdirectory such as `assets/templates/meshes`.
    pub resource_dir: PathBuf,

    /// The directory where the output files will be saved.
    ///
    /// This directory will also contain `.debug` subdirectory if debug output is enabled.
    pub output_dir: PathBuf,

    /// Generation target
    pub output_target: OutPutTarget,

    /// An optional callback function that reports the current status of the process.
    ///
    /// The callback is invoked with `Status` updates, allowing consumers to track
    /// progress, errors, or other runtime events.
    pub status_report: Option<Box<dyn Fn(Status) + Send + Sync>>,

    /// Enables lenient parsing for known issues in unofficial or modded patches.
    ///
    /// This setting allows the parser to work around common community patch errors
    /// such as incorrect field names or missing values. Use with caution as it may
    /// mask actual data issues.
    pub hack_options: Option<HackOptions>,

    /// Options controlling the output of debug artifacts.
    pub debug: DebugOptions,
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

/// A collection of hack options that enable non-standard parsing behavior.
///
/// These options exist to handle cases where game mods or other tools produce
/// invalid or inconsistent data. Enabling these may allow parsing to succeed
/// in otherwise broken scenarios, at the risk of hiding real errors.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "ts_serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Copy, Clone, Default)]
pub struct HackOptions {
    /// Enables compatibility hacks for invalid fields in the `BSRagdollContactListenerModifier` class.
    ///
    /// This option activates targeted fixes for common field naming mistakes in patches:
    /// - Substitutes `event` with `contactEvent`
    /// - Substitutes `anotherBoneIndex` with `bones`
    pub cast_ragdoll_event: bool,
}

impl HackOptions {
    /// Enable all hack options.
    #[inline]
    pub const fn enable_all() -> Self {
        Self {
            cast_ragdoll_event: true,
        }
    }
}

impl From<HackOptions> for nemesis_xml::hack::HackOptions {
    #[inline]
    fn from(value: HackOptions) -> Self {
        Self {
            cast_ragdoll_event: value.cast_ragdoll_event,
        }
    }
}

/// A group of flags to enable debug output of intermediate files.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "ts_serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone)]
pub struct DebugOptions {
    /// If true, outputs the raw patch JSON to the `.debug` subdirectory under `output_dir`.
    ///
    /// This includes:
    /// - `patch.json`: The raw parsed patch data.
    ///   - For `One` patches, it reflects the result of priority-based overwriting.
    ///   - For `Seq` patches, all entries are preserved in a vector (`Vec`) for later conflict resolution.
    pub output_patch_json: bool,

    /// If true, outputs the merged JSON to the `.debug` subdirectory under `output_dir`.
    ///
    /// This represents the state of the data after all patches have been applied and
    /// conflicts resolved, but before converting to `.hkx` format.
    pub output_merged_json: bool,

    /// If true, outputs the intermediate merged XML to the `.debug` subdirectory under `output_dir`.
    ///
    /// This is the final XML representation of the patched and merged data,
    /// just before conversion to the binary `.hkx` format.
    pub output_merged_xml: bool,
}

impl Default for DebugOptions {
    #[inline]
    fn default() -> Self {
        Self {
            output_patch_json: true,
            output_merged_json: true,
            output_merged_xml: false,
        }
    }
}

impl DebugOptions {
    /// Enable all debug options.
    #[inline]
    pub const fn enable_all() -> Self {
        Self {
            output_patch_json: true,
            output_merged_json: true,
            output_merged_xml: true,
        }
    }
}

/// Output type
///
/// - feature = "ts_serde"
///
///  ```txt
///  SkyrimSE | SkyrimLE
///  ```
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, Default)]
pub enum OutPutTarget {
    /// Amd64
    #[default]
    #[cfg_attr(feature = "serde", serde(rename = "SkyrimSE"))]
    SkyrimSe,

    /// Win32
    #[cfg_attr(feature = "serde", serde(rename = "SkyrimLE"))]
    SkyrimLe,
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
