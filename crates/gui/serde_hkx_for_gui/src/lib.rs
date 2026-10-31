pub mod hash;
pub mod hkanno;
mod par_walk_dir;
pub mod path;
pub mod status;

use std::{future::Future, path::Path, pin::Pin, sync::Arc};

use futures::{future::join_all, stream::FuturesUnordered};
pub use par_walk_dir::{DirEntry, load_dir_node};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use serde_hkx_features::Format;

use self::status::{Payload, Status};
use crate::{hash::hash_djb2, path::infer::generate_output_path};

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
///
/// # Binary size
/// The (de)serialization of every havok class is generic code; whatever instantiates it gets a
/// copy (~15 MB in a non-LTO build). So this function is not generic (`status_sender` is a trait
/// object) and returns a boxed future: a plain `async fn` body would be built in the caller crate.
pub fn convert(
    inputs: Vec<String>,
    output: Option<String>,
    format: Format,
    roots: Option<Vec<String>>,
    status_sender: Arc<dyn Fn(Payload) + Send + Sync>,
) -> Pin<Box<dyn Future<Output = Result<(), ConvertError>> + Send>> {
    Box::pin(convert_inner(inputs, output, format, roots, status_sender))
}

async fn convert_inner(
    inputs: Vec<String>,
    output: Option<String>,
    format: Format,
    roots: Option<Vec<String>>,
    status_sender: Arc<dyn Fn(Payload) + Send + Sync>,
) -> Result<(), ConvertError> {
    let output = output.filter(|o| !o.is_empty());
    let strip_roots = roots.unwrap_or_default();

    let tasks = inputs
        .into_iter()
        .map(|input| {
            let path_id = hash_djb2(&input);
            let input = Path::new(&input).to_path_buf();
            let status_sender = Arc::clone(&status_sender);

            let output = output.as_ref().map(|output| {
                let mut output = generate_output_path(&input, output, &strip_roots);
                if input.is_file() {
                    output.set_extension(format.as_extension());
                }
                output
            });

            tokio::spawn(async move {
                status_sender(Payload { path_id, status: Status::Processing });

                serde_hkx_features::convert(&input, output, format)
                    .await
                    .map(|_| {
                        status_sender(Payload { path_id, status: Status::Done });
                    })
                    .map_err(|err| {
                        status_sender(Payload { path_id, status: Status::Error });

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

    if !errors.is_empty() { Err(ConvertError::Multi { errors }) } else { Ok(()) }
}

#[derive(Debug, snafu::Snafu)]
pub enum ConvertOneError {
    /// serde_hkx conversion error
    #[snafu(display("{}:\n    {source}", input.display()))]
    FailedToConvert { input: std::path::PathBuf, source: serde_hkx_features::error::Error },

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
