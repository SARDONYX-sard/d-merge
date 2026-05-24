//! File-system helpers for locating and opening directories.
//!
//! Both functions walk up the path hierarchy until they find an existing
//! directory, which lets the app open a folder picker or an Explorer/Finder
//! window even when the stored path no longer exists (e.g. after a drive
//! letter change).
//!
//! # No `App` dependency
//! These are free functions — they take `impl AsRef<Path>` and return
//! `Result`.  Callers handle UI feedback (notifications, color) themselves.

use std::path::{Path, PathBuf};

/// Walks up the path hierarchy until an existing directory is found.
///
/// Starts at `dir` and checks each ancestor in turn.  Stops at the first
/// path that [`Path::exists`] returns `true` for, then canonicalizes it.
///
/// # Errors
/// Returns `Err(String)` when:
/// - No ancestor exists (reached the filesystem root without a hit).
/// - [`std::fs::canonicalize`] fails on the found path.
pub(crate) fn find_existing_dir_or_ancestor<P>(dir: P) -> Result<PathBuf, String>
where
    P: AsRef<Path>,
{
    let dir = dir.as_ref();
    let mut current = dir;

    loop {
        if current.exists() {
            return current
                .canonicalize()
                .map_err(|e| format!("Failed to canonicalize path({}): {e}", dir.display()));
        }
        match current.parent() {
            Some(parent) => current = parent,
            None => {
                return Err(format!(
                    "No existing directory found in path hierarchy ({})",
                    dir.display()
                ));
            }
        }
    }
}

/// Opens the given directory (or its closest existing ancestor) in the
/// platform's default file manager.
///
/// Calls [`find_existing_dir_or_ancestor`] and passes the result to
/// [`open::that_detached`], so the file manager opens without blocking the
/// UI thread.
///
/// # Errors
/// Returns `Err(String)` when no existing ancestor is found or the OS
/// fails to open the directory.
pub(crate) fn open_existing_dir_or_ancestor<P>(dir: P) -> Result<(), String>
where
    P: AsRef<Path>,
{
    let dir = dir.as_ref();
    let abs_dir = find_existing_dir_or_ancestor(dir)?;
    open::that_detached(abs_dir)
        .map_err(|e| format!("Failed to open directory ({}): {e}", dir.display()))
}
