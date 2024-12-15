use super::bail;
use node_expr::{build_dir_tree, DirEntry};
use rayon::prelude::*;

#[cfg(feature = "extra_fmt")]
const FILTER: [&str; 4] = ["hkx", "xml", "json", "yaml"];
#[cfg(not(feature = "extra_fmt"))]
const FILTER: [&str; 2] = ["hkx", "xml"];

/// Loads a directory structure from the specified path, filtering by allowed extensions.
///
/// # Errors
/// Returns an error message if the directory cannot be loaded or if there are issues reading the path.
#[tauri::command]
pub(crate) fn load_dir_node(dirs: Vec<String>) -> Result<Vec<DirEntry>, String> {
    let (entries, errors): (Vec<_>, Vec<_>) = dirs
        .par_iter()
        .map(|dir| build_dir_tree(dir, FILTER).or_else(|err| bail!(err)))
        .partition(Result::is_ok);

    // Collect error messages and join them
    if !errors.is_empty() {
        let error_messages: Vec<String> = errors.into_par_iter().map(Result::unwrap_err).collect();
        return Err(error_messages.join("\n"));
    }

    Ok(entries.into_par_iter().map(Result::unwrap).collect())
}
