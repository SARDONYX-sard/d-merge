use std::{collections::HashMap, path::PathBuf};

use mod_info::{ModInfo as RustModInfo, ModType as RustModType};
use nemesis_merge::{
    Config as RustConfig, DebugOptions as RustDebugOptions, HackOptions as RustHackOptions,
    OutPutTarget as RustOutPutTarget, PatchMaps as RustPatchMaps, Status as RustStatus,
};
use pyo3::prelude::*;
use rayon::prelude::*;
use skyrim_data_dir::Runtime;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Config wrappers
////////////////////////////////////////////////////////////////////////////////////////////////////

#[pyo3_stub_gen::derive::gen_stub_pyclass]
#[pyo3::pyclass]
/// A collection of hack options that enable non-standard parsing behavior.
///
/// These options exist to handle cases where game mods or other tools produce
/// invalid or inconsistent data. Enabling these may allow parsing to succeed
/// in otherwise broken scenarios, at the risk of hiding real errors.
#[derive(Debug, Clone)]
pub struct HackOptions {
    /// Enables compatibility hacks for invalid fields in the `BSRagdollContactListenerModifier` class.
    ///
    /// This option activates targeted fixes for common field naming mistakes in patches:
    /// - Substitutes `event` with `contactEvent`
    /// - Substitutes `anotherBoneIndex` with `bones`
    #[pyo3(get, set)]
    pub cast_ragdoll_event: bool,
}

#[pyo3_stub_gen::derive::gen_stub_pyclass]
#[pyo3::pyclass]
/// A group of flags to enable debug output of intermediate files.
#[derive(Debug, Clone, Default)]
pub struct DebugOptions {
    /// If true, outputs the raw patch JSON to the `.debug` subdirectory under `<output_dir>/.d_merge`.
    ///
    /// This includes:
    /// - `patch.json`: The raw parsed patch data.
    ///   - For `One` patches, it reflects the result of priority-based overwriting.
    ///   - For `Seq` patches, all entries are preserved in a vector (`Vec`) for later conflict resolution.
    #[pyo3(get, set)]
    pub output_patch_json: bool,

    /// If true, outputs the merged JSON to the `.debug` subdirectory under `<output_dir>/.d_merge`.
    ///
    /// This represents the state of the data after all patches have been applied and
    /// conflicts resolved, but before converting to `.hkx` format.
    #[pyo3(get, set)]
    pub output_merged_json: bool,

    /// If true, outputs the intermediate merged XML to the `.debug` subdirectory under `<output_dir>/.d_merge`.
    ///
    /// This is the final XML representation of the patched and merged data,
    /// just before conversion to the binary `.hkx` format.
    #[pyo3(get, set)]
    pub output_merged_xml: bool,
}

#[pyo3_stub_gen::derive::gen_stub_pyclass_enum]
#[pyo3::pyclass]
/// Behavior Output target
#[derive(Debug, Clone, Copy, Default)]
pub enum OutPutTarget {
    /// Skyrim Legendary Edition
    #[default]
    SkyrimSE,
    /// Skyrim Special Edition
    SkyrimLE,
}

#[pyo3_stub_gen::derive::gen_stub_pyclass]
#[pyo3::pyclass]
/// A configuration structure used to specify various directories and a status report callback.
///
/// The `Config` struct holds paths for input resources and output directories, along with optional
/// settings for debugging and compatibility. It is used to control behavior during operations such as
/// patching HKX templates, merging JSON data, and generating final outputs.
#[derive(Debug, Clone, Default)]
pub struct Config {
    /// The directory containing the HKX templates you want to patch.
    ///
    /// Typically this is a directory like `assets/templates`. The actual patch target directory
    /// should be a subdirectory such as `assets/templates/meshes`.
    #[pyo3(get, set)]
    pub resource_dir: String,

    /// The directory where the output files will be saved.
    ///
    /// This directory will also contain `.debug` subdirectory if debug output is enabled.
    #[pyo3(get, set)]
    pub output_dir: String,

    /// Generation target
    #[pyo3(get, set)]
    pub output_target: OutPutTarget,

    /// Enables lenient parsing for known issues in unofficial or modded patches.
    ///
    /// This setting allows the parser to work around common community patch errors
    /// such as incorrect field names or missing values. Use with caution as it may
    /// mask actual data issues.
    #[pyo3(get, set)]
    pub hack_options: Option<HackOptions>,

    /// Options controlling the output of debug artifacts.
    #[pyo3(get, set)]
    pub debug: DebugOptions,

    /// Skyrim data directories glob (required **only when using FNIS**).
    ///
    /// This must include all directories containing `animations/<namespace>`, otherwise FNIS
    /// entries will not be detected and the process will fail.
    #[pyo3(get, set)]
    pub skyrim_data_dir_glob: Option<String>,
}

#[pyo3_stub_gen::derive::gen_stub_pymethods]
#[pymethods]
impl Config {
    /// Create a new Config class.
    #[new]
    #[allow(clippy::too_many_arguments)]
    fn new(
        resource_dir: String,
        output_dir: String,
        output_target: OutPutTarget,
        cast_ragdoll_event: Option<bool>,
        output_patch_json: bool,
        output_merged_json: bool,
        output_merged_xml: bool,
        skyrim_data_dir_glob: Option<String>,
    ) -> Self {
        Config {
            resource_dir,
            output_dir,
            output_target,
            hack_options: cast_ragdoll_event
                .map(|cast_ragdoll_event| HackOptions { cast_ragdoll_event }),
            debug: DebugOptions {
                output_patch_json,
                output_merged_json,
                output_merged_xml,
            },
            skyrim_data_dir_glob,
        }
    }
}

impl Config {
    pub fn try_into_rust(self, status_report: Option<Py<PyAny>>) -> PyResult<RustConfig> {
        // intended:
        // rust closure |status| {
        //     py_fn(status)
        // }
        let status_report = status_report.map(|cb| {
            Box::new(move |status: RustStatus| {
                Python::attach(|py| {
                    if let Err(e) = cb.call1(py, (PatchStatus::from(status),)) {
                        e.print(py);
                    }
                });
            }) as Box<dyn Fn(RustStatus) + Send + Sync + 'static>
        });

        Ok(RustConfig {
            resource_dir: PathBuf::from(self.resource_dir),
            output_dir: PathBuf::from(self.output_dir),
            output_target: match self.output_target {
                OutPutTarget::SkyrimSE => RustOutPutTarget::SkyrimSe,
                OutPutTarget::SkyrimLE => RustOutPutTarget::SkyrimLe,
            },
            hack_options: self.hack_options.map(|h| RustHackOptions {
                cast_ragdoll_event: h.cast_ragdoll_event,
            }),
            debug: RustDebugOptions {
                output_patch_json: self.debug.output_patch_json,
                output_merged_json: self.debug.output_merged_json,
                output_merged_xml: self.debug.output_merged_xml,
            },
            status_report,
            skyrim_data_dir_glob: self.skyrim_data_dir_glob,
        })
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Status wrapper
////////////////////////////////////////////////////////////////////////////////////////////////////

#[pyo3_stub_gen::derive::gen_stub_pyclass_complex_enum]
#[pyo3::pyclass]
/// Represents the status of a patching process.
///
/// This struct is used to report the current state of the process. Depending on the `event`,
/// `content` may contain progress information, and `err_msg` may contain an error message if the process failed.
///
/// The backend emits these status values using `window.emit(...)` during various stages.
/// - Mirrors Rust enum with serde(tag="type", content="content").
#[derive(Debug, Clone)]
pub enum PatchStatus {
    /// Status when generating FNIS patches.
    GeneratingFnisPatches {
        /// 0 based index
        index: u32,
        total: u32,
    },

    /// Status when reading patches.
    ReadingPatches {
        /// 0 based index
        index: u32,
        total: u32,
    },

    /// Status when Parsing patches.
    ParsingPatches {
        /// 0 based index
        index: u32,
        total: u32,
    },

    /// Status when applying patches.
    ApplyingPatches {
        /// 0 based index
        index: u32,
        total: u32,
    },

    /// Status when generating HKX files.
    GeneratingHkxFiles {
        /// 0 based index
        index: u32,
        total: u32,
    },

    /// Status when the process is completed.
    Done(),

    /// Error occurred, then err msg
    Error(String),
}

#[pyo3_stub_gen::derive::gen_stub_pymethods]
#[pyo3::pymethods]
impl PatchStatus {
    /// Returns a human-readable string representation of the status.
    fn __str__(&self) -> String {
        format!("{}", RustStatus::from(self))
    }

    /// Returns a developer-friendly string representation of the status.
    fn __repr__(&self) -> String {
        format!("{self:?}")
    }
}

impl From<&PatchStatus> for RustStatus {
    fn from(s: &PatchStatus) -> Self {
        match s.clone() {
            PatchStatus::GeneratingFnisPatches { index, total } => {
                RustStatus::GeneratingFnisPatches {
                    index: index as usize,
                    total: total as usize,
                }
            }
            PatchStatus::ReadingPatches { index, total } => RustStatus::ReadingPatches {
                index: index as usize,
                total: total as usize,
            },
            PatchStatus::ParsingPatches { index, total } => RustStatus::ParsingPatches {
                index: index as usize,
                total: total as usize,
            },
            PatchStatus::ApplyingPatches { index, total } => RustStatus::ApplyingPatches {
                index: index as usize,
                total: total as usize,
            },
            PatchStatus::GeneratingHkxFiles { index, total } => RustStatus::GeneratingHkxFiles {
                index: index as usize,
                total: total as usize,
            },
            PatchStatus::Done() => RustStatus::Done,
            PatchStatus::Error(message) => RustStatus::Error(message),
        }
    }
}
impl From<RustStatus> for PatchStatus {
    fn from(s: RustStatus) -> Self {
        match s {
            RustStatus::GeneratingFnisPatches { index, total } => {
                PatchStatus::GeneratingFnisPatches {
                    index: index as u32,
                    total: total as u32,
                }
            }
            RustStatus::ReadingPatches { index, total } => PatchStatus::ReadingPatches {
                index: index as u32,
                total: total as u32,
            },
            RustStatus::ParsingPatches { index, total } => PatchStatus::ParsingPatches {
                index: index as u32,
                total: total as u32,
            },
            RustStatus::ApplyingPatches { index, total } => PatchStatus::ApplyingPatches {
                index: index as u32,
                total: total as u32,
            },
            RustStatus::GeneratingHkxFiles { index, total } => PatchStatus::GeneratingHkxFiles {
                index: index as u32,
                total: total as u32,
            },
            RustStatus::Done => PatchStatus::Done(),
            RustStatus::Error(message) => PatchStatus::Error(message),
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ModInfo wrapper
////////////////////////////////////////////////////////////////////////////////////////////////////

#[pyo3_stub_gen::derive::gen_stub_pyclass]
#[pyo3::pyclass]
/// # Note
/// - Intended `Nemesis_Engine/mods/<id>/info.ini`
/// - `priority`: As with MO2, lower numbers indicate lower priority, higher numbers indicate higher priority.
#[derive(Debug, Clone)]
pub struct ModInfo {
    /// Mod-specific dir name.
    #[pyo3(get)]
    pub id: String,
    /// Mod name
    #[pyo3(get)]
    pub name: String,
    /// Mod author
    #[pyo3(get)]
    pub author: String,
    /// Mod download link
    #[pyo3(get)]
    pub site: String,
    /// Mod type. Nemesis, FNIS
    #[pyo3(get)]
    pub mod_type: ModType,
}

#[pyo3_stub_gen::derive::gen_stub_pyclass_enum]
#[pyo3::pyclass]
/// Mod type. Nemesis, FNIS
#[derive(Debug, Clone, Copy)]
pub enum ModType {
    /// GUI developers must add the following to the paths array in `nemesis_merge::behavior_gen`.
    /// - `<skyrim data dir>/Nemesis_Engine/mod/aaaa`
    Nemesis,
    /// GUI developers must add the following to the paths array in `nemesis_merge::behavior_gen`.
    /// - `<skyrim data dir>/meshes/actors/character/animations/<namespace>`
    Fnis,
}

impl From<RustModType> for ModType {
    fn from(value: RustModType) -> Self {
        match value {
            RustModType::Nemesis => ModType::Nemesis,
            RustModType::Fnis => ModType::Fnis,
        }
    }
}

impl From<RustModInfo> for ModInfo {
    fn from(m: RustModInfo) -> Self {
        ModInfo {
            id: m.id,
            name: m.name,
            author: m.author,
            site: m.site,
            mod_type: ModType::from(m.mod_type),
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Exported APIs
////////////////////////////////////////////////////////////////////////////////////////////////////

#[pyo3_stub_gen::derive::gen_stub_pyclass]
#[pyo3::pyclass]
/// Mod entries
#[derive(Debug, Clone, Default)]
pub struct PatchMaps {
    /// Nemesis patch path
    /// - key: path until mod_code(e.g. `<skyrim_data_dir>/meshes/Nemesis_Engine/mod/slide`)
    /// - value: priority
    #[pyo3(get, set)]
    pub nemesis_entries: HashMap<String, usize>,
    /// FNIS patch path
    /// - key: FNIS namespace(e.g. `namespace` of `<skyrim_data_dir>/path/meshes/actors/character/animations/<namespace>`)
    /// - value: priority
    #[pyo3(get, set)]
    pub fnis_entries: HashMap<String, usize>,
}

#[pyo3_stub_gen::derive::gen_stub_pymethods]
#[pymethods]
impl PatchMaps {
    /// Create a new class.
    #[new]
    fn new() -> Self {
        PatchMaps::default()
    }
}

#[inline]
fn into_rust_priority_map(map: PatchMaps) -> RustPatchMaps {
    RustPatchMaps {
        nemesis_entries: map.nemesis_entries,
        fnis_entries: map.fnis_entries,
    }
}

#[pyo3_stub_gen::derive::gen_stub_pyfunction]
#[gen_stub(override_return_type(type_repr="typing.Awaitable[None]", imports=("typing")))]
#[pyo3::pyfunction]
/// Generates Nemesis behaviors for given mod IDs using a configuration.
///
/// - nemesis_paths: `e.g. ["../../dummy/Data/Nemesis_Engine/mod/aaaaa"]`
/// - `config.resource_dir`: Path of the template from which the patch was applied.(e.g. `../templates/` => `../templates/meshes`)
/// - `status_fn` - Optional thread_safe JS callback for patch status updates.
///
/// # Errors
/// Returns an error if file parsing, I/O operations, or JSON serialization fails.
pub fn behavior_gen<'py>(
    py: Python<'py>,
    patch_entries: PatchMaps,
    config: Config,
    #[gen_stub(override_type(type_repr="typing.Optional[collections.abc.Callable[[PatchStatus], None]]", imports=("collections.abc", "typing")))]
    status_fn: Option<Py<PyAny>>,
) -> PyResult<Bound<'py, PyAny>> {
    pyo3::Python::initialize();

    let patches = into_rust_priority_map(patch_entries);
    let config: RustConfig = config.try_into_rust(status_fn)?;

    pyo3_async_runtimes::tokio::future_into_py(py, async move {
        nemesis_merge::behavior_gen(patches, config)
            .await
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("{e}")))?;

        Ok(())
    })
}

#[pyo3_stub_gen::derive::gen_stub_pyfunction]
#[pyo3::pyfunction]
/// Get the skyrim data directory.
///
/// # Errors
/// - When the string specified in runtime is not “SkyrimSE” or “SkyrimLE”
/// - Returns an error if the Skyrim directory cannot be found from registry.
pub fn get_skyrim_data_dir(runtime: OutPutTarget) -> PyResult<String> {
    let runtime = match runtime {
        OutPutTarget::SkyrimSE => Runtime::Se,
        OutPutTarget::SkyrimLE => Runtime::Le,
    };

    skyrim_data_dir::get_skyrim_data_dir(runtime)
        .map(|p| p.display().to_string())
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
}

#[pyo3_stub_gen::derive::gen_stub_pyfunction]
#[pyo3::pyfunction]
/// Collect both Nemesis and FNIS mods into a single vector.
///
/// # Errors
/// Returns [`napi::Error`] if glob expansion fails or files cannot be read.
pub fn load_mods_info(glob: String, is_vfs: bool) -> pyo3::PyResult<Vec<ModInfo>> {
    let infos = mod_info::get_all(&glob, is_vfs)
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

    Ok(infos.into_par_iter().map(Into::into).collect())
}
