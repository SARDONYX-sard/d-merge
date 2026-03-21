//! # jwalk_glob
//!
//! A glob path expander backed by [`jwalk`] instead of the [`glob`] crate.
//!
//! ## Why not the `glob` crate?
//!
//! On Windows, Skyrim mod managers such as **Mod Organizer 2 (MO2)** virtualize
//! the filesystem through **USVFS**. USVFS serializes filesystem access with a
//! `RecursiveBenaphore` — a reentrant lock that tracks ownership per thread.
//!
//! The `glob` crate resolves patterns by calling `fill_todo` **recursively on
//! the same thread**:
//!
//! ```text
//! thread A
//!   └─ fill_todo → read_dir   (lock depth 1)
//!        └─ fill_todo → read_dir   (lock depth 2)  ← reentrant on same thread
//!             └─ fill_todo → read_dir   (lock depth 3)  ← USVFS lock corrupted
//! ```
//!
//! Once the benaphore overflows its reentrance counter, USVFS marks the virtual
//! filesystem as inconsistent. Subsequent calls to `fs::exists`, `fs::read_dir`,
//! and similar APIs return incorrect results or fail silently — roughly 50 % of
//! the time depending on the directory tree depth.
//!
//! [`jwalk`] walks directories using a **rayon thread pool**, issuing one
//! `read_dir` call per thread with no same-thread recursion:
//!
//! ```text
//! thread A: read_dir(actors/character/animations)   (lock depth 1, released)
//! thread B: read_dir(actors/canine/animations)      (lock depth 1, released)
//! thread C: read_dir(actors/dragon/animations)      (lock depth 1, released)
//! ```
//!
//! Each thread acquires and releases the USVFS lock independently, so the
//! benaphore never sees reentrance and the virtual filesystem stays intact.
//!
//! ## Usage
//!
//! ```rust,no_run
//! use jwalk_glob::glob;
//!
//! // Expand all mod data directories under a MO2 mods folder.
//! // Each subdirectory of `mods/` that contains a `Data` folder is returned.
//! let data_dirs = glob("C:/MO2/mods/*/Data");
//! for dir in &data_dirs {
//!     println!("{}", dir.display());
//! }
//!
//! // Recursive expansion — find every `Data` directory at any depth.
//! let deep = glob("C:/MO2/mods/**/Data");
//!
//! // Literal path — returned as-is without touching the filesystem.
//! let single = glob("C:/Skyrim Special Edition/Data");
//! assert_eq!(single, vec![std::path::PathBuf::from("C:/Skyrim Special Edition/Data")]);
//! ```

mod glob;

use std::path::PathBuf;

/// Expands a glob pattern into concrete **directory** paths using [`jwalk`].
///
/// If `pattern` contains no glob meta characters (`*`, `?`, `[`), the path is
/// returned as-is without any filesystem access.
///
/// # Examples
///
/// ```rust,no_run
/// for dir in jwalk_glob::glob_dirs("C:/MO2/mods/*/Data") {
///     println!("{}", dir.display());
/// }
/// ```
pub fn glob_dirs(pattern: &str) -> Vec<PathBuf> {
    expand(pattern, true)
}

/// Expands a glob pattern into concrete **file** paths using [`jwalk`].
///
/// If `pattern` contains no glob meta characters (`*`, `?`, `[`), the path is
/// returned as-is without any filesystem access.
///
/// # Examples
///
/// ```rust,no_run
/// for file in jwalk_glob::glob_files("C:/MO2/mods/**/FNIS_*_List.txt") {
///     println!("{}", file.display());
/// }
/// ```
pub fn glob_files(pattern: &str) -> Vec<PathBuf> {
    expand(pattern, false)
}

fn expand(pattern: &str, is_dir: bool) -> Vec<PathBuf> {
    let has_glob = pattern.contains(['*', '?', '[']);
    if !has_glob {
        return vec![PathBuf::from(pattern)];
    }

    let path = std::path::Path::new(pattern);
    let mut root = PathBuf::new();
    let mut found_glob = false;
    for component in path.components() {
        let s = component.as_os_str().to_str().unwrap_or("");
        if !found_glob && !s.contains(['*', '?', '[']) {
            root.push(component);
        } else {
            found_glob = true;
        }
    }

    let pat_after_root = std::path::Path::new(pattern)
        .strip_prefix(&root)
        .unwrap_or_else(|_| std::path::Path::new(pattern));
    let has_recursive = pat_after_root.components().any(|c| c.as_os_str() == "**");
    let pat_depth = pat_after_root.components().count();
    let max_depth = if has_recursive { usize::MAX } else { pat_depth };

    jwalk::WalkDir::new(&root)
        .max_depth(max_depth)
        .into_iter()
        .filter_map(|r| {
            r.map_err(|_e| {
                #[cfg(feature = "tracing")]
                tracing::warn!(error = %_e, "jwalk_glob: failed to read entry: {_e}");
            })
            .ok()
        })
        .filter(move |e| {
            if e.depth == 0 {
                return false;
            }

            let type_is_dir = e.file_type.is_dir();
            let type_match = match is_dir {
                true => type_is_dir,
                false => !type_is_dir,
            };
            type_match && self::glob::match_glob_path(pattern, &e.path())
        })
        .map(|e| e.path())
        .collect()
}
