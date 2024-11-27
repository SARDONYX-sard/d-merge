//! errors of `This crate`
use std::{io, path::PathBuf};

/// GUI Error
#[derive(Debug, snafu::Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    /// Failed to read file from {path}
    #[snafu(display("{source}: {}", path.display()))]
    FailedIo { source: io::Error, path: PathBuf },

    /// "Failed to parse path as nemesis path: {}", path.display()
    #[snafu(display("Failed to parse path as nemesis path: {}", path.display()))]
    FailedParseNemesisPath { path: PathBuf },

    /// dir strip error
    #[snafu(transparent)]
    StripPrefixError { source: std::path::StripPrefixError },

    /// jwalk error
    #[snafu(transparent)]
    JwalkErr { source: jwalk::Error },

    /// Nemesis XML parsing error
    #[snafu(display("{}:\n{xml}\n{source}\n---------------------------------------------------------", path.display()))]
    NemesisXmlErr {
        /// input XML
        xml: String,
        /// input path
        path: PathBuf,
        source: nemesis_xml::error::Error,
    },

    #[snafu(transparent)]
    HkxDeError {
        source: serde_hkx::errors::de::Error,
    },

    #[snafu(transparent)]
    JoinError { source: tokio::task::JoinError },

    /// (De)Serialize json error
    #[cfg(feature = "serde")]
    #[snafu(display("{}:\n {source}", path.display()))]
    JsonError {
        /// input path
        path: PathBuf,
        source: simd_json::Error,
    },
}

/// `Result` for this crate.
pub type Result<T, E = Error> = core::result::Result<T, E>;
