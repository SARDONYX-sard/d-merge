use rayon::prelude::*;
use std::path::{Path, PathBuf};

/// Removes the auto `<output dir>/meshes` or `<output dir>/.d_merge/debug` directories with a safety warning if output_dir equals Skyrim data dir.
pub fn remove_meshes_dir_all(output_dir: impl AsRef<Path>) {
    let output_dir = output_dir.as_ref();

    rayon::join(
        || {
            let _ = remove_if_exists(output_dir.join("meshes"));
        },
        || {
            let _ = remove_if_exists(output_dir.join(".d_merge").join(".debug"));
        },
    );
}

/// Removes a directory if it exists, with debug logging.
///
/// # Why need this?
/// This is because the presence of a previous hkx may leave unintended changes behind.
///
/// # Reasons for not using `std::fs::remove_dir_all`
/// For some reason, egui on MO2 throws an error saying the path doesn't exist when I try to use `std::remove_dir_all`,
/// so I manually perform a recursive deletion starting from the end.
fn remove_if_exists(path: impl AsRef<Path>) -> std::io::Result<()> {
    use std::fs;

    let path = path.as_ref();

    if !path.exists() {
        return Ok(());
    }

    // Get the contents
    let entries: Vec<PathBuf> = fs::read_dir(path)?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .collect();

    // directories recursively deleted in parallel, files deleted in parallel
    entries.par_iter().for_each(|entry_path| {
        if entry_path.is_dir() {
            let _ = remove_if_exists(entry_path); // recursion
        } else {
            let _ = fs::remove_file(entry_path);
        }
    });

    // remove itself after deleting the contents
    fs::remove_dir(path)?;
    Ok(())
}
