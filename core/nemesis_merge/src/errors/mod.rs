//! errors of `This crate`
pub mod writer;

use serde_hkx::errors::readable::ReadableError;
use std::{io, path::PathBuf};

/// `nemesis_merge` Error
#[derive(Debug, snafu::Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    /// {msg}
    Custom { msg: String },

    /// Applying the FNIS mod patch requires input for config.skyrim_data_dir_glob, but it is not provided.
    MissingSkyrimDataDirGlob,

    #[snafu(transparent)]
    FnisError {
        source: crate::behaviors::tasks::fnis::collect::owned::FnisError,
    },

    /// Failed to parse FNIS_*_List.txt file.
    #[snafu(display("[FNIS_*_List.txt file Parse Error]{}:\n{source}", path.display()))]
    FailedParseFnisModList {
        source: ReadableError,
        path: PathBuf,
    },

    #[snafu(display(
        "While attempting to automatically convert FNIS HKX for target {target:?}, \
        failed to read file {}: {source}. \
        Please ensure the file exists and is readable.",
        input_path.display()
    ))]
    FNISHkxIoError {
        input_path: PathBuf,
        target: serde_hkx_features::OutFormat,
        source: io::Error,
    },

    #[snafu(display(
        "While attempting to automatically convert FNIS HKX for target {target:?}, \
        the file {} did not have the expected Havok magic numbers. \
        Expected magic=[0x57, 0xE0, 0xE0, 0x57, 0x10, 0xC0, 0xC0, 0x10, ...], \
        but got {magic_bytes:?}. \
        This file is not a valid Havok animation or may be from an unsupported version.",
        input_path.display()
    ))]
    FNISHkxInvalidMagic {
        input_path: PathBuf,
        target: serde_hkx_features::OutFormat,
        magic_bytes: [u8; 16],
    },

    #[snafu(display(
        "While attempting to automatically convert FNIS HKX for target {target:?}, \
        pointer size check failed for {}. \
        Expected pointer size {expected}-byte for {target:?}, \
        but could not determine a valid header or got {actual}-byte. \
        The HKX may be malformed or from an incompatible platform.",
        input_path.display()
    ))]
    FNISHkxInvalidHeader {
        input_path: PathBuf,
        target: serde_hkx_features::OutFormat,
        expected: u8,
        actual: u8,
    },

    #[snafu(display("The conversion of FNIS's hkx failed. input_path: {}, why: {source}", input_path.display()))]
    FNISHkxConversionError {
        input_path: PathBuf,
        source: serde_hkx_features::error::Error,
    },

    #[snafu(transparent)]
    FnisPatchGenerationError {
        source: crate::behaviors::tasks::fnis::patch_gen::FnisPatchGenerationError,
    },

    /// Failed to read file from {path}
    #[snafu(display("{source}: {}", path.display()))]
    FailedIo { source: io::Error, path: PathBuf },

    /// Reading file Error count: {errors_len}
    FailedToReadOwnedPatches { errors_len: usize },

    /// Failed to read borrowed patches.(errors count: {errors_len})
    FailedToReadBorrowedPatches { errors_len: usize },

    /// Failure to read XML templates converted from patches and hkx.(error count: {errors_len})
    FailedToReadTemplates { errors_len: usize },

    /// - Json patch error count: {patch_errors_len}
    /// - Failure to apply `patch -> XML template`.(error count: {apply_errors_len})
    /// - Failed to generate hkx of XML templates.(error count: {hkx_errors_len})
    #[snafu(display("{}", source))]
    FailedToGenerateBehaviors { source: BehaviorGenerationError },

    /// [Apply patch Error] No such template `{template_name}`.
    NotFoundTemplate { template_name: String },

    /// Json patch error
    #[snafu(display("[Apply patch Error to template file(`{template_name}`)]\n{source}\n"))]
    PatchError {
        template_name: String,
        source: json_patch::JsonPatchError,
    },

    /// Nemesis XML parsing error
    #[snafu(display("[Nemesis XML Patch Parsing Error `{}`]:\n{source}\n", path.display()))]
    NemesisXmlErr {
        /// input path
        path: PathBuf,
        source: nemesis_xml::error::Error,
    },

    /// Failed to parse adsf template
    #[snafu(display("[animationdatasinglefile template Parse Error]{}:\n{source}", path.display()))]
    FailedParseAdsfTemplate {
        source: rmp_serde::decode::Error,
        path: PathBuf,
    },

    /// Failed to diff line patch error
    #[snafu(display("[{} -> {} patch Parse Error]{}:\n{source}", kind.as_str(), sub_kind.as_str(), path.display()))]
    FailedSerialize {
        source: skyrim_anim_parser::adsf::alt::ser::SerializeError,
        kind: AnimPatchErrKind,
        sub_kind: AnimPatchErrSubKind,
        path: PathBuf,
    },

    /// Failed to diff line patch error
    #[snafu(display("[{} -> {} patch Parse Error]{}:\n{source}", kind.as_str(), sub_kind.as_str(), path.display()))]
    FailedDiffLinesPatch {
        source: skyrim_anim_parser::diff_line::error::Error,
        kind: AnimPatchErrKind,
        sub_kind: AnimPatchErrSubKind,
        path: PathBuf,
    },

    /// Failed to parse adsf patch
    #[snafu(display("[animationdatasinglefile anim_data_header(Replace/Remove) patch Parse Error]{}:\n{source}", path.display()))]
    FailedParseAdsfAnimDataHeaderPatch {
        source: skyrim_anim_parser::adsf::patch::de::error::Error,
        path: PathBuf,
    },

    /// Failed to parse adsf patch
    #[snafu(display("[animationdatasinglefile edit(Replace/Remove) patch Parse Error]{}:\n{source}", path.display()))]
    FailedParseEditAdsfPatch {
        source: skyrim_anim_parser::adsf::patch::de::error::Error,
        path: PathBuf,
    },

    /// Failed to parse adsf patch
    #[snafu(display("[animationsetdatasinglefile edit(Replace/Remove) patch Parse Error]{}:\n{source}", path.display()))]
    FailedParseEditAsdsfPatch {
        source: skyrim_anim_parser::asdsf::patch::de::error::Error,
        path: PathBuf,
    },

    /// Failed to parse adsf patch
    #[snafu(display("[animationdatasinglefile add patch Parse Error]{}:\n{source}", path.display()))]
    FailedParseAdsfPatch {
        source: ReadableError,
        path: PathBuf,
    },

    /// hkbBehaviorGraphData/hkbBehaviorGraphStringData missing or length check error.
    #[snafu(display("{}:\n {source}", path.display()))]
    DedupEventVariableError {
        path: PathBuf,
        source: serde_hkx_features::id_maker::DedupError,
    },

    /// serde_hkx serialize error.
    #[snafu(display("{}:\n {source}", path.display()))]
    HkxSerError {
        path: PathBuf,
        source: serde_hkx::errors::ser::Error,
    },

    /// Deserialize template error
    #[snafu(display("[hkx template Parsing Error]{}:\n{source}", path.display()))]
    TemplateError {
        /// input path
        path: PathBuf,
        source: rmp_serde::decode::Error,
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

    /// Unsupported Template path
    #[snafu(display("Expected `.bin`, `.xml` extension template file. but got {}", path.display()))]
    UnsupportedTemplatePath { path: PathBuf },

    /// Failed to parse path as nemesis path
    #[snafu(display("Failed to parse path as nemesis path:\n{source}"))]
    FailedParseNemesisPatchPath { source: ReadableError },

    /// Failed to parse path as nemesis path
    #[snafu(display("No template matching this path was found.: {}", path.display()))]
    FailedToCastNemesisPathToTemplateKey { path: PathBuf },

    #[snafu(transparent)]
    ParsedAdsfPathError {
        source: crate::behaviors::AsdfPathParseError,
    },

    #[snafu(transparent)]
    ParsedAsdsfPathError {
        source: crate::behaviors::AsdsfPathParseError,
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

#[derive(Debug, Clone)]
pub enum AnimPatchErrKind {
    Adsf,
    Asdsf,
}

impl AnimPatchErrKind {
    const fn as_str(&self) -> &'static str {
        match self {
            Self::Adsf => "animationdatasinglefile",
            Self::Asdsf => "animationsetdatasinglefile",
        }
    }
}

#[derive(Debug, Clone)]
pub enum AnimPatchErrSubKind {
    /// kind: adsf
    ProjectNamesHeader,
    /// kind: adsf
    AnimDataHeader,

    /// kind: asdsf
    TxtProjectHeader,
}

impl AnimPatchErrSubKind {
    const fn as_str(&self) -> &'static str {
        match self {
            Self::ProjectNamesHeader => "project names header",
            Self::AnimDataHeader => "anim header",
            Self::TxtProjectHeader => "txt project header",
        }
    }
}

#[derive(Debug, Clone)]
pub struct BehaviorGenerationError {
    pub fnis_errors_errors_len: usize,
    pub owned_file_errors_len: usize,
    pub adsf_errors_len: usize,
    pub asdsf_errors_len: usize,
    pub patch_errors_len: usize,
    pub apply_errors_len: usize,
    pub hkx_errors_len: usize,
}

impl core::fmt::Display for BehaviorGenerationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let Self {
            fnis_errors_errors_len: fnis_errors_len,
            owned_file_errors_len,
            adsf_errors_len,
            asdsf_errors_len,
            patch_errors_len,
            apply_errors_len,
            hkx_errors_len,
        } = *self;

        if adsf_errors_len == 0
            && fnis_errors_len == 0
            && asdsf_errors_len == 0
            && owned_file_errors_len == 0
            && patch_errors_len == 0
            && apply_errors_len == 0
            && hkx_errors_len == 0
        {
            return write!(f, "No errors.");
        }

        writeln!(f, "Behavior generation failed with the following errors:")?;
        if fnis_errors_len > 0 {
            writeln!(
                f,
                "-    Generating FNIS patch Error count: {fnis_errors_len}",
            )?;
        }
        if owned_file_errors_len > 0 {
            writeln!(f, "-    Reading file Error count: {owned_file_errors_len}",)?;
        }
        if adsf_errors_len > 0 {
            writeln!(
                f,
                "- `animationdatasinglefile.txt` Error Count: {adsf_errors_len}",
            )?;
        }
        if asdsf_errors_len > 0 {
            writeln!(
                f,
                "- `animationsetdatasinglefile.txt` Error Count: {asdsf_errors_len}",
            )?;
        }
        if patch_errors_len > 0 {
            writeln!(f, "-       Json Patch Error Count: {patch_errors_len}")?;
        }
        if apply_errors_len > 0 {
            writeln!(f, "- Apply Json Patch Error Count: {apply_errors_len}")?;
        }
        if hkx_errors_len > 0 {
            writeln!(f, "-     Generate hkx Error Count: {hkx_errors_len}")?;
        }
        Ok(())
    }
}

impl std::error::Error for BehaviorGenerationError {}

/// `Result` for this crate.
pub type Result<T, E = Error> = core::result::Result<T, E>;
