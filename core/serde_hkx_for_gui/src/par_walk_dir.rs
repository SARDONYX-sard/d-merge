pub use node_expr::DirEntry;
use node_expr::{build_dir_tree, error::Error};
use rayon::prelude::*;

const FILTER: [&str; 3] = ["hkx", "xml", "json"];

/// Loads a directory structure from the specified path, filtering by allowed extensions.
///
/// # Errors
/// Returns an error message if the directory cannot be loaded or if there are issues reading the path.
pub fn load_dir_node(dirs: Vec<String>) -> Result<Vec<DirEntry>, Vec<Error>> {
    let (entries, errors): (Vec<_>, Vec<_>) = dirs
        .par_iter()
        .map(|dir| build_dir_tree(dir, FILTER))
        .partition(Result::is_ok);

    // Collect error messages and join them
    if !errors.is_empty() {
        let error_messages: Vec<node_expr::error::Error> =
            errors.into_par_iter().map(Result::unwrap_err).collect();
        return Err(error_messages);
    }

    Ok(entries.into_par_iter().map(Result::unwrap).collect())
}
