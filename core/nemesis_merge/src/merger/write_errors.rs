#[cfg(not(feature = "tracing"))]
use crate::error::FailedIoSnafu;
use crate::error::{Error, Result};
use rayon::prelude::*;
#[cfg(not(feature = "tracing"))]
use snafu::ResultExt as _;
use std::path::Path;

pub async fn write_errors(path: impl AsRef<Path>, errors: &[Error]) -> Result<()> {
    let _path = path.as_ref();

    let errors: Vec<String> = errors.into_par_iter().map(|e| e.to_string()).collect();
    #[cfg(feature = "tracing")]
    tracing::error!("{}", errors.join("\n\n"));
    #[cfg(not(feature = "tracing"))]
    tokio::fs::write(&_path, errors.join("\n\n"))
        .await
        .context(FailedIoSnafu {
            path: _path.to_path_buf(),
        })?;
    Ok(())
}
