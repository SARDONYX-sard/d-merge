//! # FNIS Path collector
//!
//! ```txt
//! <skyrim data dir>/
//! └── meshes/
//!     └── actors/
//!         └── character/
//!             ├── animations/
//!             │   └── <fnis_mod_namespace>/           <- this is `animations_mod_dir`
//!             │       ├── *.hkx                 <- HKX animation files collected by `animation_paths`
//!             │       └── FNIS_<fnis_mod_namespace>_List.txt  <- List file read into `list_content`
//!             └── behaviors/
//!                 └── FNIS_<fnis_mod_namespace>_Behavior.hkx  <- Behavior file path returned as `behavior_path`
//! ```
//!
//! # Note
//! - Linux path is case sensitive: https://learn.microsoft.com/windows/wsl/case-sensitivity
use rayon::prelude::*;
use snafu::ResultExt as _;
use std::{
    fs, io,
    path::{Path, PathBuf},
};
use winnow::{ascii::Caseless, combinator::alt, seq, token::take_while, ModalResult, Parser};

use crate::behaviors::{
    priority_ids::take_until_ext, tasks::fnis::paths::parse::get_fnis_namespace,
};

/// The necessary information for creating a single FNIS mod as a d_merge patch for hkx.
#[derive(Debug)]
pub struct OwnedFnisInjection {
    /// The value must be unique.
    /// - `<namespace>` under `meshes\actors\character\animations\<namespace>\`.
    pub namespace: String,

    /// The index of the `paths` in the `nemesis_merge::behavior_gen` passed from the GUI is the priority, and that is passed.
    pub priority: usize,

    /// The contents of the FNIS list.txt file in this namespace.
    pub list_content: String,

    /// All `.hkx` files under `Meshes\actors\character\animations\<namespace>\`
    ///
    /// To register them within hkx as paths readable by the game engine,
    /// they are adjusted to always start with `Animations` and use `\` as the path separator.
    ///
    /// # Expected sample
    ///
    /// ```txt
    /// [
    ///   "Animations\FNISFlyer\FNISfl_Back_ac.hkx"
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
}

impl OwnedFnisInjection {
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

    let namespace = if animation_paths.is_empty() {
        return Err(FnisError::MissingNameSpace {
            path: animations_mod_dir.to_path_buf(),
        });
    } else {
        get_fnis_namespace(&animation_paths[0])?
    };

    // Case-insensitive List file lookup
    let list_file_name = format!("FNIS_{namespace}_List.txt");
    let list_path = find_case_insensitive(animations_mod_dir, &list_file_name).map_err(|e| {
        FnisError::ListMissing {
            expected: animations_mod_dir
                .join(&list_file_name)
                .to_string_lossy()
                .to_string(),
            source: e,
        }
    })?;
    let list_content = fs::read_to_string(&list_path).with_context(|_| ListMissingSnafu {
        expected: list_path.to_string_lossy().to_string(),
    })?;

    // Behavior file
    let behavior_path = {
        fn get_behavior_path(animations_mod_dir: &Path, unique_namespace: &str) -> Option<String> {
            let char_dir = animations_mod_dir.parent()?.parent()?;
            let behaviors_dir = find_case_insensitive(char_dir, "behaviors").ok()?;
            let file_name = format!("FNIS_{unique_namespace}_Behavior.hkx");
            let behavior_file = find_case_insensitive(&behaviors_dir, &file_name).ok()?;

            if behavior_file.exists() {
                // To ensure Skyrim reads the game, use `\` regardless of the OS.
                Some(format!("Behaviors\\FNIS_{unique_namespace}_Behavior.hkx"))
            } else {
                None
            }
        }

        get_behavior_path(animations_mod_dir, namespace).ok_or_else(|| {
            FnisError::BehaviorMissing {
                expected: format!("Behaviors\\FNIS_{namespace}_Behavior.hkx"),
            }
        })?
    };

    Ok(OwnedFnisInjection {
        namespace: namespace.to_string(),
        priority,
        list_content,
        animation_paths,
        behavior_path,
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
        .par_bridge()
        .into_par_iter()
        .filter_map(|entry| {
            let e = entry.ok()?;
            let path: PathBuf = e.path();

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
    let s = path.to_str()?;
    let rel = trim_till_animations.parse(s).ok()?;

    // Heap alloc optimization. avoid `replace & format!`
    // Convert to a single String and replace the entire string with backslashes.
    let mut normalized = String::with_capacity(rel.len() + 11); // Leave a little extra space for “Animations\”
    normalized.push_str("Animations\\");

    for c in rel.chars() {
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
        _: take_until_ext(0.., Caseless("meshes")),
        _: Caseless("meshes"),
        _: alt(('/', '\\')),
        _: Caseless("actors"),
        _: alt(('/', '\\')),
        _: Caseless("character"),
        _: alt(('/', '\\')),
        _: Caseless("animations"),
        _: alt(('/', '\\')),
        take_while(1.., |_| true),
    }
    .parse_next(input)?;
    Ok(rel)
}

#[derive(Debug, snafu::Snafu)]
pub enum FnisError {
    /// Failed to parse path as fnis path
    #[snafu(display("Failed to parse path as fnis path:\n{source}"))]
    FailedParseFnisPatchPath {
        source: serde_hkx::errors::readable::ReadableError,
    },

    #[snafu(display("Expected list file at {expected}, but not found such a path."))]
    ListMissing { expected: String, source: io::Error },

    #[snafu(display("The fnis mod was specified, but the namespace `meshes/character/actors/animations/<mod_namespace>` was not found within this path.: {}", path.display()))]
    MissingNameSpace { path: PathBuf },

    #[snafu(display("Expected behavior file at {expected}, but not found such a path."))]
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
