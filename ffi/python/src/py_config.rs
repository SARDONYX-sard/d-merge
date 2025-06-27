use nemesis_merge::{
    Config as RustConfig, DebugOptions, HackOptions, OutPutTarget as RustOutPutTarget,
};
use pyo3::prelude::*;

/// Specifies the target platform for output generation.
///
/// This determines the format of the generated `.hkx` or other behavior data files,
/// depending on the version of Skyrim being used.
///
/// - `SkyrimSe`: Targets 64-bit Skyrim Special Edition.
/// - `SkyrimLe`: Targets 32-bit Skyrim Legendary Edition.
#[pyclass]
#[derive(Clone, Copy, Debug)]
pub enum OutPutTarget {
    /// Skyrim Special Edition (64-bit, also known as Skyrim SE).
    ///
    /// Use this for output compatible with modern 64-bit versions of the game.
    SkyrimSe,

    /// Skyrim Legendary Edition (32-bit, also known as Skyrim LE).
    ///
    /// Use this for compatibility with older, 32-bit versions of the game.
    SkyrimLe,
}

impl From<OutPutTarget> for RustOutPutTarget {
    #[inline]
    fn from(value: OutPutTarget) -> Self {
        match value {
            OutPutTarget::SkyrimSe => RustOutPutTarget::SkyrimSe,
            OutPutTarget::SkyrimLe => RustOutPutTarget::SkyrimLe,
        }
    }
}

/// Specifies the logging level used by the tracing system.
///
/// This enum maps to `tracing::Level` and is used to control
/// the verbosity of logs during execution.
#[pyclass]
#[derive(Debug, Clone, Copy, Default)]
pub enum LogLevel {
    /// Extremely detailed logs for tracing fine-grained behavior.
    Trace,
    /// Useful for debugging internal state changes.
    Debug,
    /// Default level, shows general operational information.
    Info,
    /// Indicates potential issues or abnormal conditions.
    Warn,
    /// Critical problems that usually require immediate attention.
    #[default]
    Error,
}

impl From<LogLevel> for tracing::Level {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Trace => tracing::Level::TRACE,
            LogLevel::Debug => tracing::Level::DEBUG,
            LogLevel::Info => tracing::Level::INFO,
            LogLevel::Warn => tracing::Level::WARN,
            LogLevel::Error => tracing::Level::ERROR,
        }
    }
}

/// Configuration object for controlling merge behavior and output options.
///
/// This class is passed from Python to Rust to configure the behavior
/// generation and patching pipeline. It includes paths, debug settings,
/// output format target, and optional logging controls.
#[pyclass]
#[derive(Debug, Clone)]
pub struct Config {
    /// The directory containing HKX template resources.
    ///
    /// Typically this points to something like `assets/templates`.
    #[pyo3(get, set)]
    pub resource_dir: String,

    /// The directory where merged files will be written.
    ///
    /// This may include debug output if enabled.
    #[pyo3(get, set)]
    pub output_dir: String,

    /// Target output format for the merged behavior data.
    ///
    /// Determines if the output is formatted for Skyrim SE (64-bit) or LE (32-bit).
    #[pyo3(get, set)]
    pub output_target: OutPutTarget,

    /// Enables compatibility patches for ragdoll event fields.
    ///
    /// When true, substitutes invalid or legacy field names to support buggy or unofficial patches.
    #[pyo3(get, set)]
    pub cast_ragdoll_event: bool,

    /// Enables debug output for the raw patch JSON.
    ///
    /// Output is written to `.debug/patch.json` in the output directory.
    #[pyo3(get, set)]
    pub output_patch_json: bool,

    /// Enables debug output for the final merged JSON.
    ///
    /// Output is written to `.debug/merged.json` before binary export.
    #[pyo3(get, set)]
    pub output_merged_json: bool,

    /// Enables debug output for the merged XML.
    ///
    /// Output is written to `.debug/merged.xml`, representing the XML just before `.hkx` conversion.
    #[pyo3(get, set)]
    pub output_merged_xml: bool,

    /// Optional log file path for writing tracing logs.
    ///
    /// If this is set, logs will be written to the specified file.
    /// If not set, logs will be printed to stderr.
    #[pyo3(get, set)]
    pub log_path: Option<String>,

    /// Optional log level for controlling the verbosity of tracing output.
    ///
    /// Defaults to `Info` if not provided.
    #[pyo3(get, set)]
    pub log_level: Option<LogLevel>,
}

#[pymethods]
impl Config {
    /// Create a new Config object.
    ///
    /// All fields must be provided.
    #[new]
    #[allow(clippy::too_many_arguments)]
    fn new(
        resource_dir: String,
        output_dir: String,
        output_target: OutPutTarget,
        cast_ragdoll_event: bool,
        output_patch_json: bool,
        output_merged_json: bool,
        output_merged_xml: bool,
        log_path: Option<String>,
        log_level: Option<LogLevel>,
    ) -> Self {
        Config {
            resource_dir,
            output_dir,
            output_target,
            cast_ragdoll_event,
            output_patch_json,
            output_merged_json,
            output_merged_xml,
            log_path,
            log_level,
        }
    }
}

impl From<Config> for RustConfig {
    fn from(value: Config) -> Self {
        let output_target = RustOutPutTarget::from(value.output_target);

        RustConfig {
            resource_dir: value.resource_dir.into(),
            output_dir: value.output_dir.into(),
            output_target,
            status_report: None,
            hack_options: if value.cast_ragdoll_event {
                Some(HackOptions::enable_all())
            } else {
                None
            },
            debug: DebugOptions {
                output_patch_json: value.output_patch_json,
                output_merged_json: value.output_merged_json,
                output_merged_xml: value.output_merged_xml,
            },
        }
    }
}
