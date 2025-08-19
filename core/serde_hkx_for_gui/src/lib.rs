pub mod hash;
pub mod par_walk_dir;
pub mod path;
pub mod status;

use self::status::{Payload, Status};
use crate::{hash::hash_djb2, path::infer::generate_output_path};
use core::str::FromStr as _;
use futures::{future::join_all, stream::FuturesUnordered};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use serde_hkx_features::OutFormat;
use std::path::Path;

/// Converts between HKX and XML (or other supported formats) asynchronously.
///
/// # Returns
///
/// * `Ok(())` if all conversions succeeded.
/// * `Err(ConvertError)` if any conversion failed. Multiple errors are aggregated
///   in the `ConvertError::Multi` variant.
///
/// # Status Updates
///
/// The `status_sender` callback is called with a `Payload` object containing:
/// - `path_id` - A hash identifier of the input path.
/// - `status` - A `Status` enum indicating the current state (`Processing`, `Done`, `Error`).
///
/// # Errors
///
/// Errors are represented by the `ConvertError` enum:
/// - `Multi` - Aggregates multiple `ConvertOneError`s.
/// - `FormatParse` - The provided output format string could not be parsed.
///
/// Each `ConvertOneError` can be:
/// - `FailedToConvert` - The underlying `serde_hkx_features` conversion failed.
/// - `ThreadJoin` - A task panicked or failed to join in the async executor.
pub async fn convert(
    inputs: Vec<String>,
    output: Option<String>,
    format: &str,
    roots: Option<Vec<String>>,
    status_sender: impl Fn(Payload) + Clone + Send + 'static,
) -> Result<(), ConvertError> {
    let format = OutFormat::from_str(format).map_err(|_| ConvertError::FormatParse {
        invalid: format.to_string(),
    })?;
    let output = output.and_then(|o| if o.is_empty() { None } else { Some(o) });
    let strip_roots = roots.unwrap_or_default();

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

                        let input = input;

                        ConvertOneError::FailedToConvert { input, source: err }
                    })
            })
        })
        .collect::<FuturesUnordered<_>>();

    let results = join_all(tasks).await;

    let mut errors: Vec<ConvertOneError> = Vec::new();
    for result in results {
        match result {
            Ok(Ok(())) => (),
            Ok(Err(err)) => {
                errors.push(err);
            }
            Err(err) => {
                errors.push(ConvertOneError::ThreadJoin { id: err.id() });
            }
        }
    }

    if !errors.is_empty() {
        Err(ConvertError::Multi { errors })
    } else {
        Ok(())
    }
}

#[derive(Debug, snafu::Snafu)]
pub enum ConvertOneError {
    /// serde_hkx conversion error
    #[snafu(display("{}:\n    {source}", input.display()))]
    FailedToConvert {
        input: std::path::PathBuf,
        source: serde_hkx_features::error::Error,
    },

    #[snafu(display("Join error id: {id}"))]
    ThreadJoin { id: tokio::task::Id },
}

#[derive(Debug, snafu::Snafu)]
pub enum ConvertError {
    #[snafu(display("{}", errors.into_par_iter().map(|e| e.to_string()).collect::<Vec<String>>().join("\n")))]
    Multi { errors: Vec<ConvertOneError> },

    /// Output format parse error. Accepts 'amd64', 'win32', 'xml', 'json', 'toml'. but got {invalid}
    FormatParse { invalid: String },
}
