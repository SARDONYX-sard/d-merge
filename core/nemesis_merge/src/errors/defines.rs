//! errors of `This crate`
use std::{io, path::PathBuf};

/// `nemesis_merge` Error
#[derive(Debug, snafu::Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    /// {msg}
    Custom { msg: String },

    /// Failed to read file from {path}
    #[snafu(display("{source}: {}", path.display()))]
    FailedIo { source: io::Error, path: PathBuf },

    /// Failed to read owned patches.(errors count: {errors_len})
    FailedToReadOwnedPatches { errors_len: usize },

    /// Failed to read borrowed patches.(errors count: {errors_len})
    FailedToReadBorrowedPatches { errors_len: usize },

    /// Failure to read XML templates converted from patches and hkx.(error count: {errors_len})
    FailedToReadTemplates { errors_len: usize },

    /// - Json patch error count: {patch_errors_len}
    /// - Failure to apply `patch -> XML template`.(error count: {apply_errors_len})
    /// - Failed to generate hkx of XML templates.(error count: {hkx_errors_len})
    #[snafu(display("- json patch error count: {patch_errors_len}\n- Apply json patch error count: {apply_errors_len}\n- Generate hkx error count: {hkx_errors_len}"))]
    FailedToGenerateBehaviors {
        hkx_errors_len: usize,
        patch_errors_len: usize,
        apply_errors_len: usize,
    },

    /// No such template `{template_name}`.
    NotFoundTemplate { template_name: String },

    /// Json patch error
    #[snafu(display("{template_name}:\n {source}\n patch: {patch}"))]
    PatchError {
        source: json_patch::JsonPatchError,
        template_name: String,
        patch: String,
    },

    /// Nemesis XML parsing error
    #[snafu(display("{}:\n{source}\n", path.display()))]
    NemesisXmlErr {
        /// input path
        path: PathBuf,
        source: nemesis_xml::error::Error,
    },

    /// Failed to parse path {}
    #[snafu(display("Failed to parse path: {}", path.display()))]
    MissingParseNemesisPath { path: PathBuf },

    /// Failed to parse path as nemesis path
    #[snafu(transparent)]
    FailedParseNemesisPath {
        source: crate::paths::parse::NemesisPathError,
    },

    /// dir strip error
    #[snafu(transparent)]
    StripPrefixError { source: std::path::StripPrefixError },

    /// jwalk error
    #[snafu(transparent)]
    JwalkErr { source: jwalk::Error },

    #[snafu(transparent)]
    ParsedAdsfPathError {
        source: crate::adsf::path_parser::ParseError,
    },

    /// Failed to parse adsf template
    #[snafu(display("[animationdatasinglefile template Parse Error]{}:\n {source}", path.display()))]
    FailedParseAdsfTemplate {
        source: serde_hkx::errors::readable::ReadableError,
        path: PathBuf,
    },

    /// Failed to parse adsf patch
    #[snafu(display("[animationdatasinglefile patch Parse Error]{}:\n {source}", path.display()))]
    FailedParseAdsfPatch {
        source: serde_hkx::errors::readable::ReadableError,
        path: PathBuf,
    },

    /// serde_hkx serialize error.
    #[snafu(display("{}:\n {source}", path.display()))]
    HkxSerError {
        path: PathBuf,
        source: serde_hkx::errors::ser::Error,
    },

    #[snafu(transparent)]
    HkxDeError {
        source: serde_hkx::errors::de::Error,
    },

    #[snafu(transparent)]
    HkxError {
        source: serde_hkx_features::error::Error,
    },

    #[snafu(transparent)]
    JoinError { source: tokio::task::JoinError },

    /// (De)Serialize json error
    #[snafu(display("{}:\n {source}", path.display()))]
    JsonError {
        /// input path
        path: PathBuf,
        source: simd_json::Error,
    },
}

/// `Result` for this crate.
pub type Result<T, E = Error> = core::result::Result<T, E>;
