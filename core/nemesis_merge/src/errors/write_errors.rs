use crate::errors::{Error, Result};
use rayon::prelude::*;

fn errors_to_string(errors: &[Error]) -> String {
    let err_len = errors.len();
    let errors: Vec<String> = errors
        .into_par_iter()
        .enumerate()
        .map(|(index, err)| {
            let index = index + 1;
            format!("[Error {index}/{err_len}] {err}")
        })
        .collect();

    errors.join("\n\n")
}

/// Writes errors to a log file or prints them to the console based on the feature flag.
///
/// - If the `tracing` feature is enabled, it logs the errors using the `tracing` crate.
/// - Otherwise, it writes the errors to a file named `d_merge_errors.log` in the output directory.
pub(crate) async fn write_errors(_options: &crate::Config, errors: &[Error]) -> Result<()> {
    let errors = errors_to_string(errors);

    #[cfg(feature = "tracing")]
    tracing::error!("{errors}");

    // #[cfg(not(feature = "tracing"))]
    {
        use crate::errors::FailedIoSnafu;
        use snafu::ResultExt as _;
        use tokio::fs;

        let mut error_output = _options.output_dir.join(".debug");
        let _ = fs::create_dir_all(&error_output).await;
        error_output.push("d_merge_errors.log");
        fs::write(&error_output, errors)
            .await
            .with_context(|_| FailedIoSnafu { path: error_output })?;
    }

    Ok(())
}
