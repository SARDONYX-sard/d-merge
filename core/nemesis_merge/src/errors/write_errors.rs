use crate::errors::{Error, Result};
use rayon::prelude::*;
use std::path::Path;

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

pub(crate) async fn write_errors(path: impl AsRef<Path>, errors: &[Error]) -> Result<()> {
    let _path = path.as_ref();

    let errors = errors_to_string(errors);

    #[cfg(feature = "tracing")]
    tracing::error!("{errors}");

    #[cfg(not(feature = "tracing"))]
    {
        use crate::errors::FailedIoSnafu;
        use snafu::ResultExt as _;

        tokio::fs::write(&_path, errors)
            .await
            .with_context(|_| FailedIoSnafu {
                path: _path.to_path_buf(),
            })?;
    }
    Ok(())
}
