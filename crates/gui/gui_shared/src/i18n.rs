use std::{borrow::Cow, path::Path};

use indexmap::IndexMap;
use rayon::prelude::*;
use snafu::ResultExt as _;

use crate::error::{Error, *};

/// A text of the UI. Its doc comment is the English text; `locales/egui/<lang>/translation.json` has
/// the others, by the `snake_case` name of the variant.
///
/// # Rule: alphabetical order
/// The variants here and the keys of every `translation.json` are always in alphabetical order of
/// that `snake_case` name (`Invalid` excepted: it stays last). A new key goes where its name sorts,
/// not after the others of its page: the names start with their page (`Nemesis*`, `Convert*`, ...),
/// which groups them anyway. `translations_follow_the_alphabetical_order` checks both.
///
/// e.g. `NemesisAddBehavior` (`nemesis_add_behavior`) comes before `NemesisAnimData`
/// (`nemesis_anim_data`), as `_` sorts before letters.
///
/// # Rule: Japanese (`ja-JP`)
/// - No half-width space between Japanese and ASCII: `modのファイル`, `.hkxファイル`, `プレビュー(XML)`.
///   A list marker keeps its own (`- 起動時`, `1. MOD`, `# 注意`).
/// - "User Patch" is `ユーザーパッチ`; "behavior" and "actor" stay English (`behavior`, `actor`).
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

    /// Enabled
    BackgroundImageEnabled,

    /// Whether the background image is shown.
    ///
    /// This feature exists to allow you to toggle the setting on and off while keeping the background image path specified.
    BackgroundImageEnabledHover,

    /// Background Image:
    BackgroundImageLabel,

    /// Reload background image.
    ///
    /// # Notes
    /// When you click this button, the `Enabled` checkbox will also be checked.
    BackgroundImageReloadHover,

    /// Background
    BackgroundTitleLabel,

    /// See Issues
    BugReportSeeIssues,

    /// Bug Report
    BugReportTitleLabel,

    /// Cancel
    CancelButton,

    /// Change Log:
    ChangeLogLabel,

    /// Clear
    ClearButton,

    /// Close
    CloseButton,

    /// Collapse all
    CollapseAllButton,

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

    /// All this mod's patch does loses to other mods, so it does nothing in the end. Its files are not changed: it still takes part in other mods' merges by the load order.
    ConflictDeadHover,

    /// Checked for conflicts
    ConflictDone,

    /// Resolve by hand…
    ConflictEdit,

    /// Opens this conflict in the user patch of the Patch Editor, from the value kept now, with each mod's value beside it. Export it there when done.
    ConflictEditHover,

    /// Filter by path
    ConflictFilterHint,

    /// FNIS patches are generated: there is no file to take this from.
    ConflictFnisHover,

    /// hidden
    ConflictHidden,

    /// Hide
    ConflictHide,

    /// Leaves this operation of this mod out of the merge, as if the mod did not do it, like MO2's mohidden. The mod's files do not change. The flags change at once, what is generated at the next patch. Animation data goes by the whole file. Off with Apply User Patch.
    ConflictHideHover,

    /// Keep all events
    ConflictKeepAllEvents,

    /// Each mod points this field to an array of events of its own. The user patch gets one array with the events of every mod, and points the field to it. The mods' files do not change.
    ConflictKeepAllEventsHover,

    /// In effect
    ConflictKept,

    /// The value the merge keeps with the load order now.
    ConflictKeptHover,

    /// Overridden by others
    ConflictLosesHover,

    /// No conflicts with the enabled mods.
    ConflictNone,

    /// The value is in the user patch
    ConflictResolved,

    /// Check Conflicts
    ConflictScan,

    /// Finds which enabled mods change the same fields. + overrides another mod, - is overridden. Right click a mod for the details. Every patch file of the enabled mods is read, so this takes about as long as patching.
    ConflictScanHover,

    /// Checking…
    ConflictScanning,

    /// Show Conflicts
    ConflictShow,

    /// A mod was turned on after the last check: check again.
    ConflictStale,

    /// Conflicts
    ConflictTitle,

    /// Show
    ConflictUnhide,

    /// Show all hidden
    ConflictUnhideAll,

    /// Every hidden operation, of all the mods, counts again.
    ConflictUnhideAllHover,

    /// This operation counts again.
    ConflictUnhideHover,

    /// Use this
    ConflictUse,

    /// Decides on this value: it goes in the user patch, a mod that loads last and so wins, and the mod is written.
    ConflictUseHover,

    /// Overrides others
    ConflictWinsHover,

    /// Add Directories…
    ConvertAddDirs,

    /// Add Files…
    ConvertAddFiles,

    /// Add Root Directories…
    ConvertAddRootDirs,

    /// Conversion completed.
    ConvertDone,

    /// Drop .hkx / .xml / .json files or directories here
    ConvertDropHint,

    /// Inputs
    ConvertInputsLabel,

    /// No input selected.
    ConvertNoInput,

    /// Open the output directory
    ConvertOpenOutputHover,

    /// Output directory. If empty, each output is written next to its input.
    ConvertOutputHover,

    /// Next to each input
    ConvertOutputPlaceholder,

    /// Files & Directories
    ConvertSelectionFiles,

    /// Convert the listed files. A directory converts every file inside it.
    ConvertSelectionFilesHover,

    /// Directory Tree
    ConvertSelectionTree,

    /// Add root directories, then check the files to convert. The directory structure is kept in the output.
    ConvertSelectionTreeHover,

    /// Unselect All
    ConvertUnselectAll,

    /// Converting...
    ConvertingButton,

    /// Directory of a Nemesis patch mod (or drop it here)
    CrcModDirHint,

    /// A directory to list the CRC32 of every .hkx under it, e.g. the mod's root D:\MO2\mods\MyMod. Any directory will do, or drop one here.
    CrcModDirHover,

    /// Nemesis mod:
    CrcModDirLabel,

    /// No .hkx files found.
    CrcNoHkx,

    /// e.g. meshes\actors\character\animations\mymod  or  ground_bite
    CrcTextHint,

    /// Skyrim's CRC32 of the lowercase text, with `/` as `\`. A file name is hashed without `.hkx`.
    CrcTextHover,

    /// Text / path:
    CrcTextLabel,

    /// Debug output
    DebugOutput,

    /// Outputs debug JSON and XML files.
    ///
    /// # Examples
    /// - `<output directory>/.d_merge/.debug/meshes/.../0_master.xml`
    /// - `<output directory>/.d_merge/.debug/meshes/.../0_master.json`
    /// - `<output directory>/.d_merge/.debug/patches/meshes/.../0_master.patch.json`
    DebugOutputHover,

    /// Close tab (Vim: Space c, :tabc, :q)
    EditorCloseTab,

    /// Command Palette
    EditorCommandPalette,

    /// Trigger Suggest
    EditorCompletion,

    /// Configure Keybinding
    EditorConfigureKeybinding,

    /// Copy
    EditorCopy,

    /// Cut
    EditorCut,

    /// Find
    EditorFind,

    /// Find References
    EditorFindReferences,

    /// Format Document
    EditorFormat,

    /// Go to Definition
    EditorGoToDefinition,

    /// Go to Line
    EditorGoToLine,

    /// Hide Preview
    EditorHidePreview,

    /// Next tab (Alt+L, Ctrl+PageDown; Vim: gt, :tabn)
    EditorNextTab,

    /// Search the text (> for commands, : to go to a line)
    EditorPaletteHint,

    /// Paste
    EditorPaste,

    /// Previous tab (Alt+H, Ctrl+PageUp; Vim: gT, :tabp)
    EditorPreviousTab,

    /// Quick Fix
    EditorQuickFix,

    /// Revert this change
    EditorRevertChange,

    /// Show Hover
    EditorShowHover,

    /// Show Preview
    EditorShowPreview,

    /// Show or hide the preview (Vim: Space p, :preview)
    EditorTogglePreview,

    /// Execution mode:
    ExecutionModeLabel,

    /// Expand all
    ExpandAllButton,

    /// Exported
    ExportedMessage,

    /// File
    FileLabel,

    /// Filter
    FilterLabel,

    /// All
    FilterTextAll,

    /// Font family:
    FontFamily,

    /// Font Path:
    FontFileLabel,

    /// Default
    FontModeDefault,

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

    /// Font Source:
    FontModeLabel,

    /// System
    FontModeSystem,

    /// Reloads and applies the currently selected font path.
    FontReloadHover,

    /// Font
    FontTitleLabel,

    /// Generates a dummy FNIS.esp file.
    ///
    /// # Examples:
    /// - XPMSE
    /// - FNIS Sexy Move
    /// - EatingSleepingDrinking
    GenerateFnisEspHover,

    /// Gen FNIS.esp
    GenerateFnisEspLabel,

    /// Help
    HelpButton,

    /// Annotation
    HkannoAnnotationLabel,

    /// Apply
    HkannoApplyButton,

    /// Drop .hkx / .xml files here
    HkannoDropHint,

    /// Open or drop .hkx / .xml files to edit their annotations.
    HkannoEmptyMessage,

    /// Export .txt…
    HkannoExportTxt,

    /// Save the annotation text as a .txt file.
    HkannoExportTxtHover,

    /// Format
    HkannoFormatButton,

    /// Import .txt…
    HkannoImportTxt,

    /// Replace the annotations of this tab with a .txt file (hkanno text). A .txt dropped on the page does the same.
    HkannoImportTxtHover,

    /// Imported
    HkannoImported,

    /// Open
    HkannoOpenButton,

    /// Open .hkx / .xml files. (Drag & drop also works.)
    HkannoOpenHover,

    /// Preview error
    HkannoPreviewError,

    /// Preview (XML)
    HkannoPreviewTitle,

    /// Problems
    HkannoProblemsLabel,

    /// Discard the edits and restore the annotations of the original file?
    HkannoRevertConfirm,

    /// Save
    HkannoSaveButton,

    /// Save failed
    HkannoSaveFailed,

    /// Saved successfully.
    HkannoSaved,

    /// Copy Input Path
    HkannoTabCopyInputPath,

    /// Copy Output Path
    HkannoTabCopyOutputPath,

    /// Open Input Directory
    HkannoTabOpenInputDir,

    /// Open Output Directory
    HkannoTabOpenOutputDir,

    /// This tab has unsaved changes. Save it first, or close it with ✖.
    HkannoTabUnsaved,

    /// I18n Path:
    I18nPathLabel,

    /// Reload the translation file. This is useful for previewing translations without restarting the application.
    I18nReloadJsonHover,

    /// Language & Translation
    I18nTitleLabel,

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

    /// Other commands with these keys:
    KeybindingAlsoBound,

    /// Don't Save
    KeybindingDontSave,

    /// Edit in keybindings.json
    KeybindingEditInJson,

    /// Preferences: Open Keyboard Shortcuts (JSON)
    KeybindingOpenJson,

    /// Press the desired key combination and then press Enter. Esc cancels.
    KeybindingRecordHint,

    /// Save
    KeybindingSave,

    /// Key binding saved to keybindings.json
    KeybindingSaved,

    /// Do you want to save the changes you made to keybindings.json? Your changes will be lost if you don't save them.
    KeybindingUnsaved,

    /// License:
    LicenseLabel,

    /// Failed to load
    LoadFailedMessage,

    /// Locked
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

    /// Log Level
    LogLevelLabel,

    /// Change the log directory to the specified path.
    ///
    /// # Notes
    /// Logs are rotated with a maximum of 5 files retained.
    /// The log directory will not change until this button is clicked
    /// (or the application is restarted).
    LogReloadHover,

    /// Logging
    LoggingTitleLabel,

    /// Code actions
    LspCodeAction,

    /// Quick fixes for some problems (e.g., IFrame key typos).
    LspCodeActionHover,

    /// Completion
    LspCompletion,

    /// Suggestions while typing. Press Ctrl+Space to open them manually.
    LspCompletionHover,

    /// Diagnostics
    LspDiagnostics,

    /// Underline errors and warnings.
    LspDiagnosticsHover,

    /// Keymap:
    LspEditorMode,

    /// Key bindings of the editor. In Vim mode, `:w` saves.
    LspEditorModeHover,

    /// Formatter
    LspFormatter,

    /// Format the document (Shift+Alt+F. Vim: Space f, :fmt).
    LspFormatterHover,

    /// Hover
    LspHover,

    /// Show explanations when hovering a token.
    LspHoverHover,

    /// Inlay hints
    LspInlayHints,

    /// Show parameter names (time, x, y, ...) at the end of each line.
    LspInlayHintsHover,

    /// Language features
    LspOptionsTitle,

    /// Semantic highlighting
    LspSemanticTokens,

    /// Parser-based coloring (e.g., PIE parameters).
    LspSemanticTokensHover,

    /// Signature help
    LspSignatureHelp,

    /// Show the signature of the event being typed.
    LspSignatureHelpHover,

    /// Manual
    ManualMode,

    /// Mode for use without virtual mod managers such as MO2.
    ///
    /// # Notes
    /// - Conflicts may occur if multiple versions of the same mod exist.
    /// - In manual mode, absolute paths are used as identifiers to prevent duplication. This makes configuration sharing unsuitable.
    ManualModeHover,

    /// Accept
    MergeAccept,

    /// Accept all
    MergeAcceptAll,

    /// Puts the lines of every mod in place of the conflict, each line once, as git's "accept both". The resolution is kept, and done again the next time this conflict comes.
    MergeAcceptAllHover,

    /// Puts this mod's lines in place of the conflict. The resolution is kept, and done again the next time this conflict comes.
    MergeAcceptHover,

    /// Resolved as before: this conflict was resolved the same way the last time.
    MergeReused,

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

    /// Add
    NemesisAdd,

    /// Add Behavior…
    NemesisAddBehavior,

    /// Animation Data…
    NemesisAnimData,

    /// Clips (animationdatasinglefile)
    NemesisAnimDataClips,

    /// Generated animation data
    NemesisAnimDataDone,

    /// Generate
    NemesisAnimDataGenerate,

    /// Uses the mod's root directory: generates the data of the clips of this work from their hkx (animationdatasinglefile), and adds the hkx you choose to the anim set files you choose (animationsetdatasinglefile).
    NemesisAnimDataHover,

    /// No clips: this work has no new or edited hkbClipGenerator.
    NemesisAnimDataNoClips,

    /// No hkx in an animations directory under meshes in this mod.
    NemesisAnimDataNoHkx,

    /// None
    NemesisAnimDataNone,

    /// Project:
    NemesisAnimDataProject,

    /// Anim sets (animationsetdatasinglefile)
    NemesisAnimDataSet,

    /// Adds every checked hkx to every checked txt: the hkx the game caches while that anim set is active. Checks in each project are kept.
    NemesisAnimDataSetHover,

    /// Clips
    NemesisAnimKindClip,

    /// Header
    NemesisAnimKindHeader,

    /// Motions
    NemesisAnimKindMotion,

    /// New Anim Sets
    NemesisAnimKindNewSet,

    /// Anim Sets
    NemesisAnimKindSet,

    /// Added the animations to the characters
    NemesisAnimNamesAdded,

    /// No character template for these actors
    NemesisAnimNamesNoCharacter,

    /// Also add the checked hkx to their characters
    NemesisAnimNamesToCharacters,

    /// A behavior plays a hkx only if its character lists it. Each checked hkx goes to the animationNames of the characters of its actor, told by its folder: e.g. meshes\actors\character\animations\Para\Equip.hkx -> Animations\Para\Equip.hkx in defaultmale and defaultfemale. Names already there are left.
    NemesisAnimNamesToCharactersHover,

    /// Who made the mod (author in info.ini).
    NemesisAuthorHover,

    /// Behaviors
    NemesisBehaviorsSection,

    /// Where the work in progress is saved automatically, for each mod code: in the output directory of the Patch page. Decided by the app.
    NemesisCacheDirHover,

    /// Changed only
    NemesisChangedOnly,

    /// This mod has several mod codes: which one to open?
    NemesisChooseModCode,

    /// Class of the new object (filter)
    NemesisClassFilterHint,

    /// Actor
    NemesisClipActor,

    /// The project the clips go to: search among those there are, or type a new one, as <name>~<number>, e.g. DefaultMale~1. A new project also needs its place in the list of projects, $header$/$header$.txt.
    NemesisClipActorHover,

    /// Count
    NemesisClipCount,

    /// How many clips to add: their ids are numbered after what exists.
    NemesisClipCountHover,

    /// The clip's name, e.g. Attack, without / \ ~ $. With a count of 2 or more: Attack_0, Attack_1, ...
    NemesisClipNameHover,

    /// Close All
    NemesisCloseAllTabs,

    /// Close Others
    NemesisCloseOtherTabs,

    /// Close to the Right
    NemesisCloseTabsRight,

    /// Close
    NemesisCloseWork,

    /// Back to the start screen. The work is kept, and comes back from Open Recent.
    NemesisCloseWorkHover,

    /// Collapse all below
    NemesisCollapseAllBelow,

    /// Copy id
    NemesisCopyId,

    /// Copy path
    NemesisCopyPath,

    /// CRC32
    NemesisCrcTab,

    /// Create
    NemesisCreate,

    /// Custom Template…
    NemesisCustomXml,

    /// Use your own template file (.xml, or a d-merge .bin). Its object ids must match the game's template.
    NemesisCustomXmlHover,

    /// Delete
    NemesisDeleteObject,

    /// Delete this new object.
    NemesisDeleteObjectHover,

    /// Duplicate
    NemesisDuplicate,

    /// Files
    NemesisDuplicateFiles,

    /// Duplicate to
    NemesisDuplicateTo,

    /// Copies this patch to another project. DefaultMale~1 and DefaultFemale~1 usually have the same patches. A clip goes with its motion.
    NemesisDuplicateToHover,

    /// Duplicate to...
    NemesisDuplicateToOther,

    /// Events
    NemesisEvents,

    /// Expand all below
    NemesisExpandAllBelow,

    /// Export
    NemesisExport,

    /// Set the output directory and add a behavior first.
    NemesisExportDisabledHover,

    /// Export failed
    NemesisExportFailed,

    /// What Export writes. Nemesis: Nemesis_Engine, and Nemesis_EngineExt for the templates Nemesis does not support. Nemesis Ext: every template to Nemesis_EngineExt (only d-merge reads it). Pandora v5: Pandora_Engine\mod\<mod code>\patches\*.xml and info.xml of Pandora Behaviour Engine v5, without the animation data. It is the same work: a mod read in one format can be written in another.
    NemesisExportFormatHover,

    /// Written for Pandora
    NemesisExportedPandora,

    /// ⚠ NemesisExt is read by d-merge only (not by Nemesis or Pandora).
    NemesisExtOnlyWarning,

    /// Filter (id, class, name)
    NemesisFilterHint,

    /// Focus Object Tree
    NemesisFocusTree,

    /// Follow the cursor: the preview scrolls to the editor's cursor line and highlights it.
    NemesisFollowCursor,

    /// Go Back
    NemesisGoBack,

    /// Go Forward
    NemesisGoForward,

    /// Inline diff
    NemesisInlineDiff,

    /// Show the changes inside the editor: new lines in green, the old ones in red between them.
    NemesisInlineDiffHover,

    /// Previous / next change (Vim: [c, ]c)
    NemesisJumpDiffHover,

    /// Keep Open
    NemesisKeepOpen,

    /// # Nemesis special syntax: Empty Array push pattern
    /// Adds elements to an array that is empty in the template. Its line stays as it is and the elements follow it, so the array is not replaced and several mods can add to it.
    ///
    /// # Example
    /// ```xml
    /// <hkparam name="eventNames" numelements="0"></hkparam>
    /// <!-- MOD_CODE ~mymod~ OPEN -->
    ///     <hkcstring>Hi</hkcstring>
    /// </hkparam>
    /// <!-- CLOSE -->
    /// ```
    ///
    /// # Notes
    /// - Not compatible with Pandora v4.
    NemesisKeptArrayHover,

    /// Show the selected object in the tree
    NemesisLocateSelected,

    /// What each mod has at this conflict. Put together in the editor what is wanted: with an array, the elements of several mods are often wanted. Closed with its x, the patch preview comes back.
    NemesisMergeHover,

    /// Puts it below the line of the cursor in the editor. What it replaces stays: delete it there if it is not wanted.
    NemesisMergeInsertHover,

    /// Original
    NemesisMergeOriginal,

    /// It removes these lines.
    NemesisMergeRemoves,

    /// Values of the mods
    NemesisMergeTitle,

    /// e.g. mymod
    NemesisModCodeHint,

    /// What identifies the mod: 4-6 ASCII letters/digits, not "vanilla", and not used by another mod. It is the name of its folder and the prefix of its new objects (#mymod$0). Once the work is made, "Rename…" of the project details changes it, and those ids with it.
    NemesisModCodeHover,

    /// A mod code is 4-6 ASCII letters/digits and not "vanilla".
    NemesisModCodeInvalid,

    /// Mod code:
    NemesisModCodeLabel,

    /// Read from
    NemesisModDirLabel,

    /// The mod's name, as mod lists show it (name in info.ini).
    NemesisNameHover,

    /// Name:
    NemesisNameLabel,

    /// Set the output directory on the Patch page first: the works and the User Patch are kept in it. The Skyrim Data directory can be used too, but something may be overwritten unintentionally.
    NemesisNeedPatchOutput,

    /// New Clips
    NemesisNewClips,

    /// New clips with numbered ids: the clip and motion files of each.
    NemesisNewClipsHover,

    /// Event name
    NemesisNewEventHint,

    /// The new event's name, e.g. myEvent: no spaces, and not one that exists, whatever its case. Patches refer to it as $eventID[myEvent]$.
    NemesisNewEventHover,

    /// New
    NemesisNewObject,

    /// New project
    NemesisNewProject,

    /// Variable name
    NemesisNewVariableHint,

    /// The new variable's name, e.g. myVar: no spaces, and not one that exists, whatever its case. Patches refer to it as $variableID[myVar]$.
    NemesisNewVariableHover,

    /// The type of its value: BOOL, the whole numbers INT8 / INT16 / INT32, or REAL for decimals.
    NemesisNewVariableTypeHover,

    /// Its initial value, by type. BOOL: true or false. INT: a whole number, e.g. 1. REAL: a decimal, e.g. 0.2.
    NemesisNewVariableValueHover,

    /// New
    NemesisNewWork,

    /// Make a new patch mod.
    NemesisNewWorkHover,

    /// Add a behavior (template xml) to start.
    NemesisNoBehavior,

    /// No change
    NemesisNoChange,

    /// No template found for
    NemesisNoTemplateFor,

    /// No templates (.bin / .xml under meshes/) in the template directory. Check template_dir in the settings.
    NemesisNoTemplates,

    /// This template has no event / variable data (hkbBehaviorGraphStringData).
    NemesisNoVars,

    /// Not a Nemesis mod (no Nemesis_Engine/mod/<code>)
    NemesisNotAMod,

    /// Not linked
    NemesisNotLinked,

    /// New objects no object in the tree points to. Nemesis ignores them: add their #id to a pointer field somewhere.
    NemesisNotLinkedHover,

    /// The patch file of the object in the mod, if this work patches it (where the export writes it); else the template it is read from.
    NemesisObjectFileHover,

    /// Objects
    NemesisObjectsTab,

    /// Open work data
    NemesisOpenCacheDir,

    /// Open a mod…
    NemesisOpenMod,

    /// Open a Nemesis mod to edit it: its root (where Nemesis_Engine is), e.g. D:\MO2\mods\Paraglider, or its patch directory, e.g. ...\Nemesis_Engine\mod\para. If it has been opened before, the work goes on with its edits that are not exported yet.
    NemesisOpenModHover,

    /// Open Recent
    NemesisOpenRecent,

    /// Nothing opened yet.
    NemesisOpenRecentEmpty,

    /// The root directory of the mod Export writes to, e.g. D:\MO2\mods\MyMod: by default the mod that was opened. Export writes <here>\Nemesis_Engine\mod\<mod code> (Nemesis_EngineExt for NemesisExt templates), replacing only the files this work read or wrote. The button on the left opens the directory.
    NemesisOutputHover,

    /// Choose the output directory.
    NemesisOutputMissing,

    /// Open the Pandora patch of this template, to edit it as text. It follows the edits of the objects, and they follow it.
    NemesisPandoraDocHover,

    /// These operations of Pandora give the object as it is edited here.
    NemesisPandoraOpsOk,

    /// d-merge's Nemesis parser accepts this patch.
    NemesisParserOk,

    /// Patch preview
    NemesisPatchPreview,

    /// Reading
    NemesisReadingWork,

    /// Neither its work nor its mod is there any more, so it left the list
    NemesisRecentMissing,

    /// References:
    NemesisRefMods,

    /// The mods this one depends on (an add-on's base, the mods a user patch takes values from): the root of each, or its patch directory. Their new objects, events and variables can be pointed to and looked at. They are not edited or exported: they come with those mods when merging.
    NemesisRefModsHover,

    /// Read only: an object of a referenced mod.
    NemesisRefReadOnly,

    /// Some patches are rejected by d-merge's Nemesis parser
    NemesisRejected,

    /// Reload from the mod
    NemesisReload,

    /// Reads the mod again. Discards the edits that are not exported.
    NemesisReloadHover,

    /// Remove from the work
    NemesisRemoveBehavior,

    /// Removes this behavior and discards all its edits and new objects.
    NemesisRemoveBehaviorHover,

    /// Rename (the patches using it are updated too)
    NemesisRename,

    /// Rename…
    NemesisRenameCode,

    /// Renamed the mod code
    NemesisRenameCodeDone,

    /// The mod code could not be renamed
    NemesisRenameCodeFailed,

    /// Gives this work another mod code (4-6 letters or digits): the ids of its new objects and clips change with it. What was exported under the old code goes at the next Export.
    NemesisRenameCodeHover,

    /// Renaming the mod code
    NemesisRenameCodeRunning,

    /// Could not take the conflict's value
    NemesisResolveFailed,

    /// Discard the changes of this object (a new object is removed).
    NemesisRevertHover,

    /// Put this changed block back to the template's lines.
    NemesisRevertHunkHover,

    /// Failed to save the work in progress
    NemesisSaveFailed,

    /// Which templates this work can patch. It can't be changed later. Nemesis: only those Nemesis knows. Nemesis Ext / Pandora v5: all of them, and template files of your own. Either can be exported in any format.
    NemesisScopeHover,

    /// Templates
    NemesisScopeLabel,

    /// Only the templates Nemesis knows (this work is a Nemesis one).
    NemesisScopeNemesisOnly,

    /// Text of every object, e.g. $eventID[moveStart]$ (Ctrl+Shift+F)
    NemesisSearchHint,

    /// Select an object on the left.
    NemesisSelectObject,

    /// The URL of the mod's page (site in info.ini).
    NemesisSiteHover,

    /// Site:
    NemesisSiteLabel,

    /// The mod this work was read from. Click to open it.
    NemesisSourceHover,

    /// Make a new patch mod, or open an existing one to edit it.
    NemesisStartHint,

    /// Templates (Settings > template_dir):
    NemesisTemplatesLabel,

    /// Show or hide the inline diff (Vim: Space d, :diff)
    NemesisToggleInlineDiff,

    /// j / k: move, h / l: fold, o: open / close, t: tree / list, c: changed only, u: unlinked only, Enter: back to the editor
    NemesisTreeKeysHint,

    /// Tree
    NemesisTreeView,

    /// Show the objects as a tree from hkRootLevelContainer, following their #pointers. (A filter shows the flat list.)
    NemesisTreeViewHover,

    /// Only the new objects nothing links to (Nemesis ignores them)
    NemesisUnlinkedOnly,

    /// The User Patch is edited in its own window: the User Patch button.
    NemesisUserPatchReserved,

    /// Variables
    NemesisVariables,

    /// Events / Variables
    NemesisVarsTab,

    /// The mod's version now, e.g. 1.2.0. Written to the info.ini of Nemesis_EngineExt only, with the mod codes of the references as deps.
    NemesisVersionHover,

    /// Version:
    NemesisVersionLabel,

    /// There is already a work with this mod code: open it, or choose another code.
    NemesisWorkExists,

    /// A work on another mod had this mod code: it was replaced by this mod
    NemesisWorkReplaced,

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

    /// Open Containing Folder
    OpenContainingFolder,

    /// Open selected path.
    OpenSelectedPathHover,

    /// Output dir:
    OutputDirLabel,

    /// Output format:
    OutputFormatLabel,

    /// Output:
    OutputLabel,

    /// Convert
    PageConvert,

    /// hkanno Editor
    PageHkanno,

    /// Patch
    PagePatch,

    /// Patch Editor
    PagePatchEditor,

    /// Takes the settings of Pandora Behaviour Engine into d-merge's VFS mode, or gives them to it.
    PandoraButtonHover,

    /// The directory with Pandora's Settings.json and Pandora_Engine\ActiveMods.json. Import reads them from here and Export writes them here. Only used for these two buttons. The button on the left opens the directory.
    PandoraDirHover,

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

    /// Select the Pandora version used for import or export.
    ///
    /// - `v4.4.0-beta`: Compatible with `v5.0.0`.
    PandoraTargetVersionHover,

    /// Target version:
    PandoraTargetVersionLabel,

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

    /// Lenient
    ParserModeLenient,

    /// Strict
    ParserModeStrict,

    /// Patch
    PatchButton,

    /// Updating list...
    PatchFetchingButton,

    /// No problems have been found in the work.
    ProblemsNone,

    /// Reload
    ReloadButton,

    /// Removing the `<output_dir>/meshes` directory...
    RemovingMeshesMessage,

    /// Create
    RepeatCreate,

    /// Makes many new objects from this one, one per line of values: {{name}} in the template takes the value of the column name, {{i}} the line number from 0.
    RepeatHover,

    /// objects to create
    RepeatObjects,

    /// Template ({{name}}: the value of the column, {{i}}: the line number)
    RepeatTemplate,

    /// Repeat…
    RepeatTitle,

    /// Values (the first line names the columns; tabs or commas between them)
    RepeatValues,

    /// Reveal in File Explorer
    RevealInFileExplorer,

    /// Reveal in Finder
    RevealInFinder,

    /// Revert
    RevertButton,

    /// Back to the editor
    RevisionViewClose,

    /// Output format for hkx.
    /// - SkyrimLE/Enderal: win32
    /// - SkyrimSE/SkyrimVR/EnderalSE: amd64
    ///
    /// # Notes
    /// For Windows ver. user: When changing settings in vfs mode, it will automatically attempt to locate and modify the Skyrim Data Directory from the registry.
    RuntimeTargetHover,

    /// hits
    SearchHitsLabel,

    /// Search
    SearchLabel,

    /// objects
    SearchObjects,

    /// Use a regular expression (ignoring case)
    SearchRegexHover,

    /// results in
    SearchResultsIn,

    /// Select All
    SelectAll,

    /// Select
    SelectButton,

    /// selected
    SelectedLabel,

    /// Blame
    SideBlame,

    /// Each line of this object with the revision that last changed it.
    SideBlameHover,

    /// The work has changes since its last revision: commit or discard them before switching the branch.
    SideBranchHasChanges,

    /// The branch the work is on. Another one puts the work as it is at its newest revision.
    SideBranchHover,

    /// Changes
    SideChanges,

    /// Commit
    SideCommit,

    /// Keeps the changes as a revision of this work, with the message (Ctrl+Enter in the field). The history is in the work data, not in the mod.
    SideCommitHover,

    /// Message
    SideCommitMessageHint,

    /// Compare with the revision before
    SideComparePrevious,

    /// Compare with the work now
    SideCompareWorking,

    /// Discard changes
    SideDiscard,

    /// Explorer
    SideExplorer,

    /// History
    SideHistory,

    /// New branch (Enter)
    SideNewBranch,

    /// Nothing is changed yet.
    SideNoChanges,

    /// No revision changed this object.
    SideNoObjectHistory,

    /// Animation data has no events or variables.
    SideNoVariables,

    /// Object history
    SideObjectHistory,

    /// Restore this object to this revision
    SideRestoreObject,

    /// Restore the work to this revision
    SideRestoreRevision,

    /// Puts every object as it was at this revision. The history stays: what differs from the last revision is a change, to commit or to discard.
    SideRestoreRevisionHover,

    /// Template:
    SideTemplate,

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

    /// 🎨 Custom
    ThemeSelectCustom,

    /// 🌙 Dark
    ThemeSelectDark,

    /// ☀ Light
    ThemeSelectLight,

    /// 💻 System
    ThemeSelectSystem,

    /// Apply User Patch
    UserPatchApply,

    /// Merges what the conflict tool decided, kept in <output>/.d_merge/user_patch, after every mod, and leaves out the operations hidden there. It is not in the mod list: it always loads last, like MO2's overwrite, and belongs to this output directory.
    UserPatchApplyHover,

    /// Delete all
    UserPatchClear,

    /// Delete all of the User Patch?
    UserPatchClearConfirm,

    /// Takes everything out of the User Patch and writes that: each conflict goes back to the mod that wins by the load order. Hidden operations stay. It cannot be undone.
    UserPatchClearHover,

    /// Export as mod…
    UserPatchExportMod,

    /// Writes the user patch as a mod of its own (code upatch) to the folder chosen, e.g. into MO2's mods to manage or share it there: as Nemesis, Nemesis Ext or Pandora v5. Its own place and what is merged stay as they are.
    UserPatchExportModHover,

    /// All that the user patch has: the values chosen in conflicts and what was edited by hand.
    UserPatchHover,

    /// Decided by the output directory of the Patch page, as each profile has its own conflicts. It cannot be changed here. Edits are written a moment later.
    UserPatchOutputHover,

    /// Nemesis does not know these templates: their patches go to Nemesis_EngineExt, which only d-merge reads
    UserPatchOutsideNemesis,

    /// User Patch
    UserPatchTitle,

    /// VFS
    VfsMode,

    /// When booting using MO2's VFS, etc.
    VfsModeHover,

    // NOTE: Using `skip_serializing` causes an error when attempting to serialize `Invalid`.
    /// Invalid key comes here when deserializing unknown strings.
    #[serde(other)]
    Invalid,
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct I18nMap(IndexMap<I18nKey, Cow<'static, str>, fast_hash::RandomState>);

impl I18nMap {
    #[inline]
    pub fn new() -> Self {
        Self(IndexMap::default())
    }

    /// By placing settings in a fixed location within the Skyrim Data directory, you can handle switching between profiles in MO2.
    pub const PATH: &'static str = "./.d_merge/translation.json";

    /// Translate given key or fallback to default English.
    #[inline]
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

/// The name VS Code gives "show the file in the file manager" on this OS: Reveal in File Explorer
/// (Windows), Reveal in Finder (macOS), Open Containing Folder (Linux).
pub const REVEAL_IN_FILE_MANAGER: I18nKey = if cfg!(windows) {
    I18nKey::RevealInFileExplorer
} else if cfg!(target_os = "macos") {
    I18nKey::RevealInFinder
} else {
    I18nKey::OpenContainingFolder
};

/// The translated texts of a tab's right-click menu, in the order `egui_widgets::TabMenu::from_texts`
/// takes them: Keep Open, Close, Close Others, Close to the Right, Close All.
pub fn tab_menu_texts(i18n: &I18nMap) -> [&str; 5] {
    [
        i18n.t(I18nKey::NemesisKeepOpen),
        i18n.t(I18nKey::CloseButton),
        i18n.t(I18nKey::NemesisCloseOtherTabs),
        i18n.t(I18nKey::NemesisCloseTabsRight),
        i18n.t(I18nKey::NemesisCloseAllTabs),
    ]
}

/// The translated texts of the code editor's menus (right click, `Ctrl+Shift+P`).
///
/// In the order `egui_code_editor::EditorLabels::from_texts` takes them: this crate does not know
/// egui, so it cannot build that struct itself.
pub fn editor_label_texts(i18n: &I18nMap) -> [&str; 16] {
    [
        i18n.t(I18nKey::EditorGoToDefinition),
        i18n.t(I18nKey::EditorFindReferences),
        i18n.t(I18nKey::EditorFind),
        i18n.t(I18nKey::EditorGoToLine),
        i18n.t(I18nKey::EditorQuickFix),
        i18n.t(I18nKey::EditorFormat),
        i18n.t(I18nKey::EditorShowHover),
        i18n.t(I18nKey::EditorCompletion),
        i18n.t(I18nKey::EditorCut),
        i18n.t(I18nKey::EditorCopy),
        i18n.t(I18nKey::EditorPaste),
        i18n.t(I18nKey::SelectAll),
        i18n.t(I18nKey::EditorCommandPalette),
        i18n.t(I18nKey::EditorRevertChange),
        i18n.t(I18nKey::EditorPaletteHint),
        i18n.t(I18nKey::EditorConfigureKeybinding),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The `snake_case` name of `key`, as `translation.json` has it. e.g. `NemesisAddBehavior` -> `nemesis_add_behavior`
    fn name_of(key: I18nKey) -> String {
        sonic_rs::to_string(&key).unwrap().trim_matches('"').to_owned()
    }

    /// See "Rule: alphabetical order" of [`I18nKey`]: the variants, and the keys of every
    /// `locales/egui/<lang>/translation.json`, each once and none unknown (a renamed key must not leave a
    /// translation nothing reads).
    #[test]
    fn translations_follow_the_alphabetical_order() {
        let names: Vec<String> =
            I18nKey::ALL.iter().filter(|key| **key != I18nKey::Invalid).map(|key| name_of(*key)).collect();
        for pair in names.windows(2) {
            assert!(pair[0] < pair[1], "I18nKey: `{}` must come before `{}`", pair[1], pair[0]);
        }

        let locales = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../locales/egui");
        for entry in std::fs::read_dir(&locales).unwrap().flatten() {
            let path = entry.path().join("translation.json");
            let text = std::fs::read_to_string(&path).unwrap();
            let map: IndexMap<String, String> = sonic_rs::from_str(&text).unwrap();
            let keys: Vec<&str> = map.keys().map(String::as_str).collect();
            for key in &keys {
                assert!(names.iter().any(|name| name == key), "{}: unknown key `{key}`", path.display());
            }
            for pair in keys.windows(2) {
                assert!(pair[0] < pair[1], "{}: `{}` must come before `{}`", path.display(), pair[1], pair[0]);
            }
            // A map keeps one of a key written twice: count the lines of keys.
            let lines = text.lines().filter(|line| line.trim_start().starts_with('"')).count();
            assert_eq!(keys.len(), lines, "{}: a key is written twice", path.display());
        }
    }
}
