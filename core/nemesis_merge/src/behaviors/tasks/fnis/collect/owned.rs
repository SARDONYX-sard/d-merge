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
    fs, io,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};

use rayon::prelude::*;
use snafu::ResultExt as _;
use winnow::{
    ascii::Caseless,
    combinator::alt,
    error::{StrContext, StrContextValue},
    seq,
    token::take_while,
    ModalResult, Parser,
};

use crate::behaviors::{
    priority_ids::take_until_ext,
    tasks::fnis::collect::{owned_::collect_paths, parse::get_fnis_namespace},
};

/// The necessary information for creating a single FNIS mod as a d_merge patch for hkx.
#[derive(Debug)]
pub struct OwnedFnisInjection {
    /// Primarily used for generating havok class IDs(XML name attribute). e.g. `#namespace$1` (The value must be unique.)
    ///
    /// In Nemesis, it is called `mod_code`.
    /// - `<namespace>` under `meshes\actors\character\animations\<namespace>\`.
    pub namespace: String,

    /// The index of the `paths` in the `nemesis_merge::behavior_gen` passed from the GUI is the priority, and that is passed.
    pub priority: usize,

    /// The contents of the FNIS list.txt files in this namespace.
    pub list_contents: Vec<String>,

    /// All `.hkx` files under `Meshes\actors\character\animations\<namespace>\`
    ///
    /// To register them within hkx as paths readable by the game engine,
    /// they are adjusted to always start with `Animations` and use `\` as the path separator.
    ///
    /// # Expected sample
    ///
    /// ```log
    /// [
    ///   "Animations\FNISFlyer\FNISfl_Back_ac.hkx",
    ///   "Animations\FNISFlyer\FNISfl_Back_fm.hkx"
    /// ]
    /// ```
    pub animation_paths: Vec<String>,

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

    /// Returns the behavior file name without the `Behaviors\` prefix and `.hkx` extension.
    ///
    /// e.g., for `Behaviors\FNIS_FNISFlyer_Behavior.hkx` this returns
    /// `"FNIS_FNISFlyer_Behavior"`.
    ///
    /// Used as input for `hkbBehaviorReferenceGenerator.name` during patch generation.
    pub fn behavior_name(&self) -> &str {
        let path = &self.behavior_path;
        let file_name = path.rsplit('\\').next().unwrap_or(path);
        file_name.strip_suffix(".hkx").unwrap_or(file_name)
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
    priority: usize,
) -> Result<OwnedFnisInjection, FnisError>
where
    P: AsRef<Path>,
{
    let animations_mod_dir = animations_mod_dir.as_ref();

    // Collect hkx entries
    let animation_paths = collect_hkx_entries(animations_mod_dir);
    if animation_paths.is_empty() {
        return Err(FnisError::EmptyAnimPaths {
            animations_mod_dir: animations_mod_dir.to_path_buf(),
        });
    }

    let binding = animations_mod_dir.to_string_lossy();
    let namespace = get_fnis_namespace(binding.as_ref())?;

    let list_contents = load_all_fnis_list_files(animations_mod_dir, namespace)?;
    let behavior_path = find_behavior_file(animations_mod_dir, namespace)?;

    Ok(OwnedFnisInjection {
        namespace: namespace.to_string(),
        priority,
        list_contents,
        animation_paths,
        behavior_path,
        current_class_index: AtomicUsize::new(0),
    })
}

/// Performs a case-insensitive file or directory search on Linux.
///
/// On Linux, filesystem paths are case-sensitive. To ensure that FNIS
/// mod files can be found regardless of capitalization, this function
/// searches a directory for a file or folder matching `name` case-insensitively.
///
/// On Windows and other case-insensitive filesystems, this function
/// simply returns `base.join(name)` with **zero overhead**.
///
/// # Errors
/// - Not found target name.
/// - If dir is empty.
#[cfg(target_os = "linux")]
fn find_case_insensitive(base: &Path, name: &str) -> std::io::Result<PathBuf> {
    let name_lower = name.to_lowercase();
    for entry in fs::read_dir(base)? {
        let entry = entry?;
        if entry.file_name().to_string_lossy().to_lowercase() == name_lower {
            return Ok(entry.path());
        }
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::DirectoryNotEmpty,
        "Dir is empty.",
    ))
}

#[cfg(not(target_os = "linux"))]
fn find_case_insensitive(base: &Path, name: &str) -> std::io::Result<PathBuf> {
    Ok(base.join(name))
}

fn collect_hkx_entries(root: &Path) -> Vec<String> {
    let mut paths: Vec<String> = jwalk::WalkDir::new(root)
        .into_iter()
        .filter_map(|entry| {
            let entry = match entry {
                Ok(entry) => entry,
                Err(e) => {
                    tracing::error!(%e);
                    return None;
                }
            };
            let path: PathBuf = entry.path();

            if path.extension()?.eq_ignore_ascii_case("hkx") {
                normalize_hkx_path(&path)
            } else {
                None
            }
        })
        .collect();

    paths.sort_unstable(); // `par_bridge` is unordered. So we need sort.
    paths
}

/// To `Animations\<inner_path>\*.hkx`
///
/// NOTE: To ensure Skyrim reads the game, use `\` regardless of the OS.
fn normalize_hkx_path(path: &Path) -> Option<String> {
    let Some(s) = path.to_str() else {
        tracing::info!("unsupported non utf8 hkx file path: {}", path.display());
        return None;
    };
    // intended: `meshes\actors\character\animations\FNISZoo\sample.hkx` -> hkx_path `FNISZoo\sample.hkx`
    let hkx_path = match trim_till_animations.parse(s) {
        Ok(hkx_path) => hkx_path,
        Err(e) => {
            tracing::error!(%e);
            return None;
        }
    };

    // Heap alloc optimization. avoid `replace & format!`
    // Convert to a single String and replace the entire string with backslashes.
    let mut normalized = String::with_capacity(hkx_path.len() + 11); // Leave a little extra space for “Animations\”
    normalized.push_str("Animations\\");

    for c in hkx_path.chars() {
        if matches!(c, '\\' | '/') {
            normalized.push('\\');
        } else {
            normalized.push(c);
        }
    }

    Some(normalized)
}

/// Return the substring relative to `meshes\actors\character\animations\`
fn trim_till_animations<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
    let (rel,) = seq! {
        _: take_until_ext(0.., Caseless("animations")).context(StrContext::Expected(StrContextValue::Description("animations"))),
        _: Caseless("animations").context(StrContext::Expected(StrContextValue::Description("animations"))),
        _: alt(('/', '\\')).context(StrContext::Expected(StrContextValue::Description("path separator: /"))),
        take_while(1.., |_| true),
    }
    .parse_next(input)?;
    Ok(rel)
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
/// or an error if not found.
fn find_behavior_file(animations_mod_dir: &Path, namespace: &str) -> Result<String, FnisError> {
    let char_dir = animations_mod_dir
        .parent()
        .and_then(|p| p.parent())
        .ok_or_else(|| FnisError::BehaviorMissing {
            expected: format!("Behaviors\\FNIS_{}_Behavior.hkx", namespace),
        })?;

    let behaviors_dir =
        find_case_insensitive(char_dir, "behaviors").map_err(|_| FnisError::BehaviorMissing {
            expected: namespace.to_string(),
        })?;

    // Build a glob pattern for suffix support
    let pattern = behaviors_dir
        .join(format!("FNIS_{}*_Behavior.hkx", namespace))
        .to_string_lossy()
        .to_string();

    let mut matches = collect_paths(&pattern).map_err(|_| FnisError::BehaviorMissing {
        expected: pattern.clone(),
    })?;

    // No match -> error
    let path = matches.pop().ok_or_else(|| FnisError::BehaviorMissing {
        expected: pattern.clone(),
    })?;

    // Convert to relative path with backslashes
    let rel_path = path
        .strip_prefix(char_dir)
        .unwrap_or(&path)
        .components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("\\");

    Ok(rel_path)
}

#[derive(Debug, snafu::Snafu)]
pub enum FnisError {
    /// One or more hkx files were expected below(`{animations_mod_dir}`), but none were found.
    #[snafu(display("One or more hkx files were expected below(`{}`), but none were found.", animations_mod_dir.display()))]
    EmptyAnimPaths { animations_mod_dir: PathBuf },

    /// Failed to parse path as fnis path
    #[snafu(display("Failed to parse namespace(e.g. `FNISZoo`) from this path:\n{source}"))]
    FailedToGetFnisNamespace {
        source: serde_hkx::errors::readable::ReadableError,
    },

    /// Expected list file at {expected}, but not found such a path.
    ListMissing { expected: String, source: io::Error },

    /// Expected behavior file at `Behaviors\\FNIS_{expected}[_<suffix>]_Behavior.hkx`(e.g. FNIS_FNISZoo_tail_Behavior.txt), but not found such a path.
    BehaviorMissing { expected: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_relative_path() {
        // windows
        let s = r"D:\Game\Data\Meshes\actors\character\animations\FNISTest\Foo.hkx";
        let rel = trim_till_animations.parse(s).unwrap();
        assert_eq!(rel, r"FNISTest\Foo.hkx");

        // unix
        let s = "/mnt/data/Meshes/actors/character/animations/FNISTest/Foo.hkx";
        let rel = trim_till_animations.parse(s).unwrap();
        assert_eq!(rel, "FNISTest/Foo.hkx");

        let s = "Animations\\FNISTest\\Foo.hkx";
        let rel = trim_till_animations.parse(s).unwrap();
        assert_eq!(rel, "FNISTest\\Foo.hkx");
    }

    #[test]
    fn test_normalize_hkx_path() {
        // windows
        let path =
            Path::new(r"D:\Game\Data\Meshes\actors\character\animations\FNISFlyer\FNISfl_Back.hkx");
        let normalized = normalize_hkx_path(path).unwrap();
        assert_eq!(normalized, r"Animations\FNISFlyer\FNISfl_Back.hkx");

        // unix
        let path =
            Path::new("/mnt/data/Meshes/actors/character/animations/FNISFlyer/FNISfl_Back.hkx");
        let normalized = normalize_hkx_path(path).unwrap();
        assert_eq!(normalized, r"Animations\FNISFlyer\FNISfl_Back.hkx");

        // mix
        let path =
            Path::new("/mnt/data\\Meshes/actors/character/animations/FNISFlyer\\FNISfl_Back.hkx");
        let normalized = normalize_hkx_path(path).unwrap();
        assert_eq!(normalized, r"Animations\FNISFlyer\FNISfl_Back.hkx");
    }

    #[test]
    fn test_collect_fnis_injection() {
        let input = "../../dummy/fnis_test_mods/FNIS Flyer SE 7.0/Data/Meshes/actors/character/animations/FNISFlyer";
        let res = collect_fnis_injection(input, 0).unwrap_or_else(|e| panic!("{e}"));
        dbg!(res);
    }
}
