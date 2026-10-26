//! # jwalk_glob
//!
//! A glob path expander backed by `std::fs::read_dir` + rayon instead of the [`glob`] crate.
//! (It was backed by `jwalk` before, hence the name.)
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
//! This crate reads each directory completely and closes its handle **before** descending into
//! the children, and walks the children on a **rayon thread pool**:
//!
//! ```text
//! thread A: read_dir(actors/character/animations)   (lock depth 1, released)
//! thread B: read_dir(actors/canine/animations)      (lock depth 1, released)
//! thread C: read_dir(actors/dragon/animations)      (lock depth 1, released)
//! ```
//!
//! Each `read_dir` acquires and releases the USVFS lock independently, so the
//! benaphore never sees reentrance and the virtual filesystem stays intact.
//!
//! Only the directories matching each pattern component are read.
//! (e.g. `mods/*/Data` reads `mods` only, not the whole tree under it.)
//!
//! ## Usage
//!
//! ```rust,no_run
//! use jwalk_glob::glob_dirs;
//!
//! // Expand all mod data directories under a MO2 mods folder.
//! // Each subdirectory of `mods/` that contains a `Data` folder is returned.
//! let data_dirs = glob_dirs("C:/MO2/mods/*/Data");
//! for dir in &data_dirs {
//!     println!("{}", dir.display());
//! }
//!
//! // Recursive expansion — find every `Data` directory at any depth.
//! let deep = glob_dirs("C:/MO2/mods/**/Data");
//!
//! // Literal path — returned as-is without touching the filesystem.
//! let single = glob_dirs("C:/Skyrim Special Edition/Data");
//! assert_eq!(single, vec![std::path::PathBuf::from("C:/Skyrim Special Edition/Data")]);
//! ```

mod glob;
mod walk;

use std::path::PathBuf;

pub use walk::walk_files;

/// Expands a glob pattern into concrete **directory** paths. (Sorted)
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

/// Expands a glob pattern into concrete **file** paths. (Sorted)
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

    // NOTE: Accept both `/` and `\` regardless of the platform.
    let mut components: Vec<&str> = pat_after_root
        .to_str()
        .unwrap_or_default()
        .split(['/', '\\'])
        .filter(|s| !s.is_empty())
        .collect();
    // `**/**` is the same as `**`. (Also required by `expand_components`.)
    components.dedup_by(|a, b| *a == "**" && *b == "**");

    let mut paths = walk::expand_components(&root, &components, is_dir);
    // The walk order depends on the thread scheduling, so fix it.
    paths.sort_unstable();
    paths
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use rayon::prelude::*;

    use super::*;

    /// Creates a dir tree under the temp dir and removes it on drop.
    struct TempTree(PathBuf);

    impl TempTree {
        fn new(name: &str, dirs: &[&str], files: &[&str]) -> Self {
            let root =
                std::env::temp_dir().join(format!("jwalk_glob_{name}_{}", std::process::id()));
            let _ = fs::remove_dir_all(&root);
            for dir in dirs {
                fs::create_dir_all(root.join(dir)).unwrap();
            }
            for file in files {
                fs::write(root.join(file), "").unwrap();
            }
            Self(root)
        }

        fn pattern(&self, pattern: &str) -> String {
            format!("{}/{pattern}", self.0.display())
        }

        fn relative(&self, paths: Vec<PathBuf>) -> Vec<String> {
            paths
                .iter()
                .map(|path| {
                    path.strip_prefix(&self.0).unwrap().to_string_lossy().replace('\\', "/")
                })
                .collect()
        }
    }

    impl Drop for TempTree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn mods_tree(name: &str) -> TempTree {
        TempTree::new(
            name,
            &[
                "mods/Data",
                "mods/FNISFlyer/Data/meshes",
                "mods/FNISSFlyer",
                "mods/TKDodge/Data",
                "mods/a/b/Data",
                "mods/a/b/Other",
                "mods/a/b/c/Data",
            ],
            &["mods/FNISFlyer/Data/meshes/a.txt", "mods/a/b/c/b.txt", "mods/a/b/c/c.hkx"],
        )
    }

    #[test]
    fn glob_star() {
        let tree = mods_tree("star");
        let actual = tree.relative(glob_dirs(&tree.pattern("mods/*/Data")));
        assert_eq!(actual, ["mods/FNISFlyer/Data", "mods/TKDodge/Data"]);
    }

    #[test]
    fn glob_partial_name_and_question_mark() {
        let tree = mods_tree("partial");
        let actual = tree.relative(glob_dirs(&tree.pattern("mods/FNIS*/Data")));
        assert_eq!(actual, ["mods/FNISFlyer/Data"]);

        let actual = tree.relative(glob_dirs(&tree.pattern("mods/FNI?Flyer")));
        assert_eq!(actual, ["mods/FNISFlyer"]);
    }

    #[test]
    fn glob_case_insensitive_and_backslash() {
        let tree = mods_tree("case");
        let actual = tree.relative(glob_dirs(&tree.pattern("mods\\*\\data")));
        assert_eq!(actual, ["mods/FNISFlyer/Data", "mods/TKDodge/Data"]);
    }

    #[test]
    fn glob_double_star() {
        let tree = mods_tree("double_star");
        let actual = tree.relative(glob_dirs(&tree.pattern("mods/**/Data")));
        assert_eq!(
            actual,
            [
                "mods/Data",
                "mods/FNISFlyer/Data",
                "mods/TKDodge/Data",
                "mods/a/b/Data",
                "mods/a/b/c/Data",
            ]
        );

        // `**/**` is the same as `**`.
        let actual = tree.relative(glob_dirs(&tree.pattern("mods/**/**/Data")));
        assert_eq!(actual.len(), 5);
    }

    #[test]
    fn glob_files_only() {
        let tree = mods_tree("files");
        let actual = tree.relative(glob_files(&tree.pattern("mods/**/*.txt")));
        assert_eq!(actual, ["mods/FNISFlyer/Data/meshes/a.txt", "mods/a/b/c/b.txt"]);

        // Dirs are not returned by `glob_files`.
        assert!(glob_files(&tree.pattern("mods/*/Data")).is_empty());
    }

    #[test]
    fn glob_literal_returns_as_is() {
        assert_eq!(glob_dirs("C:/Skyrim/Data"), [PathBuf::from("C:/Skyrim/Data")]);
    }

    #[test]
    fn walk_all_files() {
        let tree = mods_tree("walk");
        let is_txt = |path: &PathBuf| path.extension().is_some_and(|ext| ext == "txt");
        let mut actual = tree.relative(walk_files(tree.0.join("mods")).filter(is_txt).collect());
        actual.sort();
        assert_eq!(actual, ["mods/FNISFlyer/Data/meshes/a.txt", "mods/a/b/c/b.txt"]);
    }
}
