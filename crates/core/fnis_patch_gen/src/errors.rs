//! Errors of FNIS patch generation.
//!
//! - [`Error`]: everything this crate returns. `behavior_merge` wraps it in its own error.
//! - [`FnisError`]: finding a mod's files (list, behavior, animations)
//! - [`FnisPatchGenerationError`]: turning one `FNIS_*_List.txt` into patches

use std::{io, path::PathBuf};

use behavior_merge_types::OutPutTarget;
use winnow_ext::ReadableError;

/// Error of this crate.
#[derive(Debug, snafu::Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    /// {msg}
    Custom { msg: String },

    #[snafu(transparent)]
    FnisError { source: FnisError },

    #[snafu(transparent)]
    FnisPatchGenerationError { source: FnisPatchGenerationError },

    /// Failed to parse FNIS_*_List.txt file.
    #[snafu(display("[FNIS_*_List.txt file Parse Error]{}:\n{source}", path.display()))]
    FailedParseFnisModList { source: ReadableError, path: PathBuf },

    #[snafu(display( "FNIS I/O processing (read/write to config.json or hkx files) failed. This is likely due to a busy condition or lack of permission.:\npath = {}, err = {source}", path.display()))]
    FNISHkxIoError { path: PathBuf, source: io::Error },

    #[snafu(display(
        "While attempting to automatically convert FNIS HKX for target {target:?}, \
        the file {} did not have the expected Havok magic numbers. \
        Expected magic=[0x57, 0xe0, 0xe0, 0x57, 0x10, 0xc0, 0xc0, 0x10, ...], \
        but got {magic_bytes:#x?}. \
        This file is not a valid Havok animation or may be from an unsupported version.",
        input_path.display()
    ))]
    FNISHkxInvalidMagic { input_path: PathBuf, target: OutPutTarget, magic_bytes: [u8; 17] },

    #[snafu(display(
        "While attempting to automatically convert FNIS HKX for target {target:?}, \
        pointer size check failed for {}. \
        Expected pointer size 4/8-byte for {target:?}, \
        but could not determine a valid header or got {actual}-byte. \
        The HKX may be malformed or from an incompatible platform.",
        input_path.display()
    ))]
    FNISHkxInvalidHeader { input_path: PathBuf, target: OutPutTarget, actual: u8 },

    #[snafu(display("The conversion of FNIS's hkx failed. input_path: {}, why: {source}", input_path.display()))]
    FNISHkxConversionError { input_path: PathBuf, source: serde_hkx_features::error::Error },

    #[snafu(display("Failed to generate FNIS.esp file: (cause: {source})"))]
    FNISGenerateEspError { source: io::Error },

    /// Failed to io: path = {path}, source = {source}
    #[snafu(display("Failed to io: path = {}, source = {source}", path.display()))]
    FailedIo { path: PathBuf, source: io::Error },

    /// serde_hkx serialize error.
    #[snafu(display("[serde_hkx Ser Error] {}:\n {source}", path.display()))]
    HkxSerError { path: PathBuf, source: serde_hkx::errors::ser::Error },

    /// serde_hkx Deserialize error.
    #[snafu(display("[serde_hkx De Error] {}:\n {source}", path.display()))]
    HkxDeError { path: PathBuf, source: serde_hkx::errors::de::Error },

    /// (De)Serialize json error
    #[snafu(display("[Json serde error] {}:\n {source}", path.display()))]
    JsonError { path: PathBuf, source: sonic_rs::Error },

    #[snafu(transparent)]
    JoinError { source: tokio::task::JoinError },
}
#[derive(Debug, snafu::Snafu)]
#[snafu(visibility(pub))]
pub enum FnisError {
    /// One or more HKX files were expected below the given directory, but none were found.
    #[snafu(display(
        "One or more hkx files were expected below(`{animations_mod_dir:?}`), but none were found."
    ))]
    EmptyAnimPaths { animations_mod_dir: PathBuf },

    /// Expected list file at `{expected}`, but couldn't read this path.
    #[snafu(display("Expected list file at {expected}, but couldn't read this path.: {source}"))]
    FailedReadingListFile { expected: String, source: io::Error },

    /// Failed to get the parent directory of the animations mod directory.
    /// This indicates that the provided `animations_mod_dir` is too shallow in the filesystem hierarchy.
    #[snafu(display("Failed to get parent directory for `{animations_mod_dir:?}`"))]
    BehaviorParentMissing { animations_mod_dir: PathBuf },

    /// Not found Parent of Mod root behavior registered target template path (e.g. behaviors/0_master.bin)
    #[snafu(display("Not found this sub dir parent: {}", sub_dir.display()))]
    BehaviorNotFoundSubDirParent { sub_dir: PathBuf },

    /// No behavior files found.
    /// For example: `FNIS_<namespace>_Behavior.hkx` or `FNIS_<namespace>_<suffix>_Behavior.hkx`.
    #[snafu(display("No behavior file found: `{}`", path.display()))]
    BehaviorNotFound { path: PathBuf },
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug, snafu::Snafu)]
#[snafu(visibility(pub))]
pub enum FnisPatchGenerationError {
    /// The addition of pairs and kill moves animation applies only to 3rd person humanoids; creatures are not supported.
    #[snafu(display("The addition of pairs and kill moves animation applies only to 3rd person humanoids; creatures are not supported.: {}", path.display()))]
    UnsupportedPairAndKillMoveForCreature { path: PathBuf },

    #[snafu(display(
        "Failed to convert alternate animation to OAR: {}", errors.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("\n")
    ))]
    FailedToConvertAltAnimToOAR { errors: Vec<Error> },

    /// The addition of furniture animation applies only to 3rd person character; `_1stperson`, creatures are not supported.
    #[snafu(display("The addition of furniture(fu, fuo) animation applies only to 3rd person `character`; `_1stperson`, creatures are not supported.: {}", path.display()))]
    UnsupportedFurnitureAnimationToCreature { path: PathBuf },

    /// The addition of OffsetArm animation applies only to 3rd person character; `_1stperson`, creatures are not supported.
    #[snafu(display("The addition of OffsetArm(ofa) animation applies only to 3rd person `character`; `_1stperson`, creatures are not supported.: {}", path.display()))]
    UnsupportedOffsetArmAnimationToCreature { path: PathBuf },

    // AnimSet Patch error ----------
    /// The generated animation path unexpectedly has no parent directory.
    #[snafu(display("Failed to determine the parent directory of the animation path: {}", path.display()))]
    MissingAnimationParent { path: PathBuf },

    /// The parent directory of the generated animation path is not valid UTF-8.
    #[snafu(display("The parent directory of the animation path is not valid UTF-8: {}", path.display()))]
    InvalidAnimationParent { path: PathBuf },

    /// The generated animation path has no valid file stem.
    #[snafu(display("Failed to determine the file stem of the animation path: {}", path.display()))]
    MissingAnimationFileStem { path: PathBuf },
}
