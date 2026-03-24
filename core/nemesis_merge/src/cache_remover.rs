use std::{
    borrow::Cow,
    fs,
    path::{Path, PathBuf},
};

use rayon::prelude::*;

/// Is the Skyrim Data directory specified as the output directory?
///
/// This is to prevent the game environment from becoming corrupted.
#[inline]
pub fn is_dangerous_remove<O, P>(output_dir: O, skyrim_data_dir: P) -> bool
where
    O: AsRef<Path>,
    P: AsRef<Path>,
{
    let output_dir = output_dir.as_ref();
    let output_path = output_dir
        .canonicalize()
        .map_or_else(|_| Cow::Borrowed(output_dir), Cow::Owned);

    let skyrim_data_dir = skyrim_data_dir.as_ref();
    let skyrim_data_dir = skyrim_data_dir
        .canonicalize()
        .map_or_else(|_| Cow::Borrowed(skyrim_data_dir), Cow::Owned);

    output_path == skyrim_data_dir
}

/// Removes the auto `<output dir>/meshes` or `<output dir>/.d_merge/debug` directories.
///
/// # Warning!
///
/// Do not execute this function if the `Skyrim Data` directory is specified as the output directory.
/// Please check this first(use `is_dangerous_remove`). Failure to do so will corrupt your game environment.
pub fn remove_meshes_dir_all<O>(output_dir: O)
where
    O: AsRef<Path>,
{
    let output_dir = output_dir.as_ref();

    #[cfg(feature = "tracing")]
    tracing::debug!("Starting removal of `{}`", output_dir.display());

    rayon::join(
        || {
            let _ = remove_if_exists(output_dir.join("meshes"));
        },
        || {
            let _ = fs::remove_file(output_dir.join(".d_merge").join("d_merge_errors.log"));
            let _ = remove_if_exists(output_dir.join(".d_merge").join(".debug"));
            let _ = fs::remove_file(
                output_dir
                    .join("SKSE")
                    .join("Plugins")
                    .join("fnis_aa")
                    .join("config.json"),
            );
            // NOTE: Internally, BDI config use the same value. So there’s probably no need to delete it.
            // SKSE/Plugins/BehaviorDataInjector/FNIS_AA_to_OAR_BDI.json
        },
    );

    #[cfg(feature = "tracing")]
    tracing::debug!("Deletion of files in the output directory is complete.",);
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
