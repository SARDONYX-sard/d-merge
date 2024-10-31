use super::{bail, sender};
use core::str::FromStr as _;
use futures::{future::join_all, stream::FuturesUnordered};
use serde_hkx_features::convert::{convert as serde_hkx_convert, OutFormat};
use std::path::Path;
use tauri::Window;

/// # Progress report for progress bar
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Payload {
    path_id: u32,
    /// 0: pending, 1: processing, 2: done, 3: error
    status: u8,
}

/// Convert
#[tauri::command]
pub(crate) async fn convert(
    window: Window,
    inputs: Vec<String>,
    output: Option<String>,
    format: &str,
) -> Result<(), String> {
    let format = OutFormat::from_str(format).or_else(|err| bail!(err))?;
    let output = output.and_then(|o| if o.is_empty() { None } else { Some(o) });

    let status_sender = sender(window, "d_merge://progress/convert");

    let tasks = inputs
        .into_iter()
        .map(|input| {
            let path_id = hash_djb2(&input);
            let input_path = Path::new(&input).to_path_buf();
            let output = output.clone();
            let status_sender = status_sender.clone();

            tokio::spawn(async move {
                status_sender(Payload { path_id, status: 1 }); // 1: processing

                let result = if input_path.is_file() {
                    let output = output.map(|output_dir| {
                        let file_name = input_path.file_stem().unwrap_or_default();
                        let mut output_file = Path::new(&output_dir).join(file_name);
                        output_file.set_extension(format.as_extension());
                        output_file
                    });
                    serde_hkx_convert(&input_path, output, format).await
                } else {
                    serde_hkx_convert(&input_path, output, format).await
                };

                match result {
                    Ok(_) => {
                        status_sender(Payload { path_id, status: 2 }); // 2: done
                        Ok(())
                    }
                    Err(err) => {
                        status_sender(Payload { path_id, status: 3 }); // 3: error
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
fn hash_djb2(key: &str) -> u32 {
    let mut hash: u32 = 5381;
    for byte in key.as_bytes() {
        hash = ((hash << 5).wrapping_add(hash)) ^ u32::from(*byte);
    }
    hash
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
