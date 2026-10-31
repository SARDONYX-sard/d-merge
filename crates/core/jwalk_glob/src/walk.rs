//! Parallel directory walker built on `std::fs::read_dir` + rayon.
//!
//! # Why not `jwalk`?
//! - jwalk on the global rayon pool gives up with a busy timeout when the pool is occupied,
//!   and the entries are silently dropped.
//! - A glob only needs to read the dirs that match each pattern component,
//!   but jwalk walks the whole tree up to `max_depth`.
//!
//! # USVFS(MO2)
//! Each `read_dir` handle is fully consumed and closed before descending into its children,
//! so the same thread never holds nested `read_dir` handles. (See the crate docs.)

use std::{
    fs,
    path::{Path, PathBuf},
};

use rayon::prelude::*;

use crate::glob::match_segment;

/// A directory entry whose type is already resolved.
struct Entry {
    path: PathBuf,
    is_dir: bool,
}

/// Reads one directory and closes its handle before returning.
///
/// Errors are logged and skipped.
fn read_dir_entries(dir: &Path) -> Vec<Entry> {
    let read_dir = match fs::read_dir(dir) {
        Ok(read_dir) => read_dir,
        Err(_e) => {
            #[cfg(feature = "tracing")]
            tracing::warn!("jwalk_glob: failed to read dir {}: {_e}", dir.display());
            return vec![];
        }
    };

    read_dir
        .filter_map(|entry| {
            let entry = entry
                .map_err(|_e| {
                    #[cfg(feature = "tracing")]
                    tracing::warn!("jwalk_glob: failed to read entry in {}: {_e}", dir.display());
                })
                .ok()?;

            // NOTE: `file_type` is obtained from the dir listing itself, so this needs no extra `stat`.
            // Only symlinks need `stat` to follow the link.
            let file_type = entry.file_type().ok()?;
            let path = entry.path();
            let is_dir = if file_type.is_symlink() { path.is_dir() } else { file_type.is_dir() };
            Some(Entry { path, is_dir })
        })
        .collect()
}

/// Returns a parallel iterator over all files under `root`. Sub dirs are read in parallel.
///
/// - The order is prefix order of the tree. (Same as the dir listing order, when collected.)
/// - No intermediate `Vec` of all paths: `filter`/`map`/`collect` as needed on the caller side.
/// - This can be called inside `rayon::par_iter`. (No busy timeout like jwalk.)
pub fn walk_files(root: impl Into<PathBuf>) -> impl ParallelIterator<Item = PathBuf> {
    let root = Entry { path: root.into(), is_dir: true };

    rayon::iter::walk_tree_prefix(root, |entry| match entry.is_dir {
        true => read_dir_entries(&entry.path),
        false => vec![],
    })
    .filter_map(|entry| (!entry.is_dir).then_some(entry.path))
}

/// Expands the glob pattern components under `dir`.
///
/// Only the dirs matching each component are read.
///
/// # Assumptions
/// - `pattern` has no consecutive `**`.
pub(crate) fn expand_components(dir: &Path, pattern: &[&str], is_dir: bool) -> Vec<PathBuf> {
    if pattern.is_empty() {
        return vec![];
    }

    read_dir_entries(dir)
        .into_par_iter()
        .flat_map_iter(|entry| match pattern {
            ["**", rest @ ..] => {
                // `**` consumes this entry (and maybe more components).
                let mut found = if rest.is_empty() && entry.is_dir == is_dir {
                    vec![entry.path.clone()]
                } else {
                    vec![]
                };
                if entry.is_dir {
                    found.extend(expand_components(&entry.path, pattern, is_dir));
                }

                // `**` consumes zero components.
                found.extend(match_entry(entry, rest, is_dir));
                found
            }
            _ => match_entry(entry, pattern, is_dir),
        })
        .collect()
}

/// Matches `entry` with the first component of `pattern`, then descends with the rest.
fn match_entry(entry: Entry, pattern: &[&str], is_dir: bool) -> Vec<PathBuf> {
    let Some((first, rest)) = pattern.split_first() else {
        return vec![];
    };

    let is_match = entry
        .path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| match_segment(first, name));
    if !is_match {
        return vec![];
    }

    match (rest.is_empty(), entry.is_dir) {
        (true, entry_is_dir) if entry_is_dir == is_dir => vec![entry.path],
        (true, _) | (false, false) => vec![],
        (false, true) => expand_components(&entry.path, rest, is_dir),
    }
}
