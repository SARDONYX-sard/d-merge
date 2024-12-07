pub mod par_walk_dir;
mod status;

use self::status::{Payload, Status};
use super::{bail, sender};
use crate::libs::{hash::hash_djb2, path::infer::generate_output_path};
use core::str::FromStr as _;
use futures::{future::join_all, stream::FuturesUnordered};
use serde_hkx_features::convert::{convert as serde_hkx_convert, OutFormat};
use std::path::Path;
use tauri::Window;

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
