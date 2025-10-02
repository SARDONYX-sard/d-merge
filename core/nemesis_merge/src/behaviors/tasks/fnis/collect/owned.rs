//! # FNIS Path collector
//!
//! - [_<suffix>] is optional. e.g. `animations/FNISZoo/FNIS_FNISZoo_tail_List.txt`
//!
//! ```txt
//! <skyrim data dir>/
//! └── meshes/
//!     └── actors/
//!         ├── character/                                      <- defaultmale, defaultfemale humanoid animations
//!         │   ├── animations/
//!         │   │   └── <fnis_mod_namespace>/                   <- this is `animations_mod_dir`
//!         │   │       ├── *.hkx                               <- HKX animation files collected by `animation_paths`
//!         │   │       └── FNIS_<fnis_mod_namespace>[_<suffix>]_List.txt  <- List file read into `list_content`
//!         │   └── behaviors/
//!         │       └── FNIS_<fnis_mod_namespace>_Behavior.hkx  <- Behavior file path returned as `behavior_path`
//!         │
//!         └── cow/                                            <- any Creature
//!             ├── animations/
//!             │   └── <fnis_mod_namespace>/                   <- this is `animations_mod_dir`
//!             │       ├── *.hkx                               <- HKX animation files collected by `animation_paths`
//!             │       └── FNIS_<fnis_mod_namespace>[_<suffix>]_List.txt  <- List file read into `list_content`
//!             └── behaviors/
//!                 └── FNIS_<fnis_mod_namespace>_Behavior.hkx  <- Behavior file path returned as `behavior_path`
//! ```
//!
//! # Note
//! - Linux path is case sensitive: https://learn.microsoft.com/windows/wsl/case-sensitivity
use std::{
    io,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};

use crate::behaviors::tasks::fnis::{
    collect::collect_paths, patch_gen::generated_behaviors::BehaviorEntry,
};

/// The necessary information for creating a single FNIS mod as a d_merge patch for hkx.
#[derive(Debug)]
pub struct OwnedFnisInjection {
    /// Actor name. (e.g. `character`, `dragon`, `dog`)
    pub behavior_entry: &'static BehaviorEntry,

    /// Primarily used for generating havok class IDs(XML name attribute). e.g. `#namespace$1` (The value must be unique.)
    ///
    /// In Nemesis, it is called `mod_code`.
    /// - `<namespace>` under `meshes\actors\character\animations\<namespace>\`.
    pub namespace: String,

    /// The index of the `paths` in the `nemesis_merge::behavior_gen` passed from the GUI is the priority, and that is passed.
    pub priority: usize,

    /// The contents of the FNIS list.txt files in this namespace.
    pub list_contents: Vec<String>,

    /// Relative path to the behavior file.
    ///
    /// NOTE: To ensure Skyrim reads the game, use `\` regardless of the OS.
    ///
    /// e.g., `Behaviors\FNIS_FNISFlyer_Behavior.hkx`
    ///
    /// # Why is this necessary?
    ///
    /// This path is used when registering the behavior in the generated
    /// `hkbBehaviorReferenceGenerator.behaviorName`.
    ///
    /// Steps:
    /// 1. Set the `generator` field in each created `hkbStateMachineStateInfo`
    ///    to the generator's index.
    /// 2. Set the `stateId` field to the hashed version of this string.
    /// 3. Push the state info index into `hkbStateMachine.states` so that
    ///    vanilla HKX recognizes the behavior.
    pub behavior_path: String,

    /// Every time an XML C++ root class is added, a sequential number must be generated.
    ///
    /// However, `d_merge_serde_hkx` has been extended to allow direct use of Nemesis variables
    /// (which use 1-based indexing and have the format `#<mod_code>$<index>`) as indices.
    ///
    /// Therefore, a counter is established per mod to prevent ID duplication.
    ///
    /// # Intended additional pattern
    /// Note that the actual code uses XML that has been further converted to JSON.
    /// ```xml
    /// <!-- Add new class (current_class_index:1) -->
    /// <hkobject name="#FNIS_Flyer$1" class="hkbStateMachine" signature="0x816c1dcb">...</hkobject>
    ///
    /// <!-- increment index & Add new class again (current_class_index:2) -->
    /// <hkobject name="#FNIS_Flyer$2" class="hkbStateMachine" signature="0x816c1dcb">...</hkobject>
    /// ```
    current_class_index: AtomicUsize,
}

impl OwnedFnisInjection {
    /// Increments the index and returns the full `name` attribute
    /// string in Nemesis format: `#<mod_code>$<index>`.
    ///
    /// # Example
    /// ```
    /// let inj = OwnedFnisInjection { current_class_index: AtomicUsize::new(0), mod_code: "FNIS_Flyer".into() };
    /// assert_eq!(inj.next_class_name_attribute(), "#FNIS_Flyer$1");
    /// assert_eq!(inj.next_class_name_attribute(), "#FNIS_Flyer$2");
    /// ```
    pub fn next_class_name_attribute(&self) -> String {
        let idx = self.next_class_index();
        format!("#{}${}", self.namespace, idx)
    }

    /// Increments the `current_class_index` counter and returns the **next available index**.
    ///
    /// This is used when registering a new XML C++ root class, ensuring
    /// that the Nemesis-style index (`#<mod_code>$<index>`) remains unique.
    ///
    /// # Ordering
    /// - Uses [`Ordering::Acquire`] to ensure correct memory synchronization across threads.
    ///
    /// # Returns
    /// The incremented class index (1-based).
    pub fn next_class_index(&self) -> usize {
        self.current_class_index.fetch_add(1, Ordering::Acquire) + 1
    }
}

/// Collects FNIS injection data from a Skyrim FNIS mod directory.
///
/// This function scans a given FNIS mod dir for animation files,
/// reads the corresponding List file, and resolves the path to the
/// Behavior `.hkx` file.
///
/// * `animations_mod_dir` - The root directory of the FNIS mod to process.
///   Must point to:
///
/// ```txt
/// <skyrim data dir>/
/// └── meshes/
///     └── actors/
///         └── character/
///             ├── animations/
///             │   └── <fnis_mod_namespace>/           <- this is `animations_mod_dir`
///             │       ├── *.hkx                 <- HKX animation files collected by `animation_paths`
///             │       └── FNIS_<fnis_mod_namespace>_List.txt  <- List file read into `list_content`
///             └── behaviors/
///                 └── FNIS_<fnis_mod_namespace>_Behavior.hkx  <- Behavior file path returned as `behavior_path`
/// ```
///
/// # Returns
///
/// Returns an `OwnedFnisInjection` struct with:
///
/// # Errors
///
/// Returns `FnisError` if:
/// - The animation directory is empty (`MissingNameSpace`)
/// - The List file is missing (`ListMissing`)
/// - The Behavior file is missing (`BehaviorMissing`)
pub fn collect_fnis_injection<P>(
    animations_mod_dir: P,
    behavior_entry: &'static BehaviorEntry,
    namespace: &str,
    priority: usize,
) -> Result<OwnedFnisInjection, FnisError>
where
    P: AsRef<Path>,
{
    let animations_mod_dir = animations_mod_dir.as_ref();

    let list_contents = load_all_fnis_list_files(animations_mod_dir, namespace)?;
    let behavior_path = find_behavior_file(animations_mod_dir, namespace)?;

    Ok(OwnedFnisInjection {
        behavior_entry,
        namespace: namespace.to_string(),
        priority,
        list_contents,
        behavior_path,
        current_class_index: AtomicUsize::new(0),
    })
}

/// Load all FNIS list files for a given namespace using glob.
///
/// Supports:
/// - `FNIS_<namespace>_List.txt`
/// - `FNIS_<namespace>_<suffix>_List.txt`
///
/// Returns a vector of the contents of all matched list files.
///
/// # Errors
/// Returns an error if no file is found.
fn load_all_fnis_list_files(
    animations_mod_dir: &Path,
    namespace: &str,
) -> Result<Vec<String>, FnisError> {
    let pattern = animations_mod_dir
        .join(format!("FNIS_{namespace}*_List.txt"))
        .to_string_lossy()
        .to_string();

    let paths = collect_paths(&pattern).map_err(|e| FnisError::ListMissing {
        expected: pattern.clone(),
        source: std::io::Error::new(std::io::ErrorKind::InvalidInput, e.to_string()),
    })?;

    if paths.is_empty() {
        return Err(FnisError::ListMissing {
            expected: pattern,
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "FNIS list not found"),
        });
    }

    // Read all matched list files
    let mut contents = Vec::with_capacity(paths.len());
    for path in paths {
        let content = std::fs::read_to_string(&path).map_err(|e| FnisError::ListMissing {
            expected: path.to_string_lossy().to_string(),
            source: e,
        })?;
        contents.push(content);
    }

    Ok(contents)
}

/// Find a FNIS behavior file for a given namespace, supporting optional suffix.
///
/// Looks for:
/// - `FNIS_<namespace>_Behavior.hkx`
/// - `FNIS_<namespace>_<suffix>_Behavior.hkx`
///
/// Returns the relative path to the behavior file (with `\` separators)
///
/// # Errors
/// An error if not found.
fn find_behavior_file(animations_mod_dir: &Path, namespace: &str) -> Result<String, FnisError> {
    let parent_dir = animations_mod_dir
        .parent()
        .and_then(|p| p.parent())
        .ok_or_else(|| FnisError::BehaviorParentMissing {
            animations_mod_dir: animations_mod_dir.to_path_buf(),
        })?;

    let mut behaviors_file = parent_dir.join("behaviors");
    behaviors_file.push(format!("FNIS_{namespace}*_Behavior.hkx"));
    let pattern = behaviors_file.to_string_lossy().to_string();

    let matches =
        collect_paths(&pattern).map_err(|source| FnisError::BehaviorInvalidGlobPatten {
            pattern: pattern.clone(),
            source,
        })?;

    match matches.len() {
        0 => Err(FnisError::BehaviorNotFound { pattern }),
        1 => {
            let path = &matches[0];
            let rel_path = path
                .strip_prefix(parent_dir)
                .map_err(|_| FnisError::BehaviorRelativePathError {
                    full_path: path.clone(),
                    base: parent_dir.to_path_buf(),
                })?
                .components()
                .map(|c| c.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("\\");
            Ok(rel_path)
        }
        _ => {
            let files: Vec<String> = matches
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect();
            Err(FnisError::MultipleBehaviorFiles { files })
        }
    }
}

#[derive(Debug, snafu::Snafu)]
pub enum FnisError {
    /// One or more HKX files were expected below the given directory, but none were found.
    #[snafu(display(
        "One or more hkx files were expected below(`{animations_mod_dir:?}`), but none were found."
    ))]
    EmptyAnimPaths { animations_mod_dir: PathBuf },

    /// Expected list file at `{expected}`, but the file was not found.
    #[snafu(display("Expected list file at {expected}, but not found such a path."))]
    ListMissing { expected: String, source: io::Error },

    /// Failed to get the parent directory of the animations mod directory.
    /// This indicates that the provided `animations_mod_dir` is too shallow in the filesystem hierarchy.
    #[snafu(display("Failed to get parent directory for `{animations_mod_dir:?}`"))]
    BehaviorParentMissing { animations_mod_dir: PathBuf },

    /// I/O error occurred while searching for behavior files matching the given pattern.
    #[snafu(display("Failed to search for behaviors with pattern `{pattern}`: {source}"))]
    BehaviorInvalidGlobPatten {
        pattern: String,
        source: glob::PatternError,
    },

    /// No behavior files found matching the expected pattern.
    /// For example: `FNIS_<namespace>_Behavior.hkx` or `FNIS_<namespace>_<suffix>_Behavior.hkx`.
    #[snafu(display("No behavior file found matching pattern `{pattern}`"))]
    BehaviorNotFound { pattern: String },

    /// Multiple behavior files were found matching the pattern.
    /// This is ambiguous, as FNIS expects exactly one behavior file per namespace.
    #[snafu(display("Multiple behavior files found: {files:?}"))]
    MultipleBehaviorFiles { files: Vec<String> },

    /// Failed to convert the absolute path to a relative path from the parent directory.
    #[snafu(display("Failed to convert `{full_path:?}` to relative path from `{base:?}`"))]
    BehaviorRelativePathError { full_path: PathBuf, base: PathBuf },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collect_fnis_injection() {
        use crate::behaviors::tasks::fnis::patch_gen::generated_behaviors::HUMANOID;

        let input = "../../dummy/fnis_test_mods/FNIS Flyer SE 7.0/Data/Meshes/actors/character/animations/FNISFlyer";
        let behavior_entry = HUMANOID.get("character").unwrap();
        let res = collect_fnis_injection(input, behavior_entry, "FNISFlyer", 0)
            .unwrap_or_else(|e| panic!("{e}"));
        dbg!(res);
    }
}
