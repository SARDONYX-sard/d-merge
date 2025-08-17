pub mod par_walk_dir;
mod status;

use self::status::{Payload, Status};
use super::{bail, sender};
use crate::libs::{hash::hash_djb2, path::infer::generate_output_path};
use core::str::FromStr as _;
use futures::{future::join_all, stream::FuturesUnordered};
use serde_hkx_features::OutFormat;
use std::path::Path;
use tauri::Window;

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

                serde_hkx_features::convert(&input, output, format)
                    .await
                    .map(|_| {
                        status_sender(Payload {
                            path_id,
                            status: Status::Done,
                        });
                    })
                    .map_err(|err| {
                        status_sender(Payload {
                            path_id,
                            status: Status::Error,
                        });
                        let input = input.display();
                        format!("{input}:\n    {err}")
                    })
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
