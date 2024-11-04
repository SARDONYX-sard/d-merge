use super::{bail, sender};
use core::str::FromStr as _;
use d_merge_core::path_node::{build_dir_tree, DirEntry};
use futures::{future::join_all, stream::FuturesUnordered};
use rayon::prelude::*;
use serde_hkx_features::convert::{convert as serde_hkx_convert, OutFormat};
use std::path::{Path, PathBuf};
use tauri::Window;

// /////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

/// Loads a directory structure from the specified path, filtering by allowed extensions.
///
/// # Errors
/// Returns an error message if the directory cannot be loaded or if there are issues reading the path.
#[tauri::command]
pub(crate) fn load_dir_node(dirs: Vec<String>) -> Result<Vec<DirEntry>, String> {
    #[cfg(feature = "extra_fmt")]
    const FILTER: [&str; 4] = ["hkx", "xml", "json", "yaml"];
    #[cfg(not(feature = "extra_fmt"))]
    const FILTER: [&str; 2] = ["hkx", "xml"];

    let (entries, errors): (Vec<_>, Vec<_>) = dirs
        .par_iter()
        .map(|dir| build_dir_tree(dir, FILTER).or_else(|err| bail!(err)))
        .partition(Result::is_ok);

    // Collect only successful entries
    let entries: Vec<DirEntry> = entries.into_iter().map(Result::unwrap).collect();

    // Collect error messages and join them
    if !errors.is_empty() {
        let error_messages: Vec<String> = errors.into_iter().map(Result::unwrap_err).collect();
        return Err(error_messages.join("\n"));
    }

    Ok(entries)
}

/// Whether the converter supports json and yaml conversion as well?
#[tauri::command]
pub(crate) const fn is_supported_extra_fmt() -> bool {
    #[cfg(feature = "extra_fmt")]
    const RET: bool = true;
    #[cfg(not(feature = "extra_fmt"))]
    const RET: bool = false;

    RET
}

// /////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, serde_repr::Serialize_repr, serde_repr::Deserialize_repr)]
#[repr(u8)]
enum Status {
    Pending = 0,
    Processing = 1,
    Done = 2,
    Error = 3,
}

/// # Progress report for progress bar
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Payload {
    /// hashed path
    path_id: u32,
    /// Current progress status
    status: Status,
}

/// Convert hkx <-> xml
/// -
#[tauri::command]
pub(crate) async fn convert(
    window: Window,
    inputs: Vec<String>,
    output: Option<String>,
    format: &str,
    roots: Option<Vec<String>>,
) -> Result<(), String> {
    let format = OutFormat::from_str(format).or_else(|err| bail!(err))?;
    let output = output.and_then(|o| if o.is_empty() { None } else { Some(o) });
    let strip_roots = roots.unwrap_or_default();

    let status_sender = sender(window, "d_merge://progress/convert");

    let tasks = inputs
        .into_iter()
        .map(|input| {
            let path_id = hash_djb2(&input);
            let input = Path::new(&input).to_path_buf();
            let status_sender = status_sender.clone();

            let output = output.as_ref().map(|output| {
                let mut output = generate_output_path(&input, output, &strip_roots);
                if input.is_file() {
                    output.set_extension(format.as_extension());
                }
                output
            });

            tokio::spawn(async move {
                status_sender(Payload {
                    path_id,
                    status: Status::Processing,
                });

                match serde_hkx_convert(&input, output, format).await {
                    Ok(_) => {
                        status_sender(Payload {
                            path_id,
                            status: Status::Done,
                        });
                        Ok(())
                    }
                    Err(err) => {
                        status_sender(Payload {
                            path_id,
                            status: Status::Error,
                        });
                        let input = input.display();
                        Err(format!("{input}:\n    {err}"))
                    }
                }
            })
        })
        .collect::<FuturesUnordered<_>>();

    let results: Vec<Result<Result<(), String>, tokio::task::JoinError>> = join_all(tasks).await;

    let mut errs = Vec::new();
    for result in results {
        match result {
            Ok(Ok(())) => (),
            Ok(Err(err)) => {
                tracing::error!("{err}");
                errs.push(err);
            }
            Err(err) => {
                tracing::error!("{err}");
                errs.push(format!("JoinError. id: {}", err.id()));
            }
        }
    }

    if !errs.is_empty() {
        let errs = errs.join("\n");
        tracing::error!("{errs}");
        Err(errs)
    } else {
        Ok(())
    }
}

/// # Why use this?
/// The frontend selection can be deleted.
/// Therefore, the conversion status shifts when using index.
/// So, using hash from path solves this problem.
/// The exact same hash function is implemented in frontend and tested.
const fn hash_djb2(key: &str) -> u32 {
    let mut hash: u32 = 5381;
    let bytes = key.as_bytes();
    let mut i = 0;

    // NOTE: For const, it is necessary to loop with while instead of using for loop(iter).
    while i < bytes.len() {
        hash = ((hash << 5).wrapping_add(hash)) ^ (bytes[i] as u32);
        i += 1;
    }
    hash
}

/// Generate an output path based on the roots_path and input_path.
/// If input_path starts with any of the roots_path entries, the relative path
/// will be used. Otherwise, only the file stem will be appended.
fn generate_output_path<P>(input: P, output_dir: &str, strip_roots: &[String]) -> PathBuf
where
    P: AsRef<Path>,
{
    let input_path = input.as_ref();

    strip_roots
        .iter()
        .find(|root| input_path.starts_with(root))
        .map_or_else(
            || {
                let file_name = input_path.file_stem().unwrap_or_default();
                Path::new(output_dir).join(file_name)
            },
            |root| {
                let relative_path = input_path.strip_prefix(root).unwrap();
                Path::new(output_dir).join(relative_path)
            },
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consistent_hash() {
        let input = "example";
        let hash1 = hash_djb2(input);
        let hash2 = hash_djb2(input);
        assert_eq!(
            hash1, hash2,
            "Different hash values were generated for the same input"
        );
    }

    #[test]
    fn test_different_hashes_for_different_inputs() {
        let hash1 = hash_djb2("example1");
        let hash2 = hash_djb2("example2");
        assert_ne!(
            hash1, hash2,
            "Same hash values were generated for different inputs"
        );
    }

    #[test]
    fn test_empty_string() {
        let hash = hash_djb2("");
        assert_eq!(
            hash, 5381,
            "Hash for empty string does not match the expected initial value"
        );
    }
}

#[cfg(test)]
mod path_tests {
    use super::*;

    #[test]
    fn test_generate_output_path_with_root_match() {
        let output_dir = "output";
        let input = Path::new("root_dir/sub_dir/sub_dir2");
        let roots_path = vec!["root_dir".to_string()];
        let result = generate_output_path(input, output_dir, &roots_path);
        assert_eq!(result, Path::new("output/sub_dir/sub_dir2"));
    }

    #[test]
    fn test_generate_output_path_without_root_match() {
        let output_dir = "output";
        let input = Path::new("other_dir/file.hkx");
        let roots_path = vec!["root_dir".to_string()];

        let mut result = generate_output_path(input, output_dir, &roots_path);
        result.set_extension("xml");
        assert_eq!(result, Path::new("output/file.xml"));
    }

    #[test]
    fn test_generate_output_path_multiple_roots() {
        let output_dir = "output";
        let input = Path::new("another_root/sub_dir/file.hkx");
        let roots_path = vec!["root_dir".to_string(), "another_root".to_string()];

        let mut result = generate_output_path(input, output_dir, &roots_path);
        result.set_extension("xml");
        assert_eq!(result, Path::new("output/sub_dir/file.xml"));
    }
}
