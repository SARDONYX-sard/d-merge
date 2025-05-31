//! errors of `This crate`
use serde_hkx::errors::readable::ReadableError;
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
    #[snafu(display(
        "
- `animationdatasinglefile.txt` Error Count: {adsf_errors_len}

-       Json Patch Error Count: {patch_errors_len}
- Apply Json Patch Error Count: {apply_errors_len}
-     Generate hkx Error Count: {hkx_errors_len}"
    ))]
    FailedToGenerateBehaviors {
        adsf_errors_len: usize,
        patch_errors_len: usize,
        apply_errors_len: usize,
        hkx_errors_len: usize,
    },

    /// No such template `{template_name}`.
    NotFoundTemplate { template_name: String },

    /// Json patch error
    #[snafu(display("[Apply patch Error to `{template_name}`]:\n {source}\n patch: {patch}"))]
    PatchError {
        source: json_patch::JsonPatchError,
        template_name: String,
        patch: String,
    },

    /// Nemesis XML parsing error
    #[snafu(display("[Nemesis XML Patch Parsing Error `{}`]:\n{source}\n", path.display()))]
    NemesisXmlErr {
        /// input path
        path: PathBuf,
        source: nemesis_xml::error::Error,
    },

    /// Failed to parse path {}
    #[snafu(display("Failed to parse path: {}", path.display()))]
    MissingParseNemesisPath { path: PathBuf },

    /// Failed to parse adsf template
    #[snafu(display("[animationdatasinglefile template Parse Error]{}:\n {source}", path.display()))]
    FailedParseAdsfTemplate {
        source: ReadableError,
        path: PathBuf,
    },

    /// Failed to parse adsf patch
    #[snafu(display("[animationdatasinglefile patch Parse Error]{}:\n {source}", path.display()))]
    FailedParseAdsfPatch {
        source: ReadableError,
        path: PathBuf,
    },

    /// serde_hkx serialize error.
    #[snafu(display("{}:\n {source}", path.display()))]
    HkxSerError {
        path: PathBuf,
        source: serde_hkx::errors::ser::Error,
    },

    /// (De)Serialize json error
    #[snafu(display("{}:\n {source}", path.display()))]
    JsonError {
        /// input path
        path: PathBuf,
        source: simd_json::Error,
    },

    /// Path must be utf-8.
    #[snafu(display("Expected utf-8 path. but got {}", path.display()))]
    NonUtf8Path { path: PathBuf },

    /// Failed to parse path as nemesis path
    FailedParseNemesisPath { source: ReadableError },

    #[snafu(transparent)]
    ParsedAdsfPathError {
        source: crate::adsf::path_parser::ParseError,
    },

    /// dir strip error
    #[snafu(transparent)]
    StripPrefixError { source: std::path::StripPrefixError },

    /// jwalk error
    #[snafu(transparent)]
    JwalkErr { source: jwalk::Error },

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
}

/// `Result` for this crate.
pub type Result<T, E = Error> = core::result::Result<T, E>;
