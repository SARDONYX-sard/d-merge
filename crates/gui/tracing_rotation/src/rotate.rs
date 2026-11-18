// SPDX-License-Identifier: MIT

//! Rotation logic: rename the current log file with a timestamp suffix and
//! delete the oldest files if the cap is exceeded.

use std::{
    fs::{self, DirEntry, File},
    path::Path,
    time::SystemTime,
};

use jiff::ZonedDateTime;

use crate::error::Result;

/// Create a new, empty log file at `log_dir/file_name`, rotating existing
/// files out of the way.
///
/// # Errors
/// Propagates any [`std::io::Error`] encountered during directory creation,
/// file renaming, deletion, or file creation.
pub(crate) fn rotate_files(log_dir: impl AsRef<Path>, file_name: &str, max_files: usize) -> Result<File> {
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

    let active_path = log_dir.join(file_name);

    if active_path.exists() {
        let timestamp = ZonedDateTime::now().strftime("%F_%H-%M-%S");
        let archived = log_dir.join(format!("{stem}_{timestamp}.{ext}"));

        fs::rename(&active_path, archived)?;
    }

    Ok(File::create(active_path)?)
}

fn modification_time(entry: &DirEntry) -> Option<SystemTime> {
    entry.metadata().ok()?.modified().ok()
}

#[cfg(test)]
mod tests {
    use std::{fs, time::Duration};

    use pretty_assertions::assert_eq;

    use super::*;

    /// A new empty directory of the test's own under the system's temp directory, removed when
    /// dropped: the tests run at once, and the temp directory has files of others.
    struct TempDir(std::path::PathBuf);

    impl TempDir {
        fn new(test: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("tracing_rotation_{test}_{}", std::process::id()));
            let _ = fs::remove_dir_all(&dir); // left by a run that stopped
            fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }

        fn path(&self) -> &std::path::Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn file_count(dir: &std::path::Path) -> usize {
        fs::read_dir(dir).unwrap().filter_map(|e| e.ok()).filter(|e| e.file_type().is_ok_and(|t| !t.is_dir())).count()
    }

    #[test]
    fn caps_at_max_files() -> Result<()> {
        let temp_dir = TempDir::new("caps_at_max_files");
        let temp_dir = temp_dir.path();

        for _ in 0..6 {
            rotate_files(temp_dir, "log.log", 3)?;
            std::thread::sleep(Duration::from_secs(1));
        }

        assert_eq!(file_count(temp_dir), 3);
        Ok(())
    }

    #[test]
    fn creates_dir_if_missing() -> Result<()> {
        let tmp = TempDir::new("creates_dir_if_missing");
        let nested = tmp.path().join("a").join("b").join("c");

        rotate_files(&nested, "app.log", 2)?;
        assert!(nested.join("app.log").exists());
        Ok(())
    }

    #[test]
    fn renames_active_file_with_timestamp() -> Result<()> {
        let temp_dir = TempDir::new("renames_active_file_with_timestamp");
        let temp_dir = temp_dir.path();

        rotate_files(temp_dir, "app.log", 10)?;
        assert!(temp_dir.join("app.log").exists());

        std::thread::sleep(Duration::from_secs(1));

        rotate_files(temp_dir, "app.log", 10)?;

        let files: Vec<_> = fs::read_dir(temp_dir)?
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();

        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|n| n == "app.log"));
        assert!(files.iter().any(|n| n.starts_with("app_") && n != "app.log"));
        Ok(())
    }
}
