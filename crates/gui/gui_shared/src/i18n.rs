use std::{borrow::Cow, path::Path};

use indexmap::IndexMap;
use rayon::prelude::*;
use snafu::ResultExt as _;

use crate::error::{Error, *};

/// A text of the UI: its doc comment is the English, `locales/egui/<lang>/translation.json` the others
/// by its `snake_case` name. Rules (also in `locales/readme.md`):
/// - Reuse a key first. One likely to be reused is named for no page from the start (`ModsDirLabel`).
/// - Alphabetical order of the `snake_case` name, `Invalid` last (a test checks it).
/// - Names end in `Label`, `Hover`, `Hint` by kind.
/// - "directory" (`dir` in a label), never "folder".
/// - A long hover: a sentence or two, then `# Notes` / `# Examples` / ... with `- items` (no full
///   stop) and `` `paths` ``. It reads as Rust docs and is drawn as Markdown (`on_hover_markdown`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(serde::Serialize, serde::Deserialize, gui_shared_derive::I18n)]
#[serde(rename_all = "snake_case")]
pub enum I18nKey {
    /// Add
    AddButton,

    /// Clips (animationdatasinglefile)
    AnimDataClips,

    /// Generated animation data
    AnimDataDone,

    /// Files
    AnimDataDuplicateFiles,

    /// Generate
    AnimDataGenerate,

    /// New Clips
    AnimDataNewClips,

    /// New clips with numbered ids: the clip and motion files of each.
    AnimDataNewClipsHover,

    /// New project
    AnimDataNewProject,

    /// No clips: this work has no new or edited hkbClipGenerator.
    AnimDataNoClips,

    /// No hkx in an animations directory under meshes in this mod.
    AnimDataNoHkx,

    /// None
    AnimDataNone,

    /// Animation Data…
    AnimDataOpen,

    /// Generates the animation data of this work from the mod's root directory.
    ///
    /// # Targets
    /// - `animationdatasinglefile`: the clip data of this work's clips, from their hkx
    /// - `animationsetdatasinglefile`: the hkx you choose, added to the anim sets you choose
    AnimDataOpenHover,

    /// Project:
    AnimDataProject,

    /// Anim sets (animationsetdatasinglefile)
    AnimDataSet,

    /// Adds every checked hkx to every checked txt.
    ///
    /// # Notes
    /// - These are the hkx the game caches while that anim set is active
    /// - The checks are kept for each project
    AnimDataSetHover,

    /// Clips
    AnimKindClip,

    /// Header
    AnimKindHeader,

    /// Motions
    AnimKindMotion,

    /// New Anim Sets
    AnimKindNewSet,

    /// Anim Sets
    AnimKindSet,

    /// Added the animations to the characters
    AnimNamesAdded,

    /// No character template for these actors
    AnimNamesNoCharacter,

    /// Also add the checked hkx to their characters
    AnimNamesToCharacters,

    /// Adds each checked hkx to the `animationNames` of the characters of its actor, told by its directory. A behavior plays a hkx only if its character lists it.
    ///
    /// # Examples
    /// - `meshes\actors\character\animations\Para\Equip.hkx`
    ///   -> `Animations\Para\Equip.hkx` in defaultmale and defaultfemale
    ///
    /// # Notes
    /// - Names already there are left
    AnimNamesToCharactersHover,

    /// Apply
    ApplyButton,

    /// Gets the bone count from each `skeleton.xml` distributed by mods such as XPMSSE, and extends the corresponding character hkx by it. The same as `Auto Skeleton Patch` and FNIS's `SKELETON arm fix`.
    ///
    /// # Recommended
    /// - XPMSSE (v5.06) and other bone-expanding mods
    ///   -> Without this, arm animations such as blocking while moving may not work
    ///
    /// # Targets
    /// - `<Skyrim Data directory>/meshes/actors/character/character assets/skeleton.xml`
    ///   -> `defaultmale.hkx`
    /// - `<Skyrim Data directory>/meshes/actors/character/character assets female/skeleton_female.xml`
    ///   -> `defaultfemale.hkx`
    ///
    /// # Processing
    /// - Missing bone indices -> extend `bonePairMap`
    /// - Missing bone weights -> append `0.0` / `1.0` to the corresponding weight arrays
    ///
    /// # Notes
    /// - Manual mode with a glob Skyrim Data directory (e.g. `MO2/mods/*`) may patch wrongly
    ///   -> When several `skeleton.xml` are found, the first is used, and a warning is logged
    ArmFixHover,

    /// Arm fix
    ArmFixLabel,

    /// Author:
    AuthorLabel,

    /// Auto Detect
    AutoDetectButton,

    /// Detects the Skyrim Data directory for the selected output format.
    ///
    /// # Notes
    /// - It uses the Steam registry: Skyrim must have been launched at least once
    AutoDetectHover,

    /// Auto remove `meshes`
    AutoRemoveMeshes,

    /// Deletes the previous output before patching.
    ///
    /// # Targets
    /// - `<output directory>/.d_merge/.debug`
    /// - `<output directory>/meshes`
    /// - `<output directory>/SKSE/Plugins/fnis_aa/config.json`
    /// - `<output directory>/FNIS.esp`
    ///
    /// # Notes
    /// - The output directory is not the Skyrim Data directory
    ///   -> Strongly recommended: the previous output may cause unintended behavior
    /// - The output directory is the Skyrim Data directory
    ///   -> Not recommended: the deletion is skipped, so as not to break mods by deleting `meshes`
    AutoRemoveMeshesHover,

    /// Auto run
    AutoRun,

    /// Each time the mod list is updated, enables all mods and patches.
    ///
    /// # Mod list updates
    /// - At startup
    /// - When the reload button is pressed
    /// - When switching between VFS and manual mode
    /// - When the Skyrim Data directory is changed
    ///
    /// # Notes
    /// - The window does not close by itself, so that auto-run can still be turned off
    AutoRunHover,

    /// Enabled
    BackgroundImageEnabled,

    /// Whether the background image is shown, keeping its path.
    BackgroundImageEnabledHover,

    /// Background Image:
    BackgroundImageLabel,

    /// Reloads the background image.
    ///
    /// # Notes
    /// - The `Enabled` checkbox is checked too
    BackgroundImageReloadHover,

    /// Background
    BackgroundTitleLabel,

    /// Add Behavior…
    BehaviorsAdd,

    /// Custom Template…
    BehaviorsCustomXml,

    /// Uses your own template file (`.xml`, or a d-merge `.bin`).
    ///
    /// # Notes
    /// - Its object ids must match the game's template
    BehaviorsCustomXmlHover,

    /// Add a behavior (template xml) to start.
    BehaviorsNone,

    /// Remove from the work
    BehaviorsRemove,

    /// Removes this behavior and discards all its edits and new objects.
    BehaviorsRemoveHover,

    /// Behaviors
    BehaviorsTitle,

    /// See Issues
    BugReportSeeIssues,

    /// Bug Report
    BugReportTitleLabel,

    /// Cancel
    CancelButton,

    /// Change Log:
    ChangeLogLabel,

    /// Changed only
    ChangedOnly,

    /// Blame
    ChangesBlame,

    /// Each line of this object with the revision that last changed it.
    ChangesBlameHover,

    /// The work has changes since its last revision: commit or discard them before switching the branch.
    ChangesBranchHasChanges,

    /// The branch the work is on. Another one puts the work as it is at its newest revision.
    ChangesBranchHover,

    /// Commit
    ChangesCommit,

    /// Keeps the changes as a revision of this work, with the message (`Ctrl+Enter` in the field).
    ///
    /// # Notes
    /// - The history is in the work data, not in the mod
    ChangesCommitHover,

    /// Message
    ChangesCommitMessageHint,

    /// Compare with the revision before
    ChangesComparePrevious,

    /// Compare with the work now
    ChangesCompareWorking,

    /// Discard changes
    ChangesDiscard,

    /// History
    ChangesHistory,

    /// New branch (Enter)
    ChangesNewBranch,

    /// No revision changed this object.
    ChangesNoObjectHistory,

    /// Nothing is changed yet.
    ChangesNone,

    /// Object history
    ChangesObjectHistory,

    /// Restore this object to this revision
    ChangesRestoreObject,

    /// Restore the work to this revision
    ChangesRestoreRevision,

    /// Puts every object as it was at this revision.
    ///
    /// # Notes
    /// - The history stays: what differs from the last revision is a change, to commit or to discard
    ChangesRestoreRevisionHover,

    /// Changes
    ChangesTab,

    /// Class of the new object (filter)
    ClassFilterHint,

    /// Clear
    ClearButton,

    /// Actor
    ClipActor,

    /// The project the clips go to: search among those there are, or type a new one as `<name>~<number>`.
    ///
    /// # Examples
    /// - `DefaultMale~1`
    ///
    /// # Notes
    /// - A new project also needs its place in the list of projects, `$header$/$header$.txt`
    ClipActorHover,

    /// Count
    ClipCount,

    /// How many clips to add: their ids are numbered after what exists.
    ClipCountHover,

    /// The clip's name.
    ///
    /// # Examples
    /// - `Attack`
    /// - With a count of 2 or more: `Attack_0`, `Attack_1`, ...
    ///
    /// # Notes
    /// - `/` `\` `~` `$` can't be used
    ClipNameHover,

    /// Close
    CloseButton,

    /// Collapse all below
    CollapseAllBelow,

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

    /// Resolve at once
    ConflictBulkButton,

    /// Conflicts decided without picking one mod's value, all of them at once.
    ///
    /// # Notes
    /// - `enterNotifyEvents` / `exitNotifyEvents`: the events of every mod are kept
    /// - Only those checked go into the user patch; it is written once
    /// - `✓`: the user patch decides it already
    ConflictBulkHover,

    /// Resolved at once
    ConflictBulkResolvedMessage,

    /// All this mod's patch does loses to other mods: it does nothing in the end.
    ///
    /// # Notes
    /// - Its files are not changed: it still takes part in other mods' merges by the load order
    ConflictDeadHover,

    /// Checked for conflicts
    ConflictDone,

    /// Resolve by hand…
    ConflictEdit,

    /// Opens this conflict in the user patch of the Patch Editor, from the value kept now, with each mod's value beside it.
    ///
    /// # Notes
    /// - Write it there when done
    ConflictEditHover,

    /// Filter by path
    ConflictFilterHint,

    /// FNIS patches are generated: there is no file to take this from.
    ConflictFnisHover,

    /// hidden
    ConflictHidden,

    /// Hide
    ConflictHide,

    /// Leaves this operation of this mod out of the merge, as if the mod did not do it (like MO2's mohidden).
    ///
    /// # Notes
    /// - The mod's files do not change
    /// - The flags change at once; what is generated, at the next patch
    /// - Animation data goes by the whole file
    /// - Off while Apply User Patch is off
    ConflictHideHover,

    /// Keep all events
    ConflictKeepAllEvents,

    /// The user patch gets one array with the events of every mod, and points this field to it.
    ///
    /// # Notes
    /// - Each mod points this field to an array of events of its own
    /// - The mods' files do not change
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

    /// Resolved in the user patch
    ConflictResolvedHover,

    /// Check Conflicts
    ConflictScan,

    /// Finds which enabled mods change the same fields.
    ///
    /// # Flags
    /// - `+`: overrides another mod
    /// - `-`: is overridden by another mod
    /// - `✓`: resolved in the user patch
    ///
    /// # Notes
    /// - Right click a mod for the details
    /// - Every patch file of the enabled mods is read: this takes about as long as patching
    ConflictScanHover,

    /// Checking…
    ConflictScanning,

    /// Show Conflicts
    ConflictShow,

    /// A mod was turned on after the last check: check again.
    ConflictStale,

    /// Could not take the conflict's value
    ConflictTakeFailed,

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

    /// Decides on this value: it goes in the user patch, which loads last and so wins, and the user patch is written.
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

    /// Add root directories, then check the files to convert.
    ///
    /// # Notes
    /// - The directory structure is kept in the output
    ConvertSelectionTreeHover,

    /// Unselect All
    ConvertUnselectAll,

    /// Converting...
    ConvertingButton,

    /// Copy id
    CopyId,

    /// Copy path
    CopyPath,

    /// Directory of a Nemesis patch mod (or drop it here)
    CrcModDirHint,

    /// Lists the CRC32 of every `.hkx` under this directory. Any directory will do, or drop one here.
    ///
    /// # Examples
    /// - The mod's root: `D:\MO2\mods\MyMod`
    CrcModDirHover,

    /// Nemesis mod:
    CrcModDirLabel,

    /// No .hkx files found.
    CrcNoHkx,

    /// CRC32
    CrcTab,

    /// e.g. meshes\actors\character\animations\mymod  or  ground_bite
    CrcTextHint,

    /// Skyrim's CRC32 of the lowercase text, with `/` as `\`.
    ///
    /// # Notes
    /// - A file name is hashed without `.hkx`
    CrcTextHover,

    /// Text / path:
    CrcTextLabel,

    /// Create
    CreateButton,

    /// Debug output
    DebugOutput,

    /// Writes the JSON and XML used for debugging.
    ///
    /// # Examples
    /// - `<output directory>/.d_merge/.debug/meshes/.../0_master.xml`
    /// - `<output directory>/.d_merge/.debug/meshes/.../0_master.json`
    /// - `<output directory>/.d_merge/.debug/patches/meshes/.../0_master.patch.json`
    DebugOutputHover,

    /// Delete
    DeleteButton,

    /// Duplicate
    DuplicateButton,

    /// Duplicate to
    DuplicateTo,

    /// Copies this patch to another project.
    ///
    /// # Notes
    /// - `DefaultMale~1` and `DefaultFemale~1` usually have the same patches
    /// - A clip goes with its motion
    DuplicateToHover,

    /// Duplicate to...
    DuplicateToOther,

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

    /// Execution mode:
    ExecutionModeLabel,

    /// Expand all below
    ExpandAllBelow,

    /// Expand all
    ExpandAllButton,

    /// Focus Object Tree
    ExplorerFocusTree,

    /// j / k: move, h / l: fold, o: open / close, t: tree / list, c: changed only, u: unlinked only, Enter: back to the editor
    ExplorerKeysHint,

    /// Show the selected object in the tree
    ExplorerLocateSelected,

    /// Not linked
    ExplorerNotLinked,

    /// New objects that no object in the tree points to: Nemesis ignores them.
    ///
    /// # Notes
    /// - Add their `#id` to a pointer field somewhere
    ExplorerNotLinkedHover,

    /// Objects
    ExplorerObjects,

    /// Explorer
    ExplorerTab,

    /// Shows the objects as a tree from `hkRootLevelContainer`, following their `#pointers`.
    ///
    /// # Notes
    /// - While filtering, the flat list is shown
    ExplorerTreeViewHover,

    /// Only the new objects nothing links to (Nemesis ignores them)
    ExplorerUnlinkedOnly,

    /// Export
    ExportButton,

    /// The root directory of the mod Export writes to. By default, the mod that was opened.
    ///
    /// # Examples
    /// - `D:\MO2\mods\MyMod` -> `D:\MO2\mods\MyMod\Nemesis_Engine\mod\<mod code>`
    ///
    /// # Notes
    /// - Templates Nemesis does not support go to `Nemesis_EngineExt`
    /// - Only the files this work read or wrote are replaced
    ExportDirHover,

    /// Set the output directory and add a behavior first.
    ExportDisabledHover,

    /// Export failed
    ExportFailed,

    /// What Export writes.
    ///
    /// - Nemesis:
    ///   `Nemesis_Engine`, and `Nemesis_EngineExt` for the templates Nemesis does not support
    /// - Nemesis Ext:
    ///   every template to `Nemesis_EngineExt` (only d-merge reads it)
    /// - Pandora v5:
    ///   `Pandora_Engine\mod\<mod code>\patches\*.xml` and `info.xml` of Pandora Behaviour Engine v5, without the animation data
    ///
    /// # Notes
    /// - It is the same work: a mod read in one format can be written in another
    ExportFormatHover,

    /// Exported
    ExportedMessage,

    /// Written for Pandora
    ExportedPandora,

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

    /// How the font is loaded.
    ///
    /// - Default:
    ///   the application's built-in font
    /// - System:
    ///   a font family installed in the system
    /// - File:
    ///   the font file you choose
    ///
    /// # Notes
    /// - Fonts differ in text width and spacing: table layouts, column alignment and the whole UI may look different
    FontModeHover,

    /// Font Source:
    FontModeLabel,

    /// System
    FontModeSystem,

    /// Reloads and applies the currently selected font path.
    FontReloadHover,

    /// Font
    FontTitleLabel,

    /// Format
    FormatButton,

    /// Generates a dummy `FNIS.esp`.
    ///
    /// # Notes
    /// - Some FNIS mods tell whether their hkx were generated by whether `FNIS.esp` exists
    ///
    /// # Examples
    /// - XPMSE
    /// - FNIS Sexy Move
    /// - EatingSleepingDrinking
    GenerateFnisEspHover,

    /// Gen FNIS.esp
    GenerateFnisEspLabel,

    /// Go Back
    GoBack,

    /// Go Forward
    GoForward,

    /// Bring the open object to the center
    GraphCenterHover,

    /// How many layers right of the open object are shown
    GraphDepthHover,

    /// Collapse all
    GraphDetailsCollapseHover,

    /// Expand all
    GraphDetailsExpandHover,

    /// Hide the details
    GraphDetailsHideHover,

    /// Details
    GraphDetailsLabel,

    /// Show the details
    GraphDetailsShowHover,

    /// Show all the nodes
    GraphFitHover,

    /// Show the neighborhood of this object as a node graph, in place of the text, with the fields
    /// on each node.
    ///
    /// # Notes
    /// - Click a value to type it (`Enter`: done, `Esc`: cancel)
    /// - Drag a pin onto a node to point to it, off any node to clear it; `+` adds to a list
    /// - Click a title to go to that object, double click to edit its text; drag it to move it
    /// - Click a field's name to edit its line
    /// - A long list shows a page of it: `‹ ›` turns the page
    /// - Point at a node to see its own edges; `×N`: N objects point to it
    /// - A dashed edge is another way to an object shown already (a shared one)
    /// - Drag the background or wheel to move, `Ctrl` + wheel to zoom, double click to center
    GraphHover,

    /// Show or hide the node graph (Vim: Space g)
    GraphToggle,

    /// Zoom in
    GraphZoomInHover,

    /// Zoom out
    GraphZoomOutHover,

    /// Back to 100%
    GraphZoomResetHover,

    /// Annotation
    HkannoAnnotationLabel,

    /// Drop .hkx / .xml files here
    HkannoDropHint,

    /// Open or drop .hkx / .xml files to edit their annotations.
    HkannoEmptyMessage,

    /// Export .txt…
    HkannoExportTxt,

    /// Save the annotation text as a .txt file.
    HkannoExportTxtHover,

    /// Import .txt…
    HkannoImportTxt,

    /// Replaces the annotations of this tab with a `.txt` file (hkanno text).
    ///
    /// # Notes
    /// - A `.txt` dropped on the page does the same
    HkannoImportTxtHover,

    /// Open .hkx / .xml files. (Drag & drop also works.)
    HkannoOpenHover,

    /// Discard the edits and restore the annotations of the original file?
    HkannoRevertConfirm,

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

    /// Reloads the translation file, to see a translation without restarting the application.
    I18nReloadJsonHover,

    /// Language & Translation
    I18nTitleLabel,

    /// New I18n
    I18nWriteNewJsonButton,

    /// Writes a new translation file for this version of the application, with the English texts.
    I18nWriteNewJsonHover,

    /// Imported
    ImportedMessage,

    /// Show the changes inside the editor: new lines in green, the old ones in red between them.
    InlineDiffHover,

    /// Show or hide the inline diff (Vim: Space d, :diff)
    InlineDiffToggle,

    /// Report Issue
    IssueReportButton,

    /// Reports a bug on GitHub, with the version and some hardware info filled in.
    ///
    /// # Notes
    /// - A GitHub account is needed
    IssueReportHover,

    /// Previous / next change (Vim: [c, ]c)
    JumpDiffHover,

    /// Keep Open
    KeepOpen,

    /// Nemesis special syntax: Empty Array push pattern. Adds elements to an array that is empty in the template: its line stays and the elements follow it, so the array is not replaced and several mods can add to it.
    ///
    /// # Examples
    /// ```xml
    /// <hkparam name="eventNames" numelements="0"></hkparam>
    /// <!-- MOD_CODE ~mymod~ OPEN -->
    ///     <hkcstring>Hi</hkcstring>
    /// </hkparam>
    /// <!-- CLOSE -->
    /// ```
    ///
    /// # Notes
    /// - Not compatible with Pandora v4
    KeptArrayHover,

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

    /// Key binding saved to keybindings.json
    KeybindingSaved,

    /// Save the changes to keybindings.json? They are lost if they are not saved.
    KeybindingUnsaved,

    /// License:
    LicenseLabel,

    /// Failed to load
    LoadFailedMessage,

    /// Locked
    LockButton,

    /// Reordering the rows is locked unless they are sorted by priority (ascending) and not searched: it is unsafe then.
    ///
    /// # Notes
    /// - Click to unlock
    LockButtonHover,

    /// Log
    LogButton,

    /// Log Dir
    LogDir,

    /// Log Path:
    LogDirPathLabel,

    /// Log Level
    LogLevelLabel,

    /// Changes the log directory to this path.
    ///
    /// # Notes
    /// - Logs are rotated, up to 5 files
    /// - The change takes effect when this button is clicked, or the application restarts
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

    /// For use without a virtual file system such as MO2's.
    ///
    /// # Notes
    /// - Several versions of one mod may conflict
    /// - Absolute paths are the ids of the mods (no duplicates): the settings are not for sharing
    ManualModeHover,

    /// Accept
    MergeAccept,

    /// Accept all
    MergeAcceptAll,

    /// Puts the lines of every mod in place of the conflict, each line once (git's "accept both").
    ///
    /// # Notes
    /// - The resolution is kept, and done again the next time this conflict comes
    MergeAcceptAllHover,

    /// Puts this mod's lines in place of the conflict.
    ///
    /// # Notes
    /// - The resolution is kept, and done again the next time this conflict comes
    MergeAcceptHover,

    /// What each mod has at this conflict: put together in the editor what is wanted.
    ///
    /// # Notes
    /// - With an array, the elements of several mods are often wanted
    /// - Closed with its ✖, the patch preview comes back
    MergeHover,

    /// Puts it below the line of the cursor in the editor.
    ///
    /// # Notes
    /// - What it replaces stays: delete it there if it is not wanted
    MergeInsertHover,

    /// Original
    MergeOriginal,

    /// What this value points to, as this mod has it.
    ///
    /// # Notes
    /// - Two mods pointing to their own objects differ in what those are: compare them before picking one
    MergePointedHover,

    /// Neither this mod nor the template has this object.
    MergePointedMissing,

    /// It removes these lines.
    MergeRemoves,

    /// Resolved as before: this conflict was resolved the same way the last time.
    MergeReused,

    /// Values of the mods
    MergeTitle,

    /// Who made the mod (author in info.ini).
    ModAuthorHover,

    /// This mod has several mod codes: which one to open?
    ModCodeChoose,

    /// e.g. mymod
    ModCodeHint,

    /// What identifies the mod: 4-6 ASCII letters or digits.
    ///
    /// # Notes
    /// - Not `vanilla`, and not used by another mod
    /// - The name of its directory, and the prefix of its new objects (`#mymod$0`)
    /// - Once the work is made, "Rename…" of the project details changes it, with those ids
    ModCodeHover,

    /// A mod code is 4-6 ASCII letters/digits and not "vanilla".
    ModCodeInvalid,

    /// Mod code:
    ModCodeLabel,

    /// Rename…
    ModCodeRename,

    /// Renamed the mod code
    ModCodeRenameDone,

    /// Gives this work another mod code (4-6 letters or digits).
    ///
    /// # Notes
    /// - The ids of its new objects and clips change with it
    /// - What was written under the old code goes at the next Export
    ModCodeRenameHover,

    /// Renaming the mod code
    ModCodeRenameRunning,

    /// The mod's name, as mod lists show it (name in info.ini).
    ModNameHover,

    /// Name:
    ModNameLabel,

    /// Not a Nemesis mod (no Nemesis_Engine/mod/<code>)
    ModNotNemesis,

    /// Open a mod…
    ModOpen,

    /// Opens a Nemesis mod to edit it.
    ///
    /// # Targets
    /// - Its root (where `Nemesis_Engine` is): `D:\MO2\mods\Paraglider`
    /// - Its patch directory: `...\Nemesis_Engine\mod\para`
    ///
    /// # Notes
    /// - A mod opened before goes on with its edits that are not written yet
    ModOpenHover,

    /// Reload from the mod
    ModReload,

    /// Reads the mod again. Discards the edits that are not exported.
    ModReloadHover,

    /// The URL of the mod's page (site in info.ini).
    ModSiteHover,

    /// Site:
    ModSiteLabel,

    /// Mod Test Status:
    ModTestStatusLabel,

    /// The mod's version now.
    ///
    /// # Examples
    /// - `1.2.0`
    ///
    /// # Notes
    /// - Written to the `info.ini` of `Nemesis_EngineExt` only, with the mod codes of the references as `deps`
    ModVersionHover,

    /// Version:
    ModVersionLabel,

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

    /// The mods are collected from these paths in the Skyrim Data directory.
    ///
    /// - Nemesis: `Nemesis_Engine/mod/*/info.ini`
    /// - NemesisExt: `Nemesis_EngineExt/mod/*/info.ini`
    /// - FNIS: `meshes/**/animations/*/FNIS_*_List.txt`
    ModsListTitleHover,

    /// No change
    NoChange,

    /// None
    NoneText,

    /// Normalize
    NormalizeButton,

    /// Sorts the mod list by these rules.
    ///
    /// - Mods are grouped by type: Nemesis -> NemesisExt -> FNIS
    /// - In each type, mods are sorted alphabetically
    /// - The priorities are given again from `0`
    /// - In manual mode, the check of duplicate ids is turned off
    ///
    /// # Notes
    /// - This does not resolve conflicts between patches
    NormalizeHover,

    /// Clear Notify
    NotificationClearButton,

    /// Getting the Skyrim Data directory is not supported on this platform (Linux, macOS): specify it by hand.
    NotifyErrPlatformNotSupported,

    /// Could not find the path in the Windows registry. If Skyrim is not the Steam version, specify the Skyrim Data directory by hand.
    NotifyErrWindowsRegistryNotFound,

    /// The patch file of the object in the mod, where Export writes it, if this work patches it; else the template it is read from.
    ObjectFileHover,

    /// Filter (id, class, name)
    ObjectFilterHint,

    /// New
    ObjectNew,

    /// Select an object on the left.
    ObjectSelectHint,

    /// Open
    OpenButton,

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

    /// Choose the output directory.
    OutputMissing,

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

    /// The directory with Pandora's `Settings.json` and `Pandora_Engine\ActiveMods.json`.
    ///
    /// # Notes
    /// - Import reads them from here and Export writes them here: only these two buttons use it
    PandoraDirHover,

    /// Open the Pandora patch of this template as text
    PandoraDocHover,

    /// Exports d-merge's VFS settings to Pandora.
    ///
    /// # Targets
    /// - `Settings.json`
    ///   - The Skyrim SE Data directory
    ///   - The output directory
    /// - `ActiveMods.json`
    ///   - Whether each Nemesis mod is on
    ///   - The priority of each Nemesis mod
    PandoraExportHover,

    /// Export
    PandoraExportLabel,

    /// Imports Pandora's settings into d-merge's VFS settings.
    ///
    /// # Targets
    /// - `Settings.json`
    ///   - The Skyrim SE Data directory
    ///   - The output directory
    /// - `ActiveMods.json`
    ///   - Whether each Nemesis mod is on
    ///   - The priority of each Nemesis mod
    PandoraImportHover,

    /// Import
    PandoraImportLabel,

    /// The version of Pandora to import from or export to.
    ///
    /// - `v4.4.0-beta`: compatible with `v5.0.0`
    PandoraTargetVersionHover,

    /// Target version:
    PandoraTargetVersionLabel,

    /// How the patches are checked, and what is done with the wrong ones.
    ///
    /// - Strict:
    ///   rejects inconsistent or invalid input, as before v2.6.0
    /// - Lenient:
    ///   like Nemesis, recovers from known mistakes when the intended value can be told from the input
    ///
    /// # Notes
    /// - A patch that needs Lenient has invalid or nearly undefined input: even recovered, it may not do what its author meant
    ///   -> A wrong length (`len`) above all: the value worked out may not be right
    /// - Lenient is not a convenience: it is for when the patch can't be fixed by hand
    /// - Input whose value can't be told still fails in Lenient
    ///
    /// # Examples
    /// Mods with `trigger_names_len` mistakes that need Lenient:
    /// - Unarmed Behaviors Expanded (v2.0.1)
    /// - BFCO (v3.100.7)
    ParserModeHover,

    /// Parser:
    ParserModeLabel,

    /// Lenient
    ParserModeLenient,

    /// Strict
    ParserModeStrict,

    /// Some patches are rejected by d-merge's Nemesis parser
    ParserRejected,

    /// Patch
    PatchButton,

    /// Updating list...
    PatchFetchingButton,

    /// These operations of Pandora give the object as it is edited here.
    PatchPreviewPandoraOpsOk,

    /// d-merge's Nemesis parser accepts this patch.
    PatchPreviewParserOk,

    /// Put this changed block back to the template's lines.
    PatchPreviewRevertHunkHover,

    /// Patch preview
    PatchPreviewTitle,

    /// Preview error
    PreviewError,

    /// Follow the cursor: the preview scrolls to the editor's cursor line and highlights it.
    PreviewFollowCursor,

    /// Hide Preview
    PreviewHide,

    /// Show Preview
    PreviewShow,

    /// Show or hide the preview (Vim: Space p, :preview)
    PreviewToggle,

    /// Problems
    ProblemsLabel,

    /// No problems have been found in the work.
    ProblemsNone,

    /// The mods this one depends on: an add-on's base, or the mods a user patch takes values from. `+` adds one: its root, or its patch directory.
    ///
    /// # Notes
    /// - Their new objects, events and variables can be pointed to and looked at
    /// - They are not edited or written: they come with those mods when merging
    RefModsHover,

    /// References:
    RefModsLabel,

    /// Read only: an object of a referenced mod.
    RefReadOnly,

    /// Reload
    ReloadButton,

    /// Removing the `<output_dir>/meshes` directory...
    RemovingMeshesMessage,

    /// Makes new objects from this one, one per line of values.
    ///
    /// # Notes
    /// - `{{name}}` in the template takes the value of the column `name`
    /// - `{{i}}` takes the line number, from 0
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

    /// The format of the hkx written.
    ///
    /// - SkyrimLE / Enderal: win32
    /// - SkyrimSE / SkyrimVR / EnderalSE: amd64
    ///
    /// # Notes
    /// - In VFS mode on Windows, a change also looks for the Skyrim Data directory in the registry
    RuntimeTargetHover,

    /// Save
    SaveButton,

    /// Save failed
    SaveFailed,

    /// Saved successfully.
    SavedMessage,

    /// Which templates this work can patch. It can't be changed later.
    ///
    /// - Nemesis:
    ///   only those Nemesis supports
    /// - Nemesis Ext / Pandora v5:
    ///   all of them, and template files of your own
    ///
    /// # Notes
    /// - Either can be written in any format
    ScopeHover,

    /// Templates
    ScopeLabel,

    /// Only the templates Nemesis knows (this work is a Nemesis one).
    ScopeNemesisOnly,

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

    /// Text of every object, e.g. $eventID[moveStart]$ (Ctrl+Shift+F)
    SearchTextHint,

    /// Select All
    SelectAll,

    /// Select
    SelectButton,

    /// Select None
    SelectNone,

    /// selected
    SelectedLabel,

    /// About
    SettingsAboutLabel,

    /// Settings
    SettingsButton,

    /// Display
    SettingsDisplayLabel,

    /// Skyrim Data dir:
    SkyrimDataDirLabel,

    /// Source Code:
    SourceCodeLabel,

    /// Make a new patch mod, or open an existing one to edit it.
    StartHint,

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

    /// files:
    StatusReadFilesLabel,

    /// Reading templates and patches...
    StatusReadingPatches,

    /// Close All
    TabsCloseAll,

    /// Close Others
    TabsCloseOthers,

    /// Close to the Right
    TabsCloseRight,

    /// Nemesis does not have this template: Export writes it to `Nemesis_EngineExt`, which d-merge reads.
    TemplateExtHover,

    /// Template:
    TemplateLabel,

    /// Nemesis has this template: Export writes it to `Nemesis_Engine`.
    TemplateNemesisHover,

    /// No template found for
    TemplateNotFound,

    /// No templates (.bin / .xml under meshes/) in the template directory: check template_dir in the settings.
    TemplatesNone,

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

    /// Merges what the conflict check decided, kept in `<output directory>/.d_merge/user_patch`, after every mod, and leaves out the operations hidden there.
    ///
    /// # Notes
    /// - It is not in the mod list: it always loads last, like MO2's overwrite
    /// - Each output directory has its own
    UserPatchApplyHover,

    /// Delete all
    UserPatchClear,

    /// Delete all of the User Patch?
    UserPatchClearConfirm,

    /// Takes everything out of the user patch and writes that: each conflict goes back to the mod that wins by the load order.
    ///
    /// # Notes
    /// - Hidden operations stay
    /// - It cannot be undone
    UserPatchClearHover,

    /// All that the user patch has: the values chosen in conflicts and what was edited by hand.
    UserPatchHover,

    /// Decided by the output directory of the Patch page: each profile has its own conflicts.
    ///
    /// # Notes
    /// - It cannot be changed here
    /// - Edits are written a moment later
    UserPatchOutputHover,

    /// The User Patch is edited in its own window: the User Patch button.
    UserPatchReserved,

    /// User Patch
    UserPatchTitle,

    /// Events
    VarsEvents,

    /// Event name
    VarsNewEventHint,

    /// The new event's name. Patches refer to it as `$eventID[myEvent]$`.
    ///
    /// # Examples
    /// - `myEvent`
    ///
    /// # Notes
    /// - No spaces, and not one that exists, whatever its case
    VarsNewEventHover,

    /// Variable name
    VarsNewVariableHint,

    /// The new variable's name. Patches refer to it as `$variableID[myVar]$`.
    ///
    /// # Examples
    /// - `myVar`
    ///
    /// # Notes
    /// - No spaces, and not one that exists, whatever its case
    VarsNewVariableHover,

    /// The type of its value: BOOL, the whole numbers INT8 / INT16 / INT32, or REAL for decimals.
    VarsNewVariableTypeHover,

    /// Its initial value, by type.
    ///
    /// - BOOL: `true` or `false`
    /// - INT: a whole number, e.g. `1`
    /// - REAL: a decimal, e.g. `0.2`
    VarsNewVariableValueHover,

    /// This template has no event / variable data (hkbBehaviorGraphStringData).
    VarsNone,

    /// Animation data has no events or variables.
    VarsNoneInAnimData,

    /// Rename (the patches using it are updated too)
    VarsRename,

    /// Events / Variables
    VarsTab,

    /// Variables
    VarsVariables,

    /// VFS
    VfsMode,

    /// When booting using MO2's VFS, etc.
    VfsModeHover,

    /// Where the work in progress is saved by itself, for each mod code.
    ///
    /// # Notes
    /// - In the output directory of the Patch page: decided by the app
    WorkCacheDirHover,

    /// Close
    WorkClose,

    /// Back to the start screen. The work is kept, and comes back from Open Recent.
    WorkCloseHover,

    /// There is already a work with this mod code: open it, or choose another code.
    WorkExists,

    /// Set the output directory on the Patch page first: the works and the user patch are kept in it. The Skyrim Data directory can be used too, but something may be overwritten unintentionally.
    WorkNeedPatchOutput,

    /// New
    WorkNew,

    /// Make a new patch mod.
    WorkNewHover,

    /// Open work data
    WorkOpenCacheDir,

    /// Open Recent
    WorkOpenRecent,

    /// Nothing opened yet.
    WorkOpenRecentEmpty,

    /// Reading
    WorkReading,

    /// Neither its work nor its mod is there any more, so it left the list
    WorkRecentMissing,

    /// A work on another mod had this mod code: it was replaced by this mod
    WorkReplaced,

    /// Failed to save the work in progress
    WorkSaveFailed,

    /// The mod this work was read from. Click to open it.
    WorkSourceHover,

    /// Read from
    WorkSourceLabel,

    /// XML preview
    XmlPreviewTitle,

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
        i18n.t(I18nKey::KeepOpen),
        i18n.t(I18nKey::CloseButton),
        i18n.t(I18nKey::TabsCloseOthers),
        i18n.t(I18nKey::TabsCloseRight),
        i18n.t(I18nKey::TabsCloseAll),
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

    /// The `snake_case` name of `key`, as `translation.json` has it. e.g. `BehaviorsAdd` -> `behaviors_add`
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
