use std::{collections::HashMap, path::PathBuf};

use mod_info::{ModInfo as RustModInfo, ModType as RustModType};
use napi::threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode};
use napi_derive::napi;
use nemesis_merge::{
    Config as RustConfig, DebugOptions as RustDebugOptions, HackOptions as RustHackOptions,
    OutPutTarget as RustOutPutTarget, PatchMaps as RustPatchMaps, Status as RustStatus,
};
use rayon::prelude::*;
use skyrim_data_dir::Runtime;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Config wrappers
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A collection of hack options that enable non-standard parsing behavior.
///
/// These options exist to handle cases where game mods or other tools produce
/// invalid or inconsistent data. Enabling these may allow parsing to succeed
/// in otherwise broken scenarios, at the risk of hiding real errors.
#[napi(object)]
pub struct HackOptions {
    /// Enables compatibility hacks for invalid fields in the `BSRagdollContactListenerModifier` class.
    ///
    /// This option activates targeted fixes for common field naming mistakes in patches:
    /// - Substitutes `event` with `contactEvent`
    /// - Substitutes `anotherBoneIndex` with `bones`
    pub cast_ragdoll_event: bool,
}

/// A group of flags to enable debug output of intermediate files.
#[napi(object)]
pub struct DebugOptions {
    /// If true, outputs the raw patch JSON to the `.debug` subdirectory under `<output_dir>/.d_merge`.
    ///
    /// This includes:
    /// - `patch.json`: The raw parsed patch data.
    ///   - For `One` patches, it reflects the result of priority-based overwriting.
    ///   - For `Seq` patches, all entries are preserved in a vector (`Vec`) for later conflict resolution.
    pub output_patch_json: bool,

    /// If true, outputs the merged JSON to the `.debug` subdirectory under `<output_dir>/.d_merge`.
    ///
    /// This represents the state of the data after all patches have been applied and
    /// conflicts resolved, but before converting to `.hkx` format.
    pub output_merged_json: bool,

    /// If true, outputs the intermediate merged XML to the `.debug` subdirectory under `<output_dir>/.d_merge`.
    ///
    /// This is the final XML representation of the patched and merged data,
    /// just before conversion to the binary `.hkx` format.
    pub output_merged_xml: bool,
}

/// Behavior Output target
#[napi(string_enum)]
pub enum OutPutTarget {
    /// Skyrim Legendary Edition
    SkyrimSE,
    /// Skyrim Special Edition
    SkyrimLE,
}

/// A configuration structure used to specify various directories and a status report callback.
///
/// The `Config` struct holds paths for input resources and output directories, along with optional
/// settings for debugging and compatibility. It is used to control behavior during operations such as
/// patching HKX templates, merging JSON data, and generating final outputs.
#[napi(object)]
pub struct Config {
    /// The directory containing the HKX templates you want to patch.
    ///
    /// Typically this is a directory like `assets/templates`. The actual patch target directory
    /// should be a subdirectory such as `assets/templates/meshes`.
    pub resource_dir: String,

    /// The directory where the output files will be saved.
    ///
    /// This directory will also contain `.debug` subdirectory if debug output is enabled.
    pub output_dir: String,

    /// Generation target
    pub output_target: OutPutTarget,

    /// Enables lenient parsing for known issues in unofficial or modded patches.
    ///
    /// This setting allows the parser to work around common community patch errors
    /// such as incorrect field names or missing values. Use with caution as it may
    /// mask actual data issues.
    pub hack_options: Option<HackOptions>,

    /// Options controlling the output of debug artifacts.
    pub debug: DebugOptions,

    /// Skyrim data directories glob (required **only when using FNIS**).
    ///
    /// This must include all directories containing `animations/<namespace>`, otherwise FNIS
    /// entries will not be detected and the process will fail.
    pub skyrim_data_dir_glob: Option<String>,
}

impl Config {
    pub fn try_into_rust(
        self,
        status_report: Option<ThreadsafeFunction<PatchStatus>>,
    ) -> napi::Result<RustConfig> {
        // intended:
        // rust closure |status| {
        //     js_fn(status)
        // }
        let status_report = status_report.map(|f| {
            Box::new(move |rust_status: RustStatus| {
                let js_status: PatchStatus = rust_status.into();
                let _ = f.call(Ok(js_status), ThreadsafeFunctionCallMode::NonBlocking);
            }) as Box<dyn Fn(RustStatus) + Send + Sync>
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

/// Represents the progress of a task, including index and total count.
#[napi(object)]
pub struct ProgressContent {
    /// Current index of the progress.
    pub index: u32,
    /// Total count for the progress.
    pub total: u32,
}

/// Represents the status of a patching process.
///
/// This struct is used to report the current state of the process. Depending on the `event`,
/// `content` may contain progress information, and `err_msg` may contain an error message if the process failed.
///
/// The backend emits these status values using `window.emit(...)` during various stages.
/// - Mirrors Rust enum with serde(tag="type", content="content").
#[napi]
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
    Done,

    /// Error occurred, then err msg
    Error(String),
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
            RustStatus::Done => PatchStatus::Done,
            RustStatus::Error(message) => PatchStatus::Error(message),
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ModInfo wrapper
////////////////////////////////////////////////////////////////////////////////////////////////////

/// # Note
/// - Intended `Nemesis_Engine/mods/<id>/info.ini`
/// - `priority`: As with MO2, lower numbers indicate lower priority, higher numbers indicate higher priority.
#[napi(object)]
pub struct ModInfo {
    /// Mod-specific dir name.
    pub id: String,
    /// Mod name
    pub name: String,
    /// Mod author
    pub author: String,
    /// Mod download link
    pub site: String,
    /// Mod type. Nemesis, FNIS
    pub mod_type: ModType,
}

/// Mod type. Nemesis, FNIS
#[napi(string_enum)]
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

/// Mod entries
#[napi(object)]
#[derive(Debug, Default)]
pub struct PatchMaps {
    /// Nemesis patch path
    /// - key: path until mod_code(e.g. `<skyrim_data_dir>/meshes/Nemesis_Engine/mod/slide`)
    /// - value: priority
    pub nemesis_entries: HashMap<String, u32>, // Node.js unsupported usize. So we use u32
    /// FNIS patch path
    /// - key: FNIS namespace(e.g. `namespace` of `<skyrim_data_dir>/path/meshes/actors/character/animations/<namespace>`)
    /// - value: priority
    pub fnis_entries: HashMap<String, u32>,
}

#[inline]
fn into_rust_priority_map(map: PatchMaps) -> RustPatchMaps {
    RustPatchMaps {
        nemesis_entries: map
            .nemesis_entries
            .into_par_iter()
            .map(|(k, v)| (k, v as usize))
            .collect(),
        fnis_entries: map
            .fnis_entries
            .into_par_iter()
            .map(|(k, v)| (k, v as usize))
            .collect(),
    }
}

/// Generates Nemesis behaviors for given mod IDs using a configuration.
///
/// - nemesis_paths: `e.g. ["../../dummy/Data/Nemesis_Engine/mod/aaaaa"]`
/// - `config.resource_dir`: Path of the template from which the patch was applied.(e.g. `../templates/` => `../templates/meshes`)
/// - `status_fn` - Optional thread_safe JS callback for patch status updates.
///
/// # Errors
/// Returns an error if file parsing, I/O operations, or JSON serialization fails.
#[napi(
    ts_args_type = "nemesis_paths: PatchMaps, config: Config, status_fn?: (err: Error | null, status: PatchStatus) => void"
)]
pub async fn behavior_gen(
    patch_entries: PatchMaps,
    config: Config,
    status_fn: Option<ThreadsafeFunction<PatchStatus>>,
) -> napi::Result<()> {
    let patches = into_rust_priority_map(patch_entries);
    let config: RustConfig = config.try_into_rust(status_fn)?;

    nemesis_merge::behavior_gen(patches, config)
        .await
        .map_err(|e| napi::Error::from_reason(e.to_string()))?;

    Ok(())
}

/// Get the skyrim data directory.
///
/// # Errors
/// - When the string specified in runtime is not “SkyrimSE” or “SkyrimLE”
/// - Returns an error if the Skyrim directory cannot be found from registry.
#[napi(ts_args_type = "runtime: \"SkyrimSE\" | \"SkyrimLE\"")]
pub fn get_skyrim_data_dir(runtime: String) -> napi::Result<String> {
    let runtime = match runtime.as_str() {
        "SkyrimSE" => Runtime::Se,
        "SkyrimLE" => Runtime::Le,
        _ => {
            return Err(napi::Error::from_reason(
                "Invalid runtime (must be SkyrimSE or SkyrimLE)",
            ))
        }
    };

    skyrim_data_dir::get_skyrim_data_dir(runtime)
        .map(|p| p.display().to_string())
        .map_err(|e| napi::Error::from_reason(e.to_string()))
}

/// Collect both Nemesis and FNIS mods into a single vector.
///
/// # Errors
/// Returns [`napi::Error`] if glob expansion fails or files cannot be read.
#[napi]
pub fn load_mods_info(glob: String, is_vfs: bool) -> napi::Result<Vec<ModInfo>> {
    let infos =
        mod_info::get_all(&glob, is_vfs).map_err(|e| napi::Error::from_reason(e.to_string()))?;
    Ok(infos.into_par_iter().map(Into::into).collect())
}
