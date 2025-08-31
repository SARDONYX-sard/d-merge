use indexmap::IndexMap;
use std::borrow::Cow;

pub type I18nMap = IndexMap<I18nKey, Cow<'static, str>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum I18nKey {
    #[serde(rename = "auto_remove_meshes")]
    AutoRemoveMeshes,
    #[serde(rename = "auto_remove_meshes_hover")]
    AutoRemoveMeshesHover,
    #[serde(rename = "auto_remove_meshes_warning_body1")]
    AutoRemoveMeshesWarningBody1,
    #[serde(rename = "auto_remove_meshes_warning_body2")]
    AutoRemoveMeshesWarningBody2,
    #[serde(rename = "execute_button")]
    CancelButton,
    #[serde(rename = "clear_button")]
    ClearButton,
    #[serde(rename = "column_name")]
    ColumnName,
    #[serde(rename = "column_id")]
    ColumnId,
    #[serde(rename = "column_priority")]
    ColumnPriority,
    #[serde(rename = "column_site")]
    ColumnSite,
    #[serde(rename = "debug_output")]
    DebugOutput,
    #[serde(rename = "debug_output_hover")]
    DebugOutputHover,
    #[serde(rename = "execution_mode_label")]
    ExecuteButton,
    #[serde(rename = "cancel_button")]
    ExecutionModeLabel,
    #[serde(rename = "error_reading_mod_info")]
    ErrorReadingModInfo,
    #[serde(rename = "log_button")]
    LogButton,
    #[serde(rename = "log_dir")]
    LogDir,
    #[serde(rename = "log_level_label")]
    LogLevelLabel,
    #[serde(rename = "lock_button_hover")]
    LockButtonHover,
    #[serde(rename = "manual_mode")]
    ManualMode,
    #[serde(rename = "manual_mode_hover")]
    ManualModeHover,
    #[serde(rename = "mods_list_title")]
    ModsListTitle,
    #[serde(rename = "notification_clear_button")]
    NotificationClearButton,
    #[serde(rename = "open_button")]
    OpenButton,
    #[serde(rename = "output_dir_label")]
    OutputDirLabel,
    #[serde(rename = "patch_button")]
    PatchButton,
    #[serde(rename = "search_label")]
    SearchLabel,
    #[serde(rename = "skyrim_data_dir_label")]
    SkyrimDataDirLabel,
    #[serde(rename = "transparent_button_label")]
    Transparent,
    #[serde(rename = "transparent_button_hover")]
    TransparentHover,
    #[serde(rename = "vfs_mode")]
    VfsMode,
    #[serde(rename = "vfs_mode_hover")]
    VfsModeHover,
    #[serde(rename = "warning_title")]
    WarningTitle,

    /// Invalid key come, then to this.
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
    pub fn default_map() -> IndexMap<Self, Cow<'static, str>> {
        use I18nKey::*;

        // To preserve the order using serde, you have no choice but to use an index map.
        let mut map = IndexMap::new();

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
        map.insert(VfsMode, Cow::Borrowed(VfsMode.default_eng()));
        map.insert(VfsModeHover, Cow::Borrowed(VfsModeHover.default_eng()));
        map.insert(WarningTitle, Cow::Borrowed(WarningTitle.default_eng()));

        map
    }
}
