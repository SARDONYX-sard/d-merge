//! errors of `This crate`
use std::{io, path::PathBuf};

/// GUI Error
#[derive(Debug, snafu::Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    /// Failed to read file from
    #[snafu(display("[I/O Error]{}: {source}", path.display()))]
    IoError { source: io::Error, path: PathBuf },
}

/// `Result` for this crate.
pub type Result<T, E = Error> = core::result::Result<T, E>;
