//! Rotation logic: rename the current log file with a timestamp suffix and
//! delete the oldest files if the cap is exceeded.

use std::{
    fs::{self, DirEntry, File},
    path::Path,
    time::SystemTime,
};

use chrono::Local;

use crate::error::Result;

/// Create a new, empty log file at `log_dir/file_name`, rotating existing
/// files out of the way.
///
/// Rotation steps:
/// 1. Collect every file in `log_dir` whose name starts with the derived stem.
/// 2. If the count is already at `max_files`, delete the oldest one (by
///    modification time).
/// 3. Rename the current active file (if present) to
///    `{stem}_{YYYY-MM-DD_HH-MM-SS}{ext}`.
/// 4. Create and return a fresh, empty file at `log_dir/file_name`.
///
/// If `file_name` has no stem, the full file name is used as the stem.
///
/// # Errors
/// Propagates any [`std::io::Error`] encountered during directory creation,
/// file renaming, deletion, or file creation.
pub fn rotate_files(log_dir: impl AsRef<Path>, file_name: &str, max_files: usize) -> Result<File> {
    let log_dir = log_dir.as_ref();
    let file_name_path = Path::new(file_name);

    fs::create_dir_all(log_dir)?;

    let stem = file_name_path.file_stem().and_then(|s| s.to_str()).unwrap_or(file_name);
    let ext = file_name_path.extension().and_then(|s| s.to_str()).unwrap_or("log");

    let mut log_files: Vec<DirEntry> = fs::read_dir(log_dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_name().to_str().is_some_and(|name| name.starts_with(stem)))
        .collect();

    // Delete the oldest file first when we are at capacity.
    if log_files.len() >= max_files {
        log_files.sort_by_key(modification_time);

        if let Some(oldest) = log_files.first() {
            fs::remove_file(oldest.path())?;
        }
    }

    // Rename the currently active file so the new session gets a clean slate.
    let active_path = log_dir.join(file_name);

    if active_path.exists() {
        let timestamp = Local::now().format("%F_%H-%M-%S");
        let archived = log_dir.join(format!("{stem}_{timestamp}.{ext}"));

        fs::rename(&active_path, archived)?;
    }

    Ok(File::create(active_path)?)
}

// ────────────────────────────────────────────────────────────────────────────
// Helpers
// ────────────────────────────────────────────────────────────────────────────

fn modification_time(entry: &DirEntry) -> Option<SystemTime> {
    entry.metadata().ok()?.modified().ok()
}

// ────────────────────────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::{fs, time::Duration};

    use pretty_assertions::assert_eq;

    use super::*;

    fn file_count(dir: &std::path::Path) -> usize {
        fs::read_dir(dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_ok_and(|t| !t.is_dir()))
            .count()
    }

    #[test]
    fn caps_at_max_files() -> Result<()> {
        let tmp = temp_dir::TempDir::new()?;
        let dir = tmp.path();

        for _ in 0..6 {
            rotate_files(dir, "log.log", 3)?;
            std::thread::sleep(Duration::from_secs(1));
        }

        assert_eq!(file_count(dir), 3);
        Ok(())
    }

    #[test]
    fn creates_dir_if_missing() -> Result<()> {
        let tmp = temp_dir::TempDir::new()?;
        let nested = tmp.path().join("a").join("b").join("c");

        rotate_files(&nested, "app.log", 2)?;
        assert!(nested.join("app.log").exists());
        Ok(())
    }

    #[test]
    fn renames_active_file_with_timestamp() -> Result<()> {
        let tmp = temp_dir::TempDir::new()?;
        let dir = tmp.path();

        // First rotation: creates active file.
        rotate_files(dir, "app.log", 10)?;
        assert!(dir.join("app.log").exists());

        std::thread::sleep(Duration::from_secs(1));

        // Second rotation: active file gets renamed.
        rotate_files(dir, "app.log", 10)?;

        let files: Vec<_> = fs::read_dir(dir)?
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();

        // There must be exactly one archived file (with timestamp) plus the
        // new active file.
        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|n| n == "app.log"));
        assert!(files.iter().any(|n| n.starts_with("app_") && n != "app.log"));
        Ok(())
    }
}
