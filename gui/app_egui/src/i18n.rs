use indexmap::IndexMap;
use std::borrow::Cow;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum I18nKey {
    AutoDetectButton,
    AutoDetectHover,
    AutoRemoveMeshes,
    AutoRemoveMeshesHover,
    CancelButton,
    ClearButton,
    ColumnId,
    ColumnName,
    ColumnModType,
    ColumnPriority,
    ColumnSite,
    DebugOutput,
    DebugOutputHover,
    ErrorReadingModInfo,
    ExecuteButton,
    ExecutionModeLabel,
    IssueReportButton,
    IssueReportHover,
    LockButton,
    LockButtonHover,
    LogButton,
    LogDir,
    LogLevelLabel,
    ManualMode,
    ManualModeHover,
    ModsListTitle,
    NotificationClearButton,
    NotifyErrPlatformNotSupported,
    NotifyErrWindowsRegistryNotFound,
    NotifyInfoUpdatingModList,
    OutputDirLabel,
    PatchButton,
    RemovingMeshesMessage,
    RuntimeTargetHover,
    RuntimeTargetLabel,
    SearchLabel,
    SelectButton,
    SkyrimDataDirLabel,
    StatusApplyingPatches,
    StatusDone,
    StatusError,
    StatusGeneratingFnisPatches,
    StatusGeneratingHkxFiles,
    StatusParsingPatches,
    StatusReadingPatches,
    Transparent,
    TransparentHover,
    VfsMode,
    VfsModeHover,

    // NOTE: Using `skip_serializing` causes an error when attempting to serialize `Invalid`.
    /// Invalid key comes here when deserializing unknown strings.
    #[serde(other)]
    Invalid,
}

impl I18nKey {
    pub const fn default_eng(&self) -> &'static str {
        match self {
            Self::AutoDetectButton => "Auto Detect",
            Self::AutoDetectHover => "Automatically detect the Skyrim Data directory based on the selected output format. This uses the Steam registry, so it will only work if you have launched Skyrim at least once.",
            Self::AutoRemoveMeshes => "Auto remove `meshes`",
            Self::AutoRemoveMeshesHover => "Delete `<output dir>/meshes`, `<output dir>/.d_merge/.debug` immediately before running the patch.\nNote: If the output directory is the same as Skyrim's data directory, the process will be skipped because deleting the mesh could potentially destroy all mods.",
            Self::CancelButton       => "Cancel",
            Self::ClearButton => "Clear",
            Self::ColumnId => "ID",
            Self::ColumnName => "Name",
            Self::ColumnModType => "Mod Type",
            Self::ColumnPriority => "Priority",
            Self::ColumnSite => "Site",
            Self::DebugOutput => "Debug output",
            Self::DebugOutputHover => "Output d merge patches & merged json files.\n(To `<Output dir>/.d_merge/.debug/patches`)",
            Self::ErrorReadingModInfo => "Error: reading mod info",
            Self::ExecuteButton      => "Execute",
            Self::ExecutionModeLabel => "Execution mode:",
            Self::IssueReportButton => "Report Issue",
            Self::IssueReportHover => "Report a bug on GitHub.\n This will auto-fill version info and some hardware.\n GitHub account required.",
            Self::LockButton => "🔒Locked",
            Self::LockButtonHover => "Row reordering is locked unless sorting by Priority ascending.\nClick to unlock.",
            Self::LogButton => "Log",
            Self::LogDir => "Log Dir",
            Self::LogLevelLabel => "Log Level",
            Self::ManualMode => "Manual mode",
            Self::ManualModeHover => "When using it completely manually.\n(The ID uses a path to prevent errors when different versions of the mod are loaded. For this reason, it is not suitable for transferring settings to others.)",
            Self::ModsListTitle => "Mods",
            Self::NotificationClearButton => "Clear Notify",
            Self::NotifyErrPlatformNotSupported => "Warn: `get_skyrim_data_dir` is not supported on this platform(Linux, MacOs). Please specify the Skyrim data directory manually.",
            Self::NotifyErrWindowsRegistryNotFound => "Error: Could not find path in the Windows registry. \nIf you are not using the Steam version of Skyrim, please specify the Skyrim data directory manually.",
            Self::NotifyInfoUpdatingModList => "Updating Mod list…",
            Self::SelectButton => "Select",
            Self::OutputDirLabel =>"Output dir:",
            Self::PatchButton => "Patch",
            Self::RemovingMeshesMessage => "Removing the `<output_dir>/meshes` directory...",
            Self::RuntimeTargetLabel => "Output format",
            Self::RuntimeTargetHover => "Output format for hkx. LE: win32, SE, VR: amd64\nNOTE(For Windows ver. user): When changing settings in vfs mode, it will automatically attempt to locate and modify the Skyrim Data Directory from the registry.",
            Self::SearchLabel => "Search:",
            Self::SkyrimDataDirLabel =>  "Skyrim Data dir:",
            Self::StatusGeneratingFnisPatches => "[1/6] Generating FNIS patches...",
            Self::StatusReadingPatches => "[2/6] Reading templates and patches...",
            Self::StatusParsingPatches => "[3/6] Parsing patches...",
            Self::StatusApplyingPatches => "[4/6] Applying patches...",
            Self::StatusGeneratingHkxFiles => "[5/6] Generating .hkx files...",
            Self::StatusDone => "[6/6] Done.",
            Self::StatusError => "[Error]: Recommend checking debug and log files.",
            Self::Transparent => "Transparent",
            Self::TransparentHover => "Toggle Transparent window",
            Self::VfsMode => "VFS mode",
            Self::VfsModeHover => "When booting using MO2's VFS, etc.",

            Self::Invalid => "Invalid key. Please confirm i18n of settings json file",
        }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct I18nMap(IndexMap<I18nKey, Cow<'static, str>>);

impl I18nMap {
    pub fn new() -> Self {
        Self(IndexMap::new())
    }

    /// By placing settings in a fixed location within the Skyrim Data directory, you can handle switching between profiles in MO2.
    const FILE: &'static str = "./.d_merge/translation.json";

    /// Translate given key or fallback to default English.
    pub(crate) fn t(&self, key: I18nKey) -> &str {
        self.0
            .get(&key)
            .map_or_else(|| key.default_eng(), |s| s.as_ref())
    }

    /// Generate all key-value pairs for translation.
    #[rustfmt::skip]
    pub fn default_map() -> Self {
        use I18nKey::*;

        // To preserve the order using serde, you have no choice but to use an index map.
        let mut map = Self::new();

        map.0.insert(AutoDetectButton, Cow::Borrowed(AutoDetectButton.default_eng()));
        map.0.insert(AutoDetectHover, Cow::Borrowed(AutoDetectHover.default_eng()));
        map.0.insert(AutoRemoveMeshes, Cow::Borrowed(AutoRemoveMeshes.default_eng()));
        map.0.insert(AutoRemoveMeshesHover, Cow::Borrowed(AutoRemoveMeshesHover.default_eng()));
        map.0.insert(CancelButton, Cow::Borrowed(CancelButton.default_eng()));
        map.0.insert(ClearButton, Cow::Borrowed(ClearButton.default_eng()));
        map.0.insert(ColumnId, Cow::Borrowed(ColumnId.default_eng()));
        map.0.insert(ColumnName, Cow::Borrowed(ColumnName.default_eng()));
        map.0.insert(ColumnModType, Cow::Borrowed(ColumnModType.default_eng()));
        map.0.insert(ColumnPriority, Cow::Borrowed(ColumnPriority.default_eng()));
        map.0.insert(ColumnSite, Cow::Borrowed(ColumnSite.default_eng()));
        map.0.insert(DebugOutput, Cow::Borrowed(DebugOutput.default_eng()));
        map.0.insert(DebugOutputHover, Cow::Borrowed(DebugOutputHover.default_eng()));
        map.0.insert(ErrorReadingModInfo, Cow::Borrowed(ErrorReadingModInfo.default_eng()));
        map.0.insert(ExecuteButton, Cow::Borrowed(ExecuteButton.default_eng()));
        map.0.insert(ExecutionModeLabel, Cow::Borrowed(ExecutionModeLabel.default_eng()));
        map.0.insert(IssueReportButton, Cow::Borrowed(IssueReportButton.default_eng()));
        map.0.insert(IssueReportHover, Cow::Borrowed(IssueReportHover.default_eng()));
        map.0.insert(LockButton, Cow::Borrowed(LockButton.default_eng()));
        map.0.insert(LockButtonHover, Cow::Borrowed(LockButtonHover.default_eng()));
        map.0.insert(LogButton, Cow::Borrowed(LogButton.default_eng()));
        map.0.insert(LogDir, Cow::Borrowed(LogDir.default_eng()));
        map.0.insert(LogLevelLabel, Cow::Borrowed(LogLevelLabel.default_eng()));
        map.0.insert(ManualMode, Cow::Borrowed(ManualMode.default_eng()));
        map.0.insert(ManualModeHover, Cow::Borrowed(ManualModeHover.default_eng()));
        map.0.insert(ModsListTitle, Cow::Borrowed(ModsListTitle.default_eng()));
        map.0.insert(NotificationClearButton, Cow::Borrowed(NotificationClearButton.default_eng()));
        map.0.insert(NotifyErrPlatformNotSupported, Cow::Borrowed(NotifyErrPlatformNotSupported.default_eng()));
        map.0.insert(NotifyErrWindowsRegistryNotFound, Cow::Borrowed(NotifyErrWindowsRegistryNotFound.default_eng()));
        map.0.insert(NotifyInfoUpdatingModList, Cow::Borrowed(NotifyInfoUpdatingModList.default_eng()));
        map.0.insert(OutputDirLabel, Cow::Borrowed(OutputDirLabel.default_eng()));
        map.0.insert(PatchButton, Cow::Borrowed(PatchButton.default_eng()));
        map.0.insert(RuntimeTargetLabel, Cow::Borrowed(RuntimeTargetLabel.default_eng()));
        map.0.insert(RuntimeTargetHover, Cow::Borrowed(RuntimeTargetHover.default_eng()));
        map.0.insert(SearchLabel, Cow::Borrowed(SearchLabel.default_eng()));
        map.0.insert(SelectButton, Cow::Borrowed(SelectButton.default_eng()));
        map.0.insert(SkyrimDataDirLabel, Cow::Borrowed(SkyrimDataDirLabel.default_eng()));
        map.0.insert(StatusGeneratingFnisPatches, Cow::Borrowed(StatusGeneratingFnisPatches.default_eng()));
        map.0.insert(StatusReadingPatches, Cow::Borrowed(StatusReadingPatches.default_eng()));
        map.0.insert(StatusParsingPatches, Cow::Borrowed(StatusParsingPatches.default_eng()));
        map.0.insert(StatusApplyingPatches, Cow::Borrowed(StatusApplyingPatches.default_eng()));
        map.0.insert(StatusGeneratingHkxFiles, Cow::Borrowed(StatusGeneratingHkxFiles.default_eng()));
        map.0.insert(StatusDone, Cow::Borrowed(StatusDone.default_eng()));
        map.0.insert(StatusError, Cow::Borrowed(StatusError.default_eng()));
        map.0.insert(Transparent, Cow::Borrowed(Transparent.default_eng()));
        map.0.insert(TransparentHover, Cow::Borrowed(TransparentHover.default_eng()));
        map.0.insert(VfsMode, Cow::Borrowed(VfsMode.default_eng()));
        map.0.insert(VfsModeHover, Cow::Borrowed(VfsModeHover.default_eng()));

        map
    }

    /// Try to load `./.d_merge/translation.json`.
    /// If not exists or failed to parse, fallback to `default_map()`.
    pub fn load_translation() -> Self {
        use std::fs;
        use std::path::Path;

        let i18n_file = Self::FILE;

        if !Path::new(i18n_file).exists() {
            tracing::warn!("{i18n_file} does not exist. Use default translation.");
            return Self::default_map();
        }

        fs::read_to_string(i18n_file)
            .map_err(|err| {
                tracing::error!("Failed to read translation.json: {err}. Fallback to default.");
            })
            .ok()
            .and_then(|content| {
                serde_json::from_str::<Self>(&content)
                    .map_err(|err| {
                        tracing::error!(
                            "Failed to parse translation.json: {err}. Fallback to default."
                        );
                    })
                    .ok()
            })
            .unwrap_or_else(Self::default_map)
    }

    /// Try save `./.d_merge/translation.json`.
    ///
    /// If already exits, then skip.
    pub fn save_translation() {
        use std::fs;
        use std::path::Path;

        let i18n_file = Self::FILE;

        if Path::new(i18n_file).exists() {
            tracing::info!("{i18n_file} is already exist. So skip write.");
        } else {
            match serde_json::to_string_pretty(&Self::default_map()) {
                Ok(text) => {
                    if let Err(err) = fs::write(i18n_file, text) {
                        tracing::error!("Failed to save translation.json: {err}");
                    };
                    tracing::info!("Settings saved to {i18n_file}");
                }
                Err(err) => {
                    tracing::error!("Failed to parse translation as JSON: {err}");
                }
            }
        };
    }
}

pub fn status_to_text(
    status: nemesis_merge::Status,
    i18n: &I18nMap,
    start_time: std::time::Instant,
) -> String {
    match status {
        nemesis_merge::Status::GeneratingFnisPatches { index, total } => format!(
            "[1/6] {} ({index}/{total})",
            i18n.t(I18nKey::StatusGeneratingFnisPatches),
        ),
        nemesis_merge::Status::ReadingPatches { index, total } => format!(
            "[2/6] {} ({index}/{total})",
            i18n.t(I18nKey::StatusReadingPatches),
        ),
        nemesis_merge::Status::ParsingPatches { index, total } => format!(
            "[3/6] {} ({index}/{total})",
            i18n.t(I18nKey::StatusParsingPatches),
        ),
        nemesis_merge::Status::ApplyingPatches { index, total } => format!(
            "[4/6] {} ({index}/{total})",
            i18n.t(I18nKey::StatusApplyingPatches),
        ),
        nemesis_merge::Status::GeneratingHkxFiles { index, total } => format!(
            "[5/6] {} ({index}/{total})",
            i18n.t(I18nKey::StatusGeneratingHkxFiles),
        ),
        nemesis_merge::Status::Done => {
            let elapsed = start_time.elapsed();
            format!("[6/6] {} ({:.2?})", i18n.t(I18nKey::StatusDone), elapsed)
        }
        nemesis_merge::Status::Error(msg) => {
            format!("[Error] {}: {}", i18n.t(I18nKey::StatusError), msg)
        }
    }
}
