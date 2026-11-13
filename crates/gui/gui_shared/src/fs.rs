//! File-system helpers for locating and opening directories, and showing a file in them.
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
            return current.canonicalize().map_err(|e| format!("Failed to canonicalize path({}): {e}", dir.display()));
        }
        match current.parent() {
            Some(parent) => current = parent,
            None => {
                return Err(format!("No existing directory found in path hierarchy ({})", dir.display()));
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
pub fn open_existing_dir_or_ancestor<P>(dir: P) -> Result<(), String>
where
    P: AsRef<Path>,
{
    let dir = dir.as_ref();
    let abs_dir = find_existing_dir_or_ancestor(dir)?;
    open::that_detached(abs_dir).map_err(|e| format!("Failed to open directory ({}): {e}", dir.display()))
}

/// [`open_existing_dir_or_ancestor`] for a path as text.
///
/// # Errors
/// Neither the directory nor an ancestor could be opened.
#[inline]
pub fn open_dir_of(path: &str) -> Result<(), String> {
    open_existing_dir_or_ancestor(path)
}

/// Shows `file` selected in the file manager, as VS Code's "Reveal in File Explorer". A file that
/// doesn't exist (e.g. a patch not written yet) opens what exists of its folder instead.
///
/// The `open` crate only opens a path with its default program: selecting needs the file manager
/// itself.
///
/// | OS      | how                                                                          |
/// | ------- | ---------------------------------------------------------------------------- |
/// | Windows | `explorer /select,"D:\mods\My\#0106.txt"`                                 |
/// | macOS   | `open -R <file>`                                                             |
/// | Linux   | D-Bus `org.freedesktop.FileManager1.ShowItems`; else the folder is opened   |
///
/// # Errors
/// Neither the file manager nor the folder could be opened.
pub fn reveal_file(file: &Path) -> Result<(), String> {
    if !file.is_file() {
        return open_existing_dir_or_ancestor(file.parent().unwrap_or(file));
    }
    // Not `canonicalize`: on Windows it gives `\\?\D:\...`, which Explorer doesn't take.
    let file = std::path::absolute(file).map_err(|e| format!("{}: {e}", file.display()))?;
    match reveal_selected(&file) {
        Ok(()) => Ok(()),
        Err(_) => open_existing_dir_or_ancestor(file.parent().unwrap_or(&file)),
    }
}

#[cfg(windows)]
fn reveal_selected(file: &Path) -> std::io::Result<()> {
    use std::os::windows::process::CommandExt as _;
    // Explorer reads `/select,` and the quoted path itself: Rust's quoting of a whole argument
    // (`"/select,D:\a b"`) is not what it takes. It wants `\`, not `/`.
    let path = file.to_string_lossy().replace('/', "\\");
    std::process::Command::new("explorer").raw_arg(format!("/select,\"{path}\"")).spawn().map(drop)
}

#[cfg(target_os = "macos")]
fn reveal_selected(file: &Path) -> std::io::Result<()> {
    std::process::Command::new("open").arg("-R").arg(file).spawn().map(drop)
}

#[cfg(all(unix, not(target_os = "macos")))]
fn reveal_selected(file: &Path) -> std::io::Result<()> {
    let uri = format!("array:string:file://{}", file.display());
    let status = std::process::Command::new("dbus-send")
        .args(["--session", "--print-reply", "--dest=org.freedesktop.FileManager1", "--type=method_call"])
        .args(["/org/freedesktop/FileManager1", "org.freedesktop.FileManager1.ShowItems"])
        .arg(uri)
        .arg("string:")
        .status()?;
    if status.success() { Ok(()) } else { Err(std::io::Error::other("no file manager took it")) }
}

/// The folder dialog, starting from `current` or else what exists of it. `None`: cancelled.
#[inline]
pub fn pick_folder_from(current: &str) -> Option<std::path::PathBuf> {
    let dialog = crate::rfd::FileDialog::new();
    let dialog = match find_existing_dir_or_ancestor(current) {
        Ok(start) => dialog.set_directory(start),
        Err(_) => dialog,
    };
    dialog.pick_folder()
}
