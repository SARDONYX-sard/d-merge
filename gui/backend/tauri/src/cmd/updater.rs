use std::io;
use std::path::Path;

const OWNER_NAME: &str = "SARDONYX-sard";
const REPO_NAME: &str = "d-merge";
const BIN_NAME: &str = "d_merge";

#[tauri::command]
pub async fn fetch_versions() -> Result<Vec<String>, String> {
    // NOTE: self_update uses `requwest` crate, and there is an issue where it does not function properly
    //       unless it is used within spawn_blocking in an asynchronous function.
    tauri::async_runtime::spawn_blocking(move || {
        let releases = self_update::backends::github::ReleaseList::configure()
            .repo_owner(OWNER_NAME)
            .repo_name(REPO_NAME)
            .build()
            .map_err(|e| format!("Build error: {e}"))?
            .fetch()
            .map_err(|e| format!("Fetch error: {e}"))?;

        Ok(releases.iter().map(|r| r.version.clone()).collect())
    })
    .await
    .map_err(|e| format!("Thread error: {e}"))?
}

#[tauri::command]
pub async fn update_to_version(version: String) -> Result<String, String> {
    use snafu::{OptionExt, ResultExt};

    tauri::async_runtime::spawn_blocking(move || -> Result<String, UpdaterError> {
        let release = self_update::backends::github::Update::configure()
            .repo_owner(OWNER_NAME)
            .repo_name(REPO_NAME)
            .bin_name(BIN_NAME)
            .target_version_tag(&version)
            .no_confirm(true)
            .build()
            .context(BuildSnafu)?;

        let archive_path = release.bin_path_in_archive();
        release.update().context(UpdateSnafu)?;

        let extractor = self_update::Extract::from_source(Path::new(&archive_path));
        let tmp_dir = self_update::TempDir::new().context(IoSnafu)?;
        extractor
            .extract_into(tmp_dir.path())
            .context(ExtractSnafu)?;

        // Copy assets, interface
        let current_dir = std::env::current_exe()
            .context(IoSnafu)?
            .parent()
            .context(OtherSnafu {
                msg: "No parent directory".to_string(),
            })?
            .to_path_buf();

        for name in &["assets", "interface"] {
            let src = tmp_dir.path().join(name);
            let dst = current_dir.join(name);
            let _ = std::fs::remove_dir_all(&dst);
            copy_dir_all(&src, &dst).context(CopySnafu { name: *name })?;
        }

        Ok(version)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

/// - ref: https://stackoverflow.com/questions/26958489/how-to-copy-a-folder-recursively-in-rust
fn copy_dir_all<S, D>(src: S, dst: D) -> std::io::Result<()>
where
    S: AsRef<Path>,
    D: AsRef<Path>,
{
    use std::fs;
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

#[derive(Debug, snafu::Snafu)]
pub enum UpdaterError {
    #[snafu(display("Failed to build updater: {}", source))]
    BuildError { source: self_update::errors::Error },

    #[snafu(display("Failed to update binary: {}", source))]
    UpdateError { source: self_update::errors::Error },

    #[snafu(display("Failed to extract archive: {}", source))]
    ExtractError { source: self_update::errors::Error },

    #[snafu(display("I/O error: {}", source))]
    IoError { source: io::Error },

    #[snafu(display("Failed to copy {}: {}", name, source))]
    CopyError {
        name: &'static str,
        source: io::Error,
    },

    #[snafu(display("Other error: {}", msg))]
    Other { msg: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[ignore]
    #[tokio::test]
    async fn test_fetch_versions_returns_versions() {
        let result = fetch_versions().await;
        assert!(result.is_ok());

        let versions = result.unwrap();
        println!("Versions: {versions:?}");
    }
}
