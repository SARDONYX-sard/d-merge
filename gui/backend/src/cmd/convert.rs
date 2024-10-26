use super::bail;
use core::str::FromStr as _;
use futures::{future::join_all, stream::FuturesUnordered};
use serde_hkx_features::convert::OutFormat;
use std::path::Path;
use tauri::{Emitter as _, Window};

/// Closure that reports the number of files
macro_rules! sender {
    ($window:ident, $emit_name:literal) => {
        move |payload: Payload| {
            if let Err(err) = $window.emit($emit_name, payload) {
                tracing::error!("{}", err);
            };
        }
    };
}

/// # Progress report for progress bar
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Payload {
    path_id: usize,
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

    let status_sender = sender!(window, "d_merge://progress/convert");

    let tasks = inputs
        .iter()
        .enumerate()
        .map(|(path_id, input)| {
            let input_path = Path::new(input).to_path_buf();
            let input = input.clone();
            let output = output.clone();
            let status_sender = status_sender.clone();

            tokio::spawn(async move {
                status_sender(Payload { path_id, status: 1 }); // 1: processing

                let result = if input_path.is_file() {
                    let output = output.map(|output_dir| {
                        let mut output_dir =
                            Path::new(&output_dir).join(input_path.file_stem().unwrap_or_default());
                        output_dir.set_extension(format.as_extension());
                        output_dir
                    });
                    serde_hkx_features::convert::convert(&input_path, output, format).await
                } else {
                    serde_hkx_features::convert::convert(&input_path, output, format).await
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
                tracing::error!("{:?}", err);
                errs.push(format!("JoinError. id: {:?}", err.id()));
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
