use rayon::prelude::*;
use std::path::{Path, PathBuf};

/// Collects all relevant file paths within the given ID directory.
///
/// # Arguments
/// - `path`: Path to the directory containing Nemesis XML files.
///
/// # Errors
/// Returns an error if path traversal fails.
pub fn collect_nemesis_paths(path: impl AsRef<Path>) -> Vec<PathBuf> {
    jwalk::WalkDir::new(path)
        .into_iter()
        .filter_map(|res| {
            if let Ok(path) = res.map(|entry| entry.path()) {
                if is_nemesis_file(&path) {
                    return Some(path);
                }
            }
            None
        })
        .collect()
}

/// Collect & flatten the path of a patch
pub fn collect_all_patch_paths(nemesis_paths: &[PathBuf]) -> Vec<PathBuf> {
    nemesis_paths
        .par_iter()
        .flat_map(collect_nemesis_paths)
        .collect()
}

pub(crate) fn is_nemesis_file(path: impl AsRef<Path>) -> bool {
    let path = path.as_ref();
    let is_txt = path
        .extension()
        .map_or(false, |ext| ext.eq_ignore_ascii_case("txt"));
    let is_sharp_prefix = path
        .file_stem()
        .and_then(|name| name.to_str().map(|name| name.starts_with('#')))
        .unwrap_or_default();

    path.is_file() && is_txt && is_sharp_prefix
}
