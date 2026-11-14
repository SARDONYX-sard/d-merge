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

// NOTE: ref open-5.4 crate src/window.rs
// SPDX-FileCopyrightText: (C) 2015 Sebastian Thiel
// SPDX-License-Identifier: MIT
/// Shows `file` selected in the file manager, as VS Code's "Reveal in File Explorer".
///
/// As Electron's `shell.showItemInFolder`, which VS Code uses. A file that doesn't exist (e.g. a
/// patch not written yet) opens what exists of its folder instead, as does a file manager that
/// can't select.
///
/// The `open` crate only opens a path with its default program: selecting asks the shell itself.
///
/// | OS      | how                                                                          |
/// | ------- | ---------------------------------------------------------------------------- |
/// | Windows | `SHOpenFolderAndSelectItems`: an Explorer window of the folder is reused     |
/// | macOS   | `open -R <file>`                                                             |
/// | Linux   | D-Bus `org.freedesktop.FileManager1.ShowItems`                               |
///
/// It runs on a thread of its own: the shell may take a moment, and the UI does not wait for it.
///
/// # Errors
/// The folder could not be opened, or the thread not started.
pub fn reveal_file(file: &Path) -> Result<(), String> {
    if !file.is_file() {
        return open_existing_dir_or_ancestor(file.parent().unwrap_or(file));
    }
    // Not `canonicalize`: on Windows it gives `\\?\D:\...`, which the shell doesn't take.
    let file = std::path::absolute(file).map_err(|e| format!("{}: {e}", file.display()))?;
    std::thread::Builder::new()
        .name("reveal_file".to_owned())
        .spawn(move || {
            if reveal_selected(&file).is_err() {
                let _ = open_existing_dir_or_ancestor(file.parent().unwrap_or(&file));
            }
        })
        .map(drop)
        .map_err(|e| format!("reveal_file: {e}"))
}

#[cfg(windows)]
fn reveal_selected(file: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt as _;

    /// `COINIT_APARTMENTTHREADED`: the shell's objects want a single-threaded apartment.
    const COINIT_APARTMENTTHREADED: u32 = 0x2;

    #[link(name = "shell32")]
    unsafe extern "system" {
        fn ILCreateFromPathW(path: *const u16) -> *mut core::ffi::c_void;
        fn ILFree(pidl: *mut core::ffi::c_void);
        fn SHOpenFolderAndSelectItems(
            folder: *const core::ffi::c_void,
            count: u32,
            items: *const *const core::ffi::c_void,
            flags: u32,
        ) -> i32;
    }
    #[link(name = "ole32")]
    unsafe extern "system" {
        fn CoInitializeEx(reserved: *const core::ffi::c_void, flags: u32) -> i32;
        fn CoUninitialize();
    }

    // The shell wants `\`. e.g. `D:/mods/My/#0106.txt` -> `D:\mods\My\#0106.txt`
    let path = file.as_os_str().encode_wide().map(|c| if c == u16::from(b'/') { u16::from(b'\\') } else { c });
    let path: Vec<u16> = path.chain(std::iter::once(0)).collect();
    // SAFETY: `path` is a NUL-terminated wide string that outlives the calls; the item list made
    // from it is freed once, after its use. COM is set up for this thread only, and undone when
    // it was set up here (`S_OK` / `S_FALSE`, not `RPC_E_CHANGED_MODE`).
    unsafe {
        let initialized = CoInitializeEx(std::ptr::null(), COINIT_APARTMENTTHREADED) >= 0;
        let item = ILCreateFromPathW(path.as_ptr());
        let result = if item.is_null() {
            Err(std::io::Error::last_os_error())
        } else {
            // No items: `item` itself is the one to select, in its folder (as Chromium does).
            let hresult = SHOpenFolderAndSelectItems(item, 0, std::ptr::null(), 0);
            ILFree(item);
            if hresult >= 0 { Ok(()) } else { Err(std::io::Error::from_raw_os_error(hresult)) }
        };
        if initialized {
            CoUninitialize();
        }
        result
    }
}

#[cfg(target_os = "macos")]
fn reveal_selected(file: &Path) -> std::io::Result<()> {
    let status = std::process::Command::new("open").arg("-R").arg(file).status()?;
    if status.success() { Ok(()) } else { Err(std::io::Error::other("Finder did not take it")) }
}

#[cfg(all(unix, not(target_os = "macos")))]
fn reveal_selected(file: &Path) -> std::io::Result<()> {
    let uri = format!("array:string:{}", file_uri(file));
    let status = std::process::Command::new("dbus-send")
        .args(["--session", "--print-reply", "--dest=org.freedesktop.FileManager1", "--type=method_call"])
        .args(["/org/freedesktop/FileManager1", "org.freedesktop.FileManager1.ShowItems"])
        .arg(uri)
        .arg("string:")
        .status()?;
    if status.success() { Ok(()) } else { Err(std::io::Error::other("no file manager took it")) }
}

/// `file://` and the path, its bytes other than the safe ones percent-encoded.
///
/// e.g. `/home/a/My Mod/#0106.txt` -> `file:///home/a/My%20Mod/%230106.txt`
#[cfg(any(test, all(unix, not(target_os = "macos"))))]
fn file_uri(file: &Path) -> String {
    use std::fmt::Write as _;
    let path = file.to_string_lossy();
    let mut uri = String::with_capacity(path.len() + 16);
    uri.push_str("file://");
    for byte in path.bytes() {
        if byte.is_ascii_alphanumeric() || b"/-._~".contains(&byte) {
            uri.push(char::from(byte));
        } else {
            let _ = write!(uri, "%{byte:02X}");
        }
    }
    uri
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_uri_is_percent_encoded() {
        assert_eq!(file_uri(Path::new("/home/a/My Mod/#0106.txt")), "file:///home/a/My%20Mod/%230106.txt");
    }
}
