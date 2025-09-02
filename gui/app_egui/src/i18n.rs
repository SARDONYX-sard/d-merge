use indexmap::IndexMap;
use std::borrow::Cow;

pub type I18nMap = IndexMap<I18nKey, Cow<'static, str>>;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum I18nKey {
    AutoRemoveMeshes,
    AutoRemoveMeshesHover,
    AutoRemoveMeshesWarningBody1,
    AutoRemoveMeshesWarningBody2,
    CancelButton,
    ClearButton,
    ColumnId,
    ColumnName,
    ColumnPriority,
    ColumnSite,
    DebugOutput,
    DebugOutputHover,
    ErrorReadingModInfo,
    ExecuteButton,
    ExecutionModeLabel,
    LockButtonHover,
    LogButton,
    LogDir,
    LogLevelLabel,
    ManualMode,
    ManualModeHover,
    ModsListTitle,
    NotificationClearButton,
    OpenButton,
    OutputDirLabel,
    PatchButton,
    SearchLabel,
    SkyrimDataDirLabel,
    Transparent,
    TransparentHover,
    VfsMode,
    VfsModeHover,
    WarningTitle,

    // NOTE: Using `skip_serializing` causes an error when attempting to serialize `Invalid`.
    /// Invalid key comes here when deserializing unknown strings.
    #[serde(other)]
    Invalid,
}

impl I18nKey {
    pub const fn default_eng(&self) -> &'static str {
        match self {
            Self::AutoRemoveMeshes => "Auto remove meshes",
            Self::AutoRemoveMeshesHover => "Delete `<output dir>/meshes`, `<output dir>/.d_merge/.debug` immediately before running the patch.",
            Self::AutoRemoveMeshesWarningBody1 => "Deleting the auto meshes directory in Skyrim Data Dir is dangerous.",
            Self::AutoRemoveMeshesWarningBody2 => "It may remove files of other mods (like OAR). Are you sure?",
            Self::CancelButton       => "Cancel",
            Self::ClearButton => "Clear",
            Self::ColumnId => "ID",
            Self::ColumnName => "Name",
            Self::ColumnPriority => "Priority",
            Self::ColumnSite => "Site",
            Self::DebugOutput => "Debug output",
            Self::DebugOutputHover => "Output d merge patches & merged json files.\n(To `<Output dir>/.d_merge/.debug/patches`)",
            Self::ErrorReadingModInfo => "Error: reading mod info",
            Self::ExecuteButton      => "Execute",
            Self::ExecutionModeLabel => "Execution mode:",
            Self::LockButtonHover => "Row reordering is locked unless sorting by Priority ascending.\nClick to unlock.",
            Self::LogButton => "Log",
            Self::LogDir => "Log Dir",
            Self::LogLevelLabel => "Log Level",
            Self::ManualMode => "Manual mode",
            Self::ManualModeHover => "When using it completely manually.",
            Self::ModsListTitle => "Mods",
            Self::NotificationClearButton => "Clear Notify",
            Self::OpenButton => "Open",
            Self::OutputDirLabel =>"Output dir:",
            Self::PatchButton => "Patch",
            Self::SearchLabel => "Search:",
            Self::SkyrimDataDirLabel =>  "Skyrim Data dir:",
            Self::Transparent => "Transparent",
            Self::TransparentHover => "Toggle Transparent window",
            Self::VfsMode => "VFS mode",
            Self::VfsModeHover => "When booting using MO2's VFS, etc.",
            Self::WarningTitle => "⚠ Warning",

            Self::Invalid => "Invalid key. Please confirm i18n of settings json file",
        }
    }

    /// Generate all key-value pairs for translation.
    #[rustfmt::skip]
    pub fn default_map() -> I18nMap {
        use I18nKey::*;

        // To preserve the order using serde, you have no choice but to use an index map.
        let mut map = I18nMap::new();

        map.insert(AutoRemoveMeshes, Cow::Borrowed(AutoRemoveMeshes.default_eng()));
        map.insert(AutoRemoveMeshesHover, Cow::Borrowed(AutoRemoveMeshesHover.default_eng()));
        map.insert(AutoRemoveMeshesWarningBody1, Cow::Borrowed(AutoRemoveMeshesWarningBody1.default_eng()));
        map.insert(AutoRemoveMeshesWarningBody2, Cow::Borrowed(AutoRemoveMeshesWarningBody2.default_eng()));
        map.insert(CancelButton, Cow::Borrowed(CancelButton.default_eng()));
        map.insert(ClearButton, Cow::Borrowed(ClearButton.default_eng()));
        map.insert(ColumnId, Cow::Borrowed(ColumnId.default_eng()));
        map.insert(ColumnName, Cow::Borrowed(ColumnName.default_eng()));
        map.insert(ColumnPriority, Cow::Borrowed(ColumnPriority.default_eng()));
        map.insert(ColumnSite, Cow::Borrowed(ColumnSite.default_eng()));
        map.insert(DebugOutput, Cow::Borrowed(DebugOutput.default_eng()));
        map.insert(DebugOutputHover, Cow::Borrowed(DebugOutputHover.default_eng()));
        map.insert(ErrorReadingModInfo, Cow::Borrowed(ErrorReadingModInfo.default_eng()));
        map.insert(ExecuteButton, Cow::Borrowed(ExecuteButton.default_eng()));
        map.insert(ExecutionModeLabel, Cow::Borrowed(ExecutionModeLabel.default_eng()));
        map.insert(LockButtonHover, Cow::Borrowed(LockButtonHover.default_eng()));
        map.insert(LogButton, Cow::Borrowed(LogButton.default_eng()));
        map.insert(LogDir, Cow::Borrowed(LogDir.default_eng()));
        map.insert(LogLevelLabel, Cow::Borrowed(LogLevelLabel.default_eng()));
        map.insert(ManualMode, Cow::Borrowed(ManualMode.default_eng()));
        map.insert(ManualModeHover, Cow::Borrowed(ManualModeHover.default_eng()));
        map.insert(ModsListTitle, Cow::Borrowed(ModsListTitle.default_eng()));
        map.insert(NotificationClearButton, Cow::Borrowed(NotificationClearButton.default_eng()));
        map.insert(OpenButton, Cow::Borrowed(OpenButton.default_eng()));
        map.insert(OutputDirLabel, Cow::Borrowed(OutputDirLabel.default_eng()));
        map.insert(PatchButton, Cow::Borrowed(PatchButton.default_eng()));
        map.insert(SearchLabel, Cow::Borrowed(SearchLabel.default_eng()));
        map.insert(SkyrimDataDirLabel, Cow::Borrowed(SkyrimDataDirLabel.default_eng()));
        map.insert(Transparent, Cow::Borrowed(SkyrimDataDirLabel.default_eng()));
        map.insert(TransparentHover, Cow::Borrowed(SkyrimDataDirLabel.default_eng()));
        map.insert(VfsMode, Cow::Borrowed(VfsMode.default_eng()));
        map.insert(VfsModeHover, Cow::Borrowed(VfsModeHover.default_eng()));
        map.insert(WarningTitle, Cow::Borrowed(WarningTitle.default_eng()));

        map
    }
}
