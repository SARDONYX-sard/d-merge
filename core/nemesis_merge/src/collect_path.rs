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
                let file_name = path.file_stem()?.to_str()?;
                let is_nemesis_file = file_name.starts_with("#");
                if path.is_file() && is_nemesis_file {
                    return Some(path);
                }
            }
            None
        })
        .collect()
}
