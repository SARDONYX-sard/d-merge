use std::borrow::Cow;

use indexmap::IndexMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(serde::Serialize, serde::Deserialize, egui_derive::I18n)]
#[serde(rename_all = "snake_case")]
pub(crate) enum I18nKey {
    /// Auto Detect
    AutoDetectButton,

    /// Automatically detect the Skyrim Data directory based on the selected output format. This uses the Steam registry, so it will only work if you have launched Skyrim at least once.
    AutoDetectHover,

    /// Auto remove `meshes`
    AutoRemoveMeshes,

    /// Delete `<output dir>/meshes`, `<output dir>/.d_merge/.debug` immediately before running the patch.
    /// Note: If the output directory is the same as Skyrim's data directory, the process will be skipped because deleting the mesh could potentially destroy all mods.
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

    /// Cancel
    CancelButton,

    /// Clear
    ClearButton,

    /// ID
    ColumnId,

    /// Name
    ColumnName,

    /// Mod Type
    ColumnModType,

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

    /// Gen FNIS.esp
    GenerateFnisEspLabel,

    /// Generates a dummy FNIS.esp file. (Use this when you want to use a mod that requires FNIS.esp but do not want to use the original FNIS.esp from FNIS SE.)
    GenerateFnisEspHover,

    /// Report Issue
    IssueReportButton,

    /// Report a bug on GitHub.
    ///  This will auto-fill version info and some hardware.
    ///  GitHub account required.
    IssueReportHover,

    /// 🔒Locked
    LockButton,

    /// Row reordering is locked unless sorting by Priority ascending.
    /// Click to unlock.
    LockButtonHover,

    /// Log
    LogButton,

    /// Log Dir
    LogDir,

    /// Log Level
    LogLevelLabel,

    /// Manual mode
    ManualMode,

    /// When using it completely manually.
    /// (The ID uses a path to prevent errors when different versions of the mod are loaded. For this reason, it is not suitable for transferring settings to others.)
    ManualModeHover,

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

    /// Transparent
    Transparent,

    /// Toggle Transparent window
    TransparentHover,

    /// VFS mode
    VfsMode,

    /// When booting using MO2's VFS, etc.
    VfsModeHover,

    /// New I18n
    WriteNewI18nJsonButton,

    /// Write the new translation file (English) to `./.d_merge/translation.json`. The changes will be loaded when you restart the application.
    WriteNewI18nJsonHover,

    // NOTE: Using `skip_serializing` causes an error when attempting to serialize `Invalid`.
    /// Invalid key comes here when deserializing unknown strings.
    #[serde(other)]
    Invalid,
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub(crate) struct I18nMap(IndexMap<I18nKey, Cow<'static, str>>);

impl I18nMap {
    pub(crate) fn new() -> Self {
        Self(IndexMap::new())
    }

    /// By placing settings in a fixed location within the Skyrim Data directory, you can handle switching between profiles in MO2.
    pub(crate) const FILE: &'static str = "./.d_merge/translation.json";

    /// Translate given key or fallback to default English.
    pub(crate) fn t(&self, key: I18nKey) -> &str {
        self.0.get(&key).map_or_else(|| key.default_eng(), |s| s.as_ref())
    }

    /// Try to load `./.d_merge/translation.json`.
    /// If not exists or failed to parse, fallback to `default_map()`.
    pub(crate) fn load_translation() -> Self {
        use std::{fs, path::Path};

        let i18n_file = Self::FILE;

        if !Path::new(i18n_file).exists() {
            tracing::warn!("{i18n_file} does not exist. Use default translation.");
            return Self::new();
        }

        fs::read_to_string(i18n_file)
            .map_err(|err| {
                tracing::error!("Failed to read translation.json: {err}. Fallback to default.");
            })
            .ok()
            .and_then(|content| {
                sonic_rs::from_str::<Self>(&content)
                    .map_err(|err| {
                        tracing::error!(
                            "Failed to parse translation.json: {err}. Fallback to default."
                        );
                    })
                    .ok()
            })
            .unwrap_or_default()
    }

    /// Try save `./.d_merge/translation.json`.
    pub(crate) fn save_translation() -> Result<(), nemesis_merge::errors::Error> {
        use std::{fs, path::PathBuf};
        let i18n_file = Self::FILE;

        let mut map = Self::new();
        for key in I18nKey::ALL {
            map.0.insert(*key, Cow::Borrowed(key.default_eng()));
        }

        let text = sonic_rs::to_string_pretty(&map).map_err(|e| {
            tracing::error!("Failed to parse translation as JSON: {e}");
            nemesis_merge::errors::Error::JsonError { path: PathBuf::from(Self::FILE), source: e }
        })?;

        fs::write(i18n_file, text).map_err(|e| {
            tracing::error!("Failed to save translation.json: {e}");
            nemesis_merge::errors::Error::FailedIo { path: PathBuf::from(Self::FILE), source: e }
        })?;
        Ok(())
    }
}

pub(crate) fn status_to_text(
    status: nemesis_merge::Status,
    i18n: &I18nMap,
    start_time: std::time::Instant,
) -> String {
    match status {
        nemesis_merge::Status::GeneratingFnisPatches { index, total } => {
            format!("[1/6] {} ({index}/{total})", i18n.t(I18nKey::StatusGeneratingFnisPatches),)
        }
        nemesis_merge::Status::ReadingPatches { index, total } => {
            format!("[2/6] {} ({index}/{total})", i18n.t(I18nKey::StatusReadingPatches),)
        }
        nemesis_merge::Status::ParsingPatches { index, total } => {
            format!("[3/6] {} ({index}/{total})", i18n.t(I18nKey::StatusParsingPatches),)
        }
        nemesis_merge::Status::ApplyingPatches { index, total } => {
            format!("[4/6] {} ({index}/{total})", i18n.t(I18nKey::StatusApplyingPatches),)
        }
        nemesis_merge::Status::GeneratingHkxFiles { index, total } => {
            format!("[5/6] {} ({index}/{total})", i18n.t(I18nKey::StatusGeneratingHkxFiles),)
        }
        nemesis_merge::Status::Done => {
            let elapsed = start_time.elapsed().as_secs_f32();
            format!("[6/6] {} ({elapsed:.2}s)", i18n.t(I18nKey::StatusDone))
        }
        nemesis_merge::Status::Error(msg) => {
            let elapsed = start_time.elapsed().as_secs_f32();
            format!("[Error] {} ({elapsed:.2}s) {msg}", i18n.t(I18nKey::StatusError),)
        }
    }
}

pub(crate) const fn status_to_color(status: &nemesis_merge::Status) -> egui::Color32 {
    match status {
        nemesis_merge::Status::GeneratingFnisPatches { .. } => {
            egui::Color32::from_rgb(120, 170, 255)
        }
        nemesis_merge::Status::ReadingPatches { .. } => EGUI_RIGHT_BLUE,
        nemesis_merge::Status::ParsingPatches { .. } => egui::Color32::from_rgb(140, 200, 255),
        nemesis_merge::Status::ApplyingPatches { .. } => egui::Color32::from_rgb(255, 170, 120),
        nemesis_merge::Status::GeneratingHkxFiles { .. } => egui::Color32::from_rgb(200, 140, 255),
        nemesis_merge::Status::Done => egui::Color32::GREEN,
        nemesis_merge::Status::Error(_) => egui::Color32::RED,
    }
}

pub(crate) const EGUI_RIGHT_BLUE: egui::Color32 = egui::Color32::from_rgb(120, 220, 255);
