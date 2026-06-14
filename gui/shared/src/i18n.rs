use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

use indexmap::IndexMap;
use rayon::prelude::*;
use snafu::ResultExt as _;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(serde::Serialize, serde::Deserialize, d_merge_gui_shared_derive::I18n)]
#[serde(rename_all = "snake_case")]
pub enum I18nKey {
    /// Author:
    AuthorLabel,

    /// Auto Detect
    AutoDetectButton,

    /// Automatically detect the Skyrim Data directory based on the selected output format. This uses the Steam registry, so it will only work if you have launched Skyrim at least once.
    AutoDetectHover,

    /// Auto remove `meshes`
    AutoRemoveMeshes,

    /// Deletes previous output before patch execution.
    ///
    /// Targets:
    /// - `<output directory>/meshes`
    /// - `<output directory>/.d_merge/.debug`
    /// - `<output directory>/FNIS.esp`
    /// - `<output directory>/SKSE/Plugins/fnis_aa/config.json`
    ///
    /// Note:
    /// To avoid accidental mod corruption caused by deleting `meshes`,
    /// deletion is skipped when the output directory is the same as the Skyrim Data directory.
    AutoRemoveMeshesHover,

    /// Auto run
    AutoRun,

    /// Once the mod list has been updated, enable all mods and run the patch once.
    /// (You must close the window manually to prevent the auto-run option from becoming disabled.)
    ///
    /// # Mod list update triggers
    /// - Once at startup
    /// - When the reload button is pressed
    /// - When switching between vfs and manual environments
    /// - When the Skyrim data directory is changed
    AutoRunHover,

    /// Bug Report:
    BugReportLabel,

    /// See Issues
    BugReportSeeIssues,

    /// Cancel
    CancelButton,

    /// Change Log:
    ChangeLogLabel,

    /// Clear
    ClearButton,

    /// ID
    ColumnId,

    /// Mod Type
    ColumnModType,

    /// Name
    ColumnName,

    /// Priority
    ColumnPriority,

    /// Site
    ColumnSite,

    /// Debug output
    DebugOutput,

    /// Output d merge patches & merged json files.
    /// (To `<Output dir>/.d_merge/.debug/patches`)
    DebugOutputHover,

    /// Execute
    ExecuteButton,

    /// Execution mode:
    ExecutionModeLabel,

    /// All
    FilterTextAll,

    /// Generates a dummy FNIS.esp file.
    ///
    /// This feature is required in the following situations:
    /// - You are using mods that depend on FNIS.esp but are not using the original FNIS.esp
    GenerateFnisEspHover,

    /// Gen FNIS.esp
    GenerateFnisEspLabel,

    /// Help
    HelpButton,

    /// I18n Path:
    I18nPathLabel,

    /// Reload the translation file. This is useful for previewing translations without restarting the application.
    I18nReloadJsonHover,

    /// New I18n
    I18nWriteNewJsonButton,

    /// Write the new English translation file.
    I18nWriteNewJsonHover,

    /// Report Issue
    IssueReportButton,

    /// Report a bug on GitHub.
    /// This will auto-fill version info and some hardware.
    /// GitHub account required.
    IssueReportHover,

    /// License:
    LicenseLabel,

    /// 🔒Locked
    LockButton,

    /// Row reordering is locked unless sorting by Priority ascending.
    /// Click to unlock.
    LockButtonHover,

    /// Log
    LogButton,

    /// Log Dir
    LogDir,

    /// Log Path:
    LogDirPathLabel,

    /// Change the log directory to the specified path.
    /// The log path will not change until you click this button.(Or restart app)
    LogReloadHover,

    /// Log Level
    LogLevelLabel,

    /// Manual mode
    ManualMode,

    /// When using it completely manually.
    /// (The ID uses a path to prevent errors when different versions of the mod are loaded. For this reason, it is not suitable for transferring settings to others.)
    ManualModeHover,

    /// Mod Test Status:
    ModTestStatusLabel,

    /// Updated mod list
    ModsListFetchStateDone,

    /// No mods found
    ModsListFetchStateEmpty,

    /// Failed to update mod list
    ModsListFetchStateError,

    /// Fetching mod list...
    ModsListFetchStateFetching,

    /// Mods
    ModsListTitle,

    /// The list of mods is collected by scanning the following glob paths in the Skyrim Data directory.
    ///
    /// - Nemesis: Nemesis_Engine/mod/*/info.ini
    /// - NemesisExt: Nemesis_EngineExt/mod/*/info.ini
    /// - FNIS: meshes/**/animations/*/FNIS_*_List.txt
    ModsListTitleHover,

    /// Normalize
    NormalizeButton,

    /// Sort the mod list according to the following rules
    ///
    /// 1. Group mods by type in the following order:
    ///    Nemesis -> NemesisExt -> FNIS
    /// 2. Sort mods alphabetically within each type.
    /// 3. Reassign priorities sequentially starting from `0`.
    /// 4. In manual mode, disable the check for duplicate ID rows.
    ///
    /// # Notes
    /// This sorting does not resolve conflicts between patches.
    NormalizeHover,

    /// Clear Notify
    NotificationClearButton,

    /// Warn: `get_skyrim_data_dir` is not supported on this platform(Linux, MacOs). Please specify the Skyrim data directory manually.
    NotifyErrPlatformNotSupported,

    /// Error: Could not find path in the Windows registry.
    /// If you are not using the Steam version of Skyrim, please specify the Skyrim data directory manually.
    NotifyErrWindowsRegistryNotFound,

    /// Updating Mod list…
    NotifyInfoUpdatingModList,

    /// Output dir:
    OutputDirLabel,

    /// Patch
    PatchButton,

    /// Updating list...
    PatchFetchingButton,

    /// Reload
    ReloadButton,

    /// Removing the `<output_dir>/meshes` directory...
    RemovingMeshesMessage,

    /// Output format for hkx. LE: win32, SE, VR: amd64
    /// NOTE(For Windows ver. user): When changing settings in vfs mode, it will automatically attempt to locate and modify the Skyrim Data Directory from the registry.
    RuntimeTargetHover,

    /// Output format
    RuntimeTargetLabel,

    /// Search:
    SearchLabel,

    /// Select
    SelectButton,

    /// Skyrim Data dir:
    SkyrimDataDirLabel,

    /// Source Code:
    SourceCodeLabel,

    /// Applying patches...
    StatusApplyingPatches,

    /// Done.
    StatusDone,

    /// Recommend checking debug and log files.
    StatusError,

    /// Generating FNIS patches...
    StatusGeneratingFnisPatches,

    /// Generating .hkx files...
    StatusGeneratingHkxFiles,

    /// Parsing patches...
    StatusParsingPatches,

    /// Reading templates and patches...
    StatusReadingPatches,

    /// Theme:
    ThemeLabel,

    /// Toggle between Dark, Light, and System themes.
    /// - NOTE: Using Light theme and the transparent background feature at the same time makes the screen very hard to read.
    ThemeHover,

    /// Tooling:
    ToolingLabel,

    /// Transparent
    Transparent,

    /// Enable transparent background. This makes the background of the application transparent, allowing you to see the desktop or other windows behind it.
    TransparentHover,

    /// VFS mode
    VfsMode,

    /// When booting using MO2's VFS, etc.
    VfsModeHover,

    // NOTE: Using `skip_serializing` causes an error when attempting to serialize `Invalid`.
    /// Invalid key comes here when deserializing unknown strings.
    #[serde(other)]
    Invalid,
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct I18nMap(IndexMap<I18nKey, Cow<'static, str>, rapidhash::fast::RandomState>);

impl I18nMap {
    pub fn new() -> Self {
        Self(IndexMap::default())
    }

    /// By placing settings in a fixed location within the Skyrim Data directory, you can handle switching between profiles in MO2.
    pub const FILE: &'static str = "./.d_merge/translation.json";

    /// Translate given key or fallback to default English.
    pub fn t(&self, key: I18nKey) -> &str {
        self.0.get(&key).map_or_else(|| key.default_eng(), |s| s.as_ref())
    }

    /// Try to load `./.d_merge/translation.json`.
    /// If not exists or failed to parse, fallback to `default_map()`.
    ///
    /// # Errors
    /// failed to read json
    pub fn load<P>(path: P) -> Result<Self, Error>
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();

        if !path.exists() {
            #[cfg(feature = "tracing")]
            tracing::warn!("translation.json does not exist");
            return Ok(Self::new());
        }

        let content = std::fs::read_to_string(path).with_context(|_| ReadFileSnafu { path })?;
        let map: Self = sonic_rs::from_str(&content).with_context(|_| ParseJsonSnafu { path })?;
        Ok(map)
    }

    /// Save translation.json
    /// # Errors
    /// failed to write json
    pub fn save<P>(path: P) -> Result<(), Error>
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();

        let map = Self(
            I18nKey::ALL.par_iter().map(|key| (*key, Cow::Borrowed(key.default_eng()))).collect(),
        );

        let text = sonic_rs::to_string_pretty(&map).context(SerializeJsonSnafu { path })?;
        std::fs::write(path, text).context(WriteFileSnafu { path })?;
        Ok(())
    }
}

#[derive(Debug, snafu::Snafu)]
pub enum Error {
    #[snafu(display("Failed to read file: {}", path.display()))]
    ReadFile { path: PathBuf, source: std::io::Error },

    #[snafu(display("Failed to write file: {}", path.display()))]
    WriteFile { path: PathBuf, source: std::io::Error },

    #[snafu(display("Failed to parse json: {}", path.display()))]
    ParseJson { path: PathBuf, source: sonic_rs::Error },

    #[snafu(display("Failed to serialize json: {}", path.display()))]
    SerializeJson { path: PathBuf, source: sonic_rs::Error },
}
