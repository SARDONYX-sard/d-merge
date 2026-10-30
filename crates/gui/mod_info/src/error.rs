//! errors of `This crate`
use std::{io, path::PathBuf};

/// GUI Error
#[derive(Debug, snafu::Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    /// Failed to read file from
    #[snafu(display("[I/O Error]{}: {source}", path.display()))]
    IoError { source: io::Error, path: PathBuf },

    #[allow(clippy::use_self)]
    #[snafu(display("{}", errors.iter().map(|e| format!("- {e}")).collect::<Vec<_>>().join("\n")))]
    Nested { errors: Vec<Error> },
}
