use crate::errors::{Error, Result};
use rayon::prelude::*;
use std::path::Path;

pub(crate) async fn write_errors(path: impl AsRef<Path>, errors: &[Error]) -> Result<()> {
    let _path = path.as_ref();

    let err_len = errors.len();
    let errors: Vec<String> = errors
        .into_par_iter()
        .enumerate()
        .map(|(index, err)| {
            let index = index + 1;
            format!("[Error {index}/{err_len}] {err}")
        })
        .collect();

    #[cfg(feature = "tracing")]
    tracing::error!("{}", errors.join("\n\n"));
    #[cfg(not(feature = "tracing"))]
    {
        use crate::errors::FailedIoSnafu;
        use snafu::ResultExt as _;

        tokio::fs::write(&_path, errors.join("\n\n"))
            .await
            .with_context(|_| FailedIoSnafu {
                path: _path.to_path_buf(),
            })?;
    }
    Ok(())
}
