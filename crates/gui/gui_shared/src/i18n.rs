use std::{borrow::Cow, path::Path};

use indexmap::IndexMap;
use rayon::prelude::*;
use snafu::ResultExt as _;

use crate::error::{Error, *};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(serde::Serialize, serde::Deserialize, gui_shared_derive::I18n)]
#[serde(rename_all = "snake_case")]
pub enum I18nKey {
    /// Gets the bone count from each `skeleton.xml` distributed by mods such as
    /// XPMSSE and extends the corresponding character hkx based on that information.
    ///
    /// This provides functionality equivalent to `Auto Skeleton Patch` and FNIS's `SKELETON arm fix`.
    ///
    /// # Recommended
    /// The following mods are recommended to use with Arm Fix:
    /// (Without this, arm animations such as blocking while moving may not work)
    /// - XPMSSE (v5.06) and other bone-expanding mods
    ///
    /// # Notes
    /// The following may result in incorrect patching:
    /// - Manual mode + `Skyrim Data directory = MO2/mods/*`
    ///   -> Multiple `skeleton.xml` files may be found.
    ///   -> The first match is used.
    ///   -> A warning is logged when multiple files are found.
    ///
    /// # Targets
    /// - `<Skyrim Data directory>/meshes/actors/character/character assets/skeleton.xml`
    ///   -> `defaultmale.hkx`
    /// - `<Skyrim Data directory>/meshes/actors/character/character assets female/skeleton_female.xml`
    ///   -> `defaultfemale.hkx`
    ///
    /// # Processing
    /// - Missing bone indices -> extend `bonePairMap`.
    /// - Missing bone weights -> append `0.0` / `1.0` to the corresponding weight arrays.
    ArmFixHover,

    /// Arm fix
    ArmFixLabel,

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
    /// # Targets
    /// - `<output directory>/meshes`
    /// - `<output directory>/.d_merge/.debug`
    /// - `<output directory>/FNIS.esp`
    /// - `<output directory>/SKSE/Plugins/fnis_aa/config.json`
    ///
    /// # Notes
    /// To avoid accidental mod corruption caused by deleting `meshes`,
    /// deletion is skipped when the output directory is the same as the Skyrim Data directory.
    AutoRemoveMeshesHover,

    /// Auto run
    AutoRun,

    /// Once the mod list has been updated, enable all mods and run the patch once.
    ///
    /// # Mod list update triggers
    /// - Once at startup
    /// - When the reload button is pressed
    /// - When switching between vfs and manual environments
    /// - When the Skyrim data directory is changed
    ///
    /// # Notes
    /// You must close the window manually to prevent the auto-run option from becoming disabled.
    AutoRunHover,

    /// Background
    BackgroundTitleLabel,

    /// Background Image:
    BackgroundImageLabel,

    /// Reload background image.
    ///
    /// # Notes
    /// When you click this button, the `Enabled` checkbox will also be checked.
    BackgroundImageReloadHover,

    /// Enabled
    BackgroundImageEnabled,

    /// Whether the background image is shown.
    ///
    /// This feature exists to allow you to toggle the setting on and off while keeping the background image path specified.
    BackgroundImageEnabledHover,

    /// Bug Report
    BugReportTitleLabel,

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

    /// Outputs debug JSON and XML files.
    ///
    /// # Examples
    /// - `<output directory>/.d_merge/.debug/meshes/.../0_master.xml`
    /// - `<output directory>/.d_merge/.debug/meshes/.../0_master.json`
    /// - `<output directory>/.d_merge/.debug/patches/meshes/.../0_master.patch.json`
    DebugOutputHover,

    /// Execution mode:
    ExecutionModeLabel,

    /// All
    FilterTextAll,

    /// Generates a dummy FNIS.esp file.
    ///
    /// # Examples:
    /// - XPMSE
    /// - FNIS Sexy Move
    /// - EatingSleepingDrinking
    GenerateFnisEspHover,

    /// Gen FNIS.esp
    GenerateFnisEspLabel,

    /// Font
    FontTitleLabel,

    /// Font family:
    FontFamily,

    /// Font Source:
    FontModeLabel,

    /// Font loading mode configuration.
    ///
    /// - Default:
    ///   Uses the application's built-in font configuration.
    /// - System:
    ///   Loads a font from an installed system font family.
    /// - File:
    ///   Loads a font from the specified font file.
    ///
    /// # Notes
    /// Different fonts may alter text width and spacing, which can affect
    /// table layouts, column alignment, and overall UI appearance.
    FontModeHover,

    /// Default
    FontModeDefault,

    /// System
    FontModeSystem,

    /// File
    FontModeFile,

    /// Font Path:
    FontFileLabel,

    /// Reloads and applies the currently selected font path.
    FontReloadHover,

    /// Help
    HelpButton,

    /// Language & Translation
    I18nTitleLabel,

    /// I18n Path:
    I18nPathLabel,

    /// Reload the translation file. This is useful for previewing translations without restarting the application.
    I18nReloadJsonHover,

    /// New I18n
    I18nWriteNewJsonButton,

    /// Create a new translation JSON file for the current application version with English default values.
    I18nWriteNewJsonHover,

    /// Report Issue
    IssueReportButton,

    /// Report a bug on GitHub.
    /// This will auto-fill version info and some hardware.
    ///
    /// # Notes
    /// GitHub account required.
    IssueReportHover,

    /// License:
    LicenseLabel,

    /// Locked
    LockButton,

    /// Row reordering is locked unless sorting by Priority ascending.
    /// Click to unlock.
    LockButtonHover,

    /// Logging
    LoggingTitleLabel,

    /// Log
    LogButton,

    /// Log Dir
    LogDir,

    /// Log Path:
    LogDirPathLabel,

    /// Change the log directory to the specified path.
    ///
    /// # Notes
    /// Logs are rotated with a maximum of 5 files retained.
    /// The log directory will not change until this button is clicked
    /// (or the application is restarted).
    LogReloadHover,

    /// Log Level
    LogLevelLabel,

    /// Manual
    ManualMode,

    /// Mode for use without virtual mod managers such as MO2.
    ///
    /// # Notes
    /// - Conflicts may occur if multiple versions of the same mod exist.
    /// - In manual mode, absolute paths are used as identifiers to prevent duplication. This makes configuration sharing unsuitable.
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

    /// Open selected path.
    OpenSelectedPathHover,

    /// Output dir:
    OutputDirLabel,

    /// Controls how parser input is validated and how invalid patch data is handled.
    ///
    /// - Strict:
    ///   Rejects inconsistent or invalid input. This is the traditional behavior
    ///   used before v2.6.0.
    /// - Lenient:
    ///   Behaves similarly to Nemesis by attempting to recover from known input
    ///   mistakes when the intended value can be inferred from the input.
    ///
    /// # Notes
    /// - A patch that requires Lenient mode contains invalid or potentially
    ///   undefined input. Even if the parser can recover from it, the resulting
    ///   behavior is not guaranteed to match the mod author's intent.
    /// - Incorrect length values may cause unintended behavior. Lenient mode only
    ///   allows the parser to continue based on the available input; it does not
    ///   guarantee that the recovered data is correct.
    /// - Lenient mode is not a convenience mode. It is a fallback for users who
    ///   cannot reasonably fix the original patch themselves.
    /// - Input that cannot be reasonably recovered, or whose intended value cannot
    ///   be inferred, still fails in Lenient mode.
    ///
    /// Examples of mods with `trigger_names_len` errors that require Lenient mode:
    /// - Unarmed Behaviors Expanded (v2.0.1)
    /// - BFCO (v3.100.7)
    ParserModeHover,

    /// Parser:
    ParserModeLabel,

    /// Strict
    ParserModeStrict,

    /// Lenient
    ParserModeLenient,

    /// Patch
    PatchButton,

    /// Updating list...
    PatchFetchingButton,

    /// Target version:
    PandoraTargetVersionLabel,

    /// Select the Pandora version used for import or export.
    ///
    /// - `v4.4.0-beta`: Compatible with `v5.0.0`.
    PandoraTargetVersionHover,

    /// output dir:
    PandoraOutputDirLabel,

    /// Takes the settings of Pandora Behaviour Engine into d-merge's VFS mode, or gives them to it.
    PandoraButtonHover,

    /// The directory with Pandora's Settings.json and Pandora_Engine\ActiveMods.json. Import reads them from here and Export writes them here. Only used for these two buttons. The button on the left opens the directory.
    PandoraDirHover,

    /// Imports Pandora settings into the d-merge VFS configuration.
    ///
    /// # Targets
    /// - `Settings.json`
    ///   - Skyrim SE data path
    ///   - Output path
    /// - `ActiveMods.json`
    ///   - Nemesis mod enabled state
    ///   - Nemesis mod priority
    PandoraImportHover,

    /// Import
    PandoraImportLabel,

    /// Exports d-merge VFS settings to Pandora.
    ///
    /// # Targets
    /// - `Settings.json`
    ///   - Skyrim SE data path
    ///   - Output path
    /// - `ActiveMods.json`
    ///   - Nemesis mod enabled state
    ///   - Nemesis mod priority
    PandoraExportHover,

    /// Export
    PandoraExportLabel,

    /// Reload
    ReloadButton,

    /// Removing the `<output_dir>/meshes` directory...
    RemovingMeshesMessage,

    /// Output format for hkx.
    /// - SkyrimLE/Enderal: win32
    /// - SkyrimSE/SkyrimVR/EnderalSE: amd64
    ///
    /// # Notes
    /// For Windows ver. user: When changing settings in vfs mode, it will automatically attempt to locate and modify the Skyrim Data Directory from the registry.
    RuntimeTargetHover,

    /// Output format:
    RuntimeTargetLabel,

    /// Search
    SearchLabel,

    /// hits
    SearchHitsLabel,

    /// Select
    SelectButton,

    /// selected
    SelectedLabel,

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

    /// 🎨 Edit
    ThemeEditorButton,

    /// Open the theme editor to customize the app's appearance and color scheme
    ThemeEditorButtonHover,

    /// Change the application's overall appearance and color scheme
    ThemeHover,

    /// Theme:
    ThemeLabel,

    /// 💻 System
    ThemeSelectSystem,

    /// 🌙 Dark
    ThemeSelectDark,

    /// ☀ Light
    ThemeSelectLight,

    /// 🎨 Custom
    ThemeSelectCustom,

    /// VFS
    VfsMode,

    /// When booting using MO2's VFS, etc.
    VfsModeHover,

    // ── Page navigation ─────────────────────────────────────────────────────
    /// Patch
    PagePatch,

    /// Convert
    PageConvert,

    /// hkanno Editor
    PageHkanno,

    /// Patch Editor
    PagePatchEditor,

    // ── Patch Editor page ─────────────────────────────────────────────────
    /// Patch
    NemesisPatchTab,

    /// CRC32
    NemesisCrcTab,

    /// Output:
    NemesisOutputLabel,

    /// The root directory of the mod Export writes to, e.g. D:\MO2\mods\MyMod: by default the mod that was opened. Export writes <here>\Nemesis_Engine\mod\<mod code> (Nemesis_EngineExt for NemesisExt templates), replacing only the files this work read or wrote. The button on the left opens the directory.
    NemesisOutputHover,

    /// Mod code:
    NemesisModCodeLabel,

    /// e.g. mymod
    NemesisModCodeHint,

    /// A mod code is 4-6 ASCII letters/digits and not "vanilla".
    NemesisModCodeInvalid,

    /// Name:
    NemesisNameLabel,

    /// Author:
    NemesisAuthorLabel,

    /// Site:
    NemesisSiteLabel,

    /// Templates (Settings > template_dir):
    NemesisTemplatesLabel,

    /// Custom Template…
    NemesisCustomXml,

    /// Use your own template file (.xml, or a d-merge .bin). Its object ids must match the game's template.
    NemesisCustomXmlHover,

    /// No templates (.bin / .xml under meshes/) in the template directory. Check template_dir in the settings.
    NemesisNoTemplates,

    /// Add Behavior…
    NemesisAddBehavior,

    /// Export
    NemesisExport,

    /// Set the output directory and add a behavior first.
    NemesisExportDisabledHover,

    /// Exported
    NemesisExported,

    /// Export failed
    NemesisExportFailed,

    /// Some patches are rejected by d-merge's Nemesis parser
    NemesisRejected,

    /// Failed to load
    NemesisLoadFailed,

    /// Failed to save the work in progress
    NemesisSaveFailed,

    /// No template found for
    NemesisNoTemplateFor,

    /// Filter (id, class, name)
    NemesisFilterHint,

    /// Changed only
    NemesisChangedOnly,

    /// New
    NemesisNewObject,

    /// Duplicate
    NemesisDuplicate,

    /// Revert
    NemesisRevert,

    /// Discard the changes of this object (a new object is removed).
    NemesisRevertHover,

    /// Patch preview
    NemesisPatchPreview,

    /// No change
    NemesisNoChange,

    /// d-merge's Nemesis parser accepts this patch.
    NemesisParserOk,

    /// Add a behavior (template xml) to start.
    NemesisNoBehavior,

    /// Select an object on the left.
    NemesisSelectObject,

    /// Put this changed block back to the template's lines.
    NemesisRevertHunkHover,

    /// Previous / next change
    NemesisJumpDiffHover,

    /// Class of the new object (filter)
    NemesisClassFilterHint,

    /// Delete
    NemesisDeleteObject,

    /// Delete this new object.
    NemesisDeleteObjectHover,

    /// Copy id
    NemesisCopyId,

    /// Go to Definition
    NemesisGoToDefinition,

    /// Find References
    NemesisFindReferences,

    /// Tree
    NemesisTreeView,

    /// Show the objects as a tree from hkRootLevelContainer, following their #pointers. (A filter shows the flat list.)
    NemesisTreeViewHover,

    /// Not linked
    NemesisNotLinked,

    /// New objects no object in the tree points to. Nemesis ignores them: add their #id to a pointer field somewhere.
    NemesisNotLinkedHover,

    /// Expand all below
    NemesisExpandAllBelow,

    /// Collapse all below
    NemesisCollapseAllBelow,

    /// j / k: move, h / l: fold, o: open / close, Enter: back to the editor
    NemesisTreeKeysHint,

    /// Inline diff
    NemesisInlineDiff,

    /// Show the changes inside the editor: new lines in green, the old ones in red between them.
    NemesisInlineDiffHover,

    /// Close
    NemesisCloseTab,

    /// Close Others
    NemesisCloseOtherTabs,

    /// Close to the Right
    NemesisCloseTabsRight,

    /// Close All
    NemesisCloseAllTabs,

    /// Keep Open
    NemesisKeepOpen,

    /// Check Conflicts
    ConflictScan,

    /// Finds which enabled mods change the same fields. + overrides another mod, - is overridden. Right click a mod for the details. Every patch file of the enabled mods is read, so this takes about as long as patching.
    ConflictScanHover,

    /// In effect
    ConflictKept,

    /// Show Conflicts
    ConflictShow,

    /// Conflicts
    ConflictTitle,

    /// Checking…
    ConflictScanning,

    /// No conflicts with the enabled mods.
    ConflictNone,

    /// A mod was turned on after the last check: check again.
    ConflictStale,

    /// Filter by path
    ConflictFilterHint,

    /// Overrides others
    ConflictWinsHover,

    /// Overridden by others
    ConflictLosesHover,

    /// The value the merge keeps with the load order now.
    ConflictKeptHover,

    /// Use this
    ConflictUse,

    /// Decides on this value: it goes in the user patch, a mod that loads last and so wins, and the mod is written.
    ConflictUseHover,

    /// Edit…
    ConflictEdit,

    /// Takes this value into the user patch and opens it in the Patch Editor, to change it by hand. Export it there when done.
    ConflictEditHover,

    /// Hide
    ConflictHide,

    /// Leaves this operation of this mod out of the merge, as if the mod did not do it, like MO2's mohidden. The mod's files do not change. The flags change at once, what is generated at the next patch. Animation data goes by the whole file. Off with Apply User Patch.
    ConflictHideHover,

    /// Show
    ConflictUnhide,

    /// This operation counts again.
    ConflictUnhideHover,

    /// hidden
    ConflictHidden,

    /// The value is in the user patch
    ConflictResolved,

    /// Checked for conflicts
    ConflictDone,

    /// Apply User Patch
    UserPatchApply,

    /// Merges what the conflict tool decided, kept in <output>/.d_merge/user_patch, after every mod, and leaves out the operations hidden there. It is not in the mod list: it always loads last, like MO2's overwrite, and belongs to this output directory.
    UserPatchApplyHover,

    /// User Patch
    UserPatchTitle,

    /// All that the user patch has: the values chosen in conflicts and what was edited by hand.
    UserPatchHover,

    /// FNIS patches are generated: there is no file to take this from.
    ConflictFnisHover,

    /// Could not take the conflict's value
    NemesisResolveFailed,

    /// User Patch
    NemesisUserPatchName,

    /// Header
    NemesisAnimKindHeader,

    /// Clips
    NemesisAnimKindClip,

    /// Motions
    NemesisAnimKindMotion,

    /// Anim Sets
    NemesisAnimKindSet,

    /// New Anim Sets
    NemesisAnimKindNewSet,

    /// References:
    NemesisRefMods,

    /// The mods this one depends on (an add-on's base, the mods a user patch takes values from): the root of each, or its patch directory. Their new objects, events and variables can be pointed to and looked at. They are not edited or exported: they come with those mods when merging.
    NemesisRefModsHover,

    /// Read only: an object of a referenced mod.
    NemesisRefReadOnly,

    /// Open Recent
    NemesisOpenRecent,

    /// Nothing opened yet.
    NemesisOpenRecentEmpty,

    /// Behaviors
    NemesisBehaviorsSection,

    /// Search
    NemesisSearchTab,

    /// Text of every object, e.g. $eventID[moveStart]$ (Ctrl+Shift+F)
    NemesisSearchHint,

    /// Go Back
    NemesisGoBack,

    /// Go Forward
    NemesisGoForward,

    /// New Clips
    NemesisNewClips,

    /// New clips with numbered ids: the clip and motion files of each.
    NemesisNewClipsHover,

    /// Actor
    NemesisClipActor,

    /// Name
    NemesisClipName,

    /// Count
    NemesisClipCount,

    /// Create
    NemesisCreate,

    /// Follow the cursor: the preview scrolls to the editor's cursor line and highlights it.
    NemesisFollowCursor,

    /// Animation Data…
    NemesisAnimData,

    /// Uses the mod's root directory: generates the data of the clips of this work from their hkx (animationdatasinglefile), and adds the hkx you choose to the anim set files you choose (animationsetdatasinglefile).
    NemesisAnimDataHover,

    /// No clips: this work has no new or edited hkbClipGenerator.
    NemesisAnimDataNoClips,

    /// Clips (animationdatasinglefile)
    NemesisAnimDataClips,

    /// Anim sets (animationsetdatasinglefile)
    NemesisAnimDataSet,

    /// Adds every checked hkx to every checked txt: the hkx the game caches while that anim set is active. Checks in each project are kept.
    NemesisAnimDataSetHover,

    /// Project:
    NemesisAnimDataProject,

    /// No hkx in an animations directory under meshes in this mod.
    NemesisAnimDataNoHkx,

    /// Filter
    NemesisAnimDataFilter,

    /// All
    NemesisAnimDataAll,

    /// None
    NemesisAnimDataNone,

    /// Generate
    NemesisAnimDataGenerate,

    /// Generated animation data
    NemesisAnimDataDone,

    /// What identifies the mod: 4-6 ASCII letters/digits, not "vanilla", and not used by another mod. It is the name of its folder and the prefix of its new objects (#mymod$0), so it cannot be changed once the work is made.
    NemesisModCodeHover,

    /// The mod's name, as mod lists show it (name in info.ini).
    NemesisNameHover,

    /// Who made the mod (author in info.ini).
    NemesisAuthorHover,

    /// The URL of the mod's page (site in info.ini).
    NemesisSiteHover,

    /// Variable name
    NemesisNewVariableHint,

    /// The new variable's name, e.g. myVar: no spaces, and not one that exists, whatever its case. Patches refer to it as $variableID[myVar]$.
    NemesisNewVariableHover,

    /// The type of its value: BOOL, the whole numbers INT8 / INT16 / INT32, or REAL for decimals.
    NemesisNewVariableTypeHover,

    /// Its initial value, by type. BOOL: true or false. INT: a whole number, e.g. 1. REAL: a decimal, e.g. 0.2.
    NemesisNewVariableValueHover,

    /// Event name
    NemesisNewEventHint,

    /// The new event's name, e.g. myEvent: no spaces, and not one that exists, whatever its case. Patches refer to it as $eventID[myEvent]$.
    NemesisNewEventHover,

    /// The project the clips go to: search among those there are, or type a new one, as <name>~<number>, e.g. DefaultMale~1. A new project also needs its place in the list of projects, $header$/$header$.txt.
    NemesisClipActorHover,

    /// The clip's name, e.g. Attack, without / \ ~ $. With a count of 2 or more: Attack_0, Attack_1, ...
    NemesisClipNameHover,

    /// How many clips to add: their ids are numbered after what exists.
    NemesisClipCountHover,

    /// A directory to list the CRC32 of every .hkx under it, e.g. the mod's root D:\MO2\mods\MyMod. Any directory will do, or drop one here.
    CrcModDirHover,

    /// File
    NemesisFileMenu,

    /// Make a new patch mod, or open an existing one to edit it.
    NemesisStartHint,

    /// New
    NemesisNewWork,

    /// Make a new patch mod.
    NemesisNewWorkHover,

    /// Open a mod…
    NemesisOpenMod,

    /// Open a Nemesis mod to edit it: its root (where Nemesis_Engine is), e.g. D:\MO2\mods\Paraglider, or its patch directory, e.g. ...\Nemesis_Engine\mod\para. If it has been opened before, the work goes on with its edits that are not exported yet.
    NemesisOpenModHover,

    /// Close
    NemesisCloseWork,

    /// Back to the start screen. The work is kept, and comes back from Open Recent.
    NemesisCloseWorkHover,

    /// Cancel
    NemesisCancel,

    /// Choose the output directory.
    NemesisOutputMissing,

    /// There is already a work with this mod code: open it, or choose another code.
    NemesisWorkExists,

    /// Not a Nemesis mod (no Nemesis_Engine/mod/<code>)
    NemesisNotAMod,

    /// A work on another mod had this mod code: it was replaced by this mod
    NemesisWorkReplaced,

    /// Neither its work nor its mod is there any more, so it left the list
    NemesisRecentMissing,

    /// Where the work in progress is saved automatically, for each mod code: in the output directory of the Patch page. Decided by the app.
    NemesisCacheDirHover,

    /// Set the output directory on the Patch page first: the works and the User Patch are kept in it. The Skyrim Data directory can be used too, but something may be overwritten unintentionally.
    NemesisNeedPatchOutput,

    /// The User Patch is edited in its own window: the User Patch button.
    NemesisUserPatchReserved,

    /// Version:
    NemesisVersionLabel,

    /// The mod's version now, e.g. 1.2.0. Written to the info.ini of Nemesis_EngineExt only, with the mod codes of the references as deps.
    NemesisVersionHover,

    /// Decided by the output directory of the Patch page, as each profile has its own conflicts. It cannot be changed here. Edits are written a moment later.
    UserPatchOutputHover,

    /// Delete all
    UserPatchClear,

    /// Takes everything out of the User Patch and writes that: each conflict goes back to the mod that wins by the load order. Hidden operations stay. It cannot be undone.
    UserPatchClearHover,

    /// Really delete all
    UserPatchClearConfirm,

    /// All this mod's patch does loses to other mods, so it does nothing in the end. Its files are not changed: it still takes part in other mods' merges by the load order.
    ConflictDeadHover,

    /// Show all hidden
    ConflictUnhideAll,

    /// Every hidden operation, of all the mods, counts again.
    ConflictUnhideAllHover,

    /// Values of the mods
    NemesisMergeTitle,

    /// What each mod has at this conflict. Put together in the editor what is wanted: with an array, the elements of several mods are often wanted. Closed with its x, the patch preview comes back.
    NemesisMergeHover,

    /// Original
    NemesisMergeOriginal,

    /// Insert
    NemesisMergeInsert,

    /// Puts it below the line of the cursor in the editor. What it replaces stays: delete it there if it is not wanted.
    NemesisMergeInsertHover,

    /// It removes these lines.
    NemesisMergeRemoves,

    /// Next tab (Vim: Space l, gt, :tabn)
    NemesisNextTab,

    /// Previous tab (Vim: Space h, gT, :tabp)
    NemesisPreviousTab,

    /// Close tab (Vim: :tabc, :q)
    NemesisCloseCurrentTab,

    /// Show or hide the patch preview (Vim: Space p, :preview)
    NemesisTogglePreview,

    /// Show or hide the inline diff (Vim: Space d, :diff)
    NemesisToggleInlineDiff,

    /// Check
    NemesisCheck,

    /// Check all
    NemesisCheckAll,

    /// Expand all
    NemesisExpandAll,

    /// Collapse all
    NemesisCollapseAll,

    /// Checks without opening them. Animation data: counts that say something else than what is there, and clip ids that are not the ones in the file names; patches that edit are left out, as the merge counts for them. Behaviors: the fields and pointers of what the mod changes, and d-merge's parser on its patches.
    NemesisCheckHover,

    /// Check results
    NemesisCheckTitle,

    /// No problems found.
    NemesisCheckNone,

    /// Errors
    NemesisCheckErrors,

    /// Warnings
    NemesisCheckWarnings,

    /// Duplicate to
    NemesisDuplicateTo,

    /// Duplicate to...
    NemesisDuplicateToOther,

    /// New project
    NemesisNewProject,

    /// Files
    NemesisDuplicateFiles,

    /// Copies this patch to another project. DefaultMale~1 and DefaultFemale~1 usually have the same patches. A clip goes with its motion.
    NemesisDuplicateToHover,

    /// Open work data
    NemesisOpenCacheDir,

    /// The mod this work was read from. Click to open it.
    NemesisSourceHover,

    /// Reload from the mod
    NemesisReload,

    /// Reads the mod again. Discards the edits that are not exported.
    NemesisReloadHover,

    /// Read from
    NemesisModDirLabel,

    /// Focus Object Tree
    NemesisFocusTree,

    /// Rename (the patches using it are updated too)
    NemesisRename,

    /// Type a command
    EditorPaletteHint,

    /// Cut
    EditorCut,

    /// Copy
    EditorCopy,

    /// Paste
    EditorPaste,

    /// Select All
    EditorSelectAll,

    /// Command Palette
    EditorCommandPalette,

    /// Revert this change
    EditorRevertChange,

    /// Show Preview
    EditorShowPreview,

    /// Hide Preview
    EditorHidePreview,

    /// Show the selected object in the tree
    NemesisLocateSelected,

    /// Find
    EditorFind,

    /// Go to Line
    EditorGoToLine,

    /// Quick Fix
    EditorQuickFix,

    /// Format Document
    EditorFormat,

    /// Show Hover
    EditorShowHover,

    /// Trigger Suggest
    EditorCompletion,

    /// Objects
    NemesisObjectsTab,

    /// Events / Variables
    NemesisVarsTab,

    /// Events
    NemesisEvents,

    /// Variables
    NemesisVariables,

    /// Add
    NemesisAdd,

    /// This template has no event / variable data (hkbBehaviorGraphStringData).
    NemesisNoVars,

    /// Remove from the work
    NemesisRemoveBehavior,

    /// Removes this behavior and discards all its edits and new objects.
    NemesisRemoveBehaviorHover,

    /// NemesisExt
    NemesisForceExt,

    /// Write to Nemesis_EngineExt/mod/<code>/meshes/... even though Nemesis supports this template.
    NemesisForceExtHover,

    /// Nemesis does not support this template: it can only be written as NemesisExt.
    NemesisUnsupportedTemplate,

    /// ⚠ NemesisExt is read by d-merge only (not by Nemesis or Pandora).
    NemesisExtOnlyWarning,

    // ── CRC32 page ──────────────────────────────────────────────────────────
    /// Text / path:
    CrcTextLabel,

    /// e.g. meshes\actors\character\animations\mymod  or  ground_bite
    CrcTextHint,

    /// Skyrim's CRC32 of the lowercase text, with `/` as `\`. A file name is hashed without `.hkx`.
    CrcTextHover,

    /// Nemesis mod:
    CrcModDirLabel,

    /// Directory of a Nemesis patch mod (or drop it here)
    CrcModDirHint,

    /// No .hkx files found.
    CrcNoHkx,

    /// Copy
    CrcCopy,

    /// Filter
    CrcFilterHint,

    // ── Convert page ────────────────────────────────────────────────────────
    /// Inputs
    ConvertInputsLabel,

    /// Clear
    ConvertClearButton,

    /// Add Files…
    ConvertAddFiles,

    /// Add Directories…
    ConvertAddDirs,

    /// Add Root Directories…
    ConvertAddRootDirs,

    /// Drop .hkx / .xml / .json files or directories here
    ConvertDropHint,

    /// Directory Tree
    ConvertSelectionTree,

    /// Add root directories, then check the files to convert. The directory structure is kept in the output.
    ConvertSelectionTreeHover,

    /// Files & Directories
    ConvertSelectionFiles,

    /// Convert the listed files. A directory converts every file inside it.
    ConvertSelectionFilesHover,

    /// Next to each input
    ConvertOutputPlaceholder,

    /// Open the output directory
    ConvertOpenOutputHover,

    /// Output:
    ConvertOutputLabel,

    /// Output directory. If empty, each output is written next to its input.
    ConvertOutputHover,

    /// Output format:
    ConvertOutputFormatLabel,

    /// Convert
    ConvertButton,

    /// Converting...
    ConvertingButton,

    /// Conversion completed.
    ConvertDone,

    /// No input selected.
    ConvertNoInput,

    /// Select All
    ConvertSelectAll,

    /// Unselect All
    ConvertUnselectAll,

    /// Expand All
    ConvertExpandAll,

    /// Collapse All
    ConvertCollapseAll,

    // ── hkanno editor page ──────────────────────────────────────────────────
    /// Open
    HkannoOpenButton,

    /// Open .hkx / .xml files. (Drag & drop also works.)
    HkannoOpenHover,

    /// Open Input Directory
    HkannoTabOpenInputDir,

    /// Open Output Directory
    HkannoTabOpenOutputDir,

    /// Copy Input Path
    HkannoTabCopyInputPath,

    /// Copy Output Path
    HkannoTabCopyOutputPath,

    /// Close
    HkannoTabClose,

    /// Save
    HkannoSaveButton,

    /// Saved successfully.
    HkannoSaved,

    /// Save failed
    HkannoSaveFailed,

    /// Failed to load
    HkannoLoadFailed,

    /// Import .txt…
    HkannoImportTxt,

    /// Replace the annotations of this tab with a .txt file (hkanno text). A .txt dropped on the page does the same.
    HkannoImportTxtHover,

    /// Export .txt…
    HkannoExportTxt,

    /// Save the annotation text as a .txt file.
    HkannoExportTxtHover,

    /// Imported
    HkannoImported,

    /// Exported
    HkannoExported,

    /// Revert
    HkannoRevertButton,

    /// Discard the edits and restore the annotations of the original file?
    HkannoRevertConfirm,

    /// Apply
    HkannoApplyButton,

    /// Format
    HkannoFormatButton,

    /// Annotation
    HkannoAnnotationLabel,

    /// Preview (XML)
    HkannoPreviewTitle,

    /// Preview error
    HkannoPreviewError,

    /// Problems
    HkannoProblemsLabel,

    /// Drop .hkx / .xml files here
    HkannoDropHint,

    /// Open or drop .hkx / .xml files to edit their annotations.
    HkannoEmptyMessage,

    /// Language features
    HkannoLspOptionsTitle,

    /// Keymap:
    LspEditorMode,

    /// Key bindings of the editor. In Vim mode, `:w` saves.
    LspEditorModeHover,

    /// Completion
    LspCompletion,

    /// Suggestions while typing. Press Ctrl+Space to open them manually.
    LspCompletionHover,

    /// Code actions
    LspCodeAction,

    /// Quick fixes for some problems (e.g., IFrame key typos).
    LspCodeActionHover,

    /// Diagnostics
    LspDiagnostics,

    /// Underline errors and warnings.
    LspDiagnosticsHover,

    /// Formatter
    LspFormatter,

    /// Format the document (Shift+Alt+F. Vim: Space f, :fmt).
    LspFormatterHover,

    /// Semantic highlighting
    LspSemanticTokens,

    /// Parser-based coloring (e.g., PIE parameters).
    LspSemanticTokensHover,

    /// Hover
    LspHover,

    /// Show explanations when hovering a token.
    LspHoverHover,

    /// Inlay hints
    LspInlayHints,

    /// Show parameter names (time, x, y, ...) at the end of each line.
    LspInlayHintsHover,

    /// Signature help
    LspSignatureHelp,

    /// Show the signature of the event being typed.
    LspSignatureHelpHover,

    // NOTE: Using `skip_serializing` causes an error when attempting to serialize `Invalid`.
    /// Invalid key comes here when deserializing unknown strings.
    #[serde(other)]
    Invalid,
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct I18nMap(IndexMap<I18nKey, Cow<'static, str>, fast_hash::RandomState>);

impl I18nMap {
    pub fn new() -> Self {
        Self(IndexMap::default())
    }

    /// By placing settings in a fixed location within the Skyrim Data directory, you can handle switching between profiles in MO2.
    pub const PATH: &'static str = "./.d_merge/translation.json";

    /// Translate given key or fallback to default English.
    pub fn t(&self, key: I18nKey) -> &str {
        self.0.get(&key).map_or_else(|| key.default_eng(), |s| s.as_ref())
    }

    /// Try to load path & parse i18n map.
    ///
    /// # Errors
    /// failed to read json
    #[inline]
    pub fn load<P>(path: P) -> Result<Self, Error>
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();

        let content = std::fs::read_to_string(path).with_context(|_| ReadFileSnafu { path })?;
        let map: Self = sonic_rs::from_str(&content).with_context(|_| ParseJsonSnafu { path })?;
        Ok(map)
    }

    /// Try to load `./.d_merge/translation.json`.
    /// If not exists or failed to parse, fallback to `default_map()`.
    ///
    /// # Errors
    /// failed to read json
    pub fn load_with_fallback<P>(path: P) -> Result<Self, Error>
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();

        if !path.exists() {
            #[cfg(feature = "tracing")]
            tracing::info!("{} does not exist.", path.display());
            return Ok(Self::new());
        }

        Self::load(path)
    }

    /// Save translation.json
    /// # Errors
    /// failed to write json
    pub fn save<P>(path: P) -> Result<(), Error>
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();

        let map = Self(I18nKey::ALL.par_iter().map(|key| (*key, Cow::Borrowed(key.default_eng()))).collect());

        let text = sonic_rs::to_string_pretty(&map).context(SerializeJsonSnafu { path })?;
        std::fs::write(path, text).context(WriteFileSnafu { path })?;
        Ok(())
    }
}

/// The translated texts of the code editor's menus (right click, `Ctrl+Shift+P`).
///
/// In the order `egui_code_editor::EditorLabels::from_texts` takes them: this crate does not know
/// egui, so it cannot build that struct itself.
pub fn editor_label_texts(i18n: &I18nMap) -> [&str; 15] {
    [
        i18n.t(I18nKey::NemesisGoToDefinition),
        i18n.t(I18nKey::NemesisFindReferences),
        i18n.t(I18nKey::EditorFind),
        i18n.t(I18nKey::EditorGoToLine),
        i18n.t(I18nKey::EditorQuickFix),
        i18n.t(I18nKey::EditorFormat),
        i18n.t(I18nKey::EditorShowHover),
        i18n.t(I18nKey::EditorCompletion),
        i18n.t(I18nKey::EditorCut),
        i18n.t(I18nKey::EditorCopy),
        i18n.t(I18nKey::EditorPaste),
        i18n.t(I18nKey::EditorSelectAll),
        i18n.t(I18nKey::EditorCommandPalette),
        i18n.t(I18nKey::EditorRevertChange),
        i18n.t(I18nKey::EditorPaletteHint),
    ]
}
