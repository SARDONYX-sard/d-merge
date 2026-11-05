//! Patch tool: edit template objects, get Nemesis patches (`#XXXX.txt`) out.
//!
//! ```text
//! Output: [D:/mods/mymod ] [Select]   Mod code: [mymod]  Name: [..]  Author: [..]  Site: [..]
//! [Add Behavior ▼] [Custom Template…] [Import Mod…]                                  [Export]
//!   [filter      ]  meshes/actors/draugr/behaviors/draugrbehavior.bin   Nemesis   <- picker
//! [draugrbehavior] [0_master] ...
//! ┌ filter ──────────┐┌ #0106 hkbModifierList ────────────────────────────────────────────┐
//! │ ☐ Changed only   ││ <hkobject name="#0106" ...>          (xml editor, one object)     │
//! │ ● #0106 GetUp..  ││ ...                                                               │
//! │ + #mymod$0       │├ Patch preview ─────────────────────────────────────────────────────┤
//! │   #0107 ...      ││ <!-- MOD_CODE ~mymod~ OPEN --> ...                ✔ parser OK      │
//! └ [New][Dup][Rev] ┘└───────────────────────────────────────────────────────────────────┘
//! ```
//!
//! Work is cached under the patch page's output dir and saved a moment after each edit (see [`workspace`]).

mod anim_data;
mod anim_lines;
mod anim_patch;
mod checklist;
mod diagnostics;
mod editor_tabs;
mod inline_diff;
mod language;
mod names;
mod navigate;
mod new_work;
mod preview;
mod resolve;
mod search;
mod template_tree;
mod tree;
mod ui;
mod user_patch;
mod vars_edit;
mod workspace;

use std::{
    path::{Path, PathBuf},
    rc::Rc,
    time::Instant,
};

use egui_code_editor::{CodeEditorState, Diagnostic, EditorCommand, InlineDiff};
use egui_widgets::notify::Notification;
use gui_shared::{
    i18n::{I18nKey, I18nMap},
    settings::{
        pages::{
            patch::behavior::ParserMode,
            patch_editor::{PatchEditorSettings, RecentWork},
        },
        ui::editor::{EditorMode, LspOptions},
    },
};
use nemesis_diff::is_valid_mod_code;

use self::workspace::{Behavior, Session, SessionTab, TemplateEntry, Workspace};

/// How many works the list of recent ones keeps.
const MAX_RECENT: usize = 12;

/// `work` goes first in the list, once. A code is one work, wherever it is exported to.
fn remember_recent(recent: &mut Vec<RecentWork>, work: RecentWork) {
    recent.retain(|other| other.mod_code != work.mod_code);
    recent.insert(0, work);
    recent.truncate(MAX_RECENT);
}

/// How long typing must pause before the patch, its check, the underlines and the diff are computed
/// again. Together they take over 100 ms on a big object in a debug build: too slow for each key.
const REFRESH_DELAY: std::time::Duration = std::time::Duration::from_millis(150);

/// How long after an edit the workspace is written to its cache.
const SAVE_DELAY: std::time::Duration = std::time::Duration::from_secs(1);

/// What the tool needs of the patch page's settings.
#[derive(Debug, Clone, Copy)]
pub struct PatchPaths<'a> {
    /// The patch page's output directory: the works and the user patch are kept in it. It belongs to
    /// the profile, as its conflicts do. Empty: not set yet, and nothing can be kept.
    pub patch_output_dir: &'a str,
    /// `settings.behavior.template_dir` (absolute or relative).
    pub template_dir: &'a str,
    /// `settings.behavior.parser_mode`: patches are checked as d-merge will read them.
    pub parser_mode: ParserMode,
}

#[derive(Debug, Default)]
pub(crate) struct PatchTool {
    /// The patch page's output directory, see [`PatchPaths`].
    patch_output: String,
    /// Where the works are: see [`workspace::cache_root_of`].
    cache_root: PathBuf,
    /// This tool is the user patch's, in its window: see [`user_patch`].
    is_user_patch: bool,
    /// What the mods have at the conflict being edited, shown beside the editor.
    merge_values: Option<resolve::MergeValues>,
    /// What `Edit...` proposed, until it is edited.
    draft: Option<user_patch::Draft>,
    /// Its bulk delete asks once more.
    confirm_clear: bool,
    /// Where this work is exported to.
    output: String,
    /// The root of the mod the work was read from. Empty: it was made here.
    mod_dir: String,
    /// See [`Self::take_mod_written`].
    mod_written: bool,
    /// See [`new_work`]. `Some`: its form is shown on the start screen.
    new_work: Option<new_work::NewWork>,
    mod_code: String,
    /// `settings.behavior.template_dir` (`.bin` or `.xml` templates under `meshes/`).
    template_dir: String,
    /// Templates found in `template_dir`, for the picker.
    catalog: Vec<TemplateEntry>,
    catalog_for: Option<String>,
    picker_open: bool,
    picker_filter: String,
    /// The next quick pick window drawn takes the keyboard.
    focus_quick_pick: bool,
    /// The details of the bar above are shown. See [`ui::top_bar`].
    project_open: bool,
    /// The side bar's behavior list is shown (else only its header).
    behaviors_open: bool,
    /// `output` / `code` were taken from the settings.
    restored: bool,
    workspace: Workspace,
    /// The code of the work the workspace belongs to (`None` = no work is open).
    loaded_for: Option<String>,
    /// Unsaved edits since this time.
    dirty_since: Option<Instant>,

    active: usize,
    selected: Option<String>,
    filter: String,
    changed_only: bool,

    /// The text in the editor and what it belongs to (`(behavior, id)`).
    editor_text: String,
    editor_for: Option<(usize, String)>,
    editor: CodeEditorState,
    /// The patch page's parser mode: what d-merge will accept, so what is checked here.
    parser_mode: ParserMode,
    /// The trigger names of the template, read once (see [`anim_data::template_trigger_names`]).
    template_triggers: Option<std::collections::BTreeSet<String>>,
    /// Those and the work's, for completion in clip data files. Updated when one is opened.
    trigger_names: Vec<String>,
    /// The "animation data" window.
    anim_data_dialog: Option<ui::anim_data::AnimDataDialog>,
    /// The objects open in tabs.
    tabs: editor_tabs::EditorTabs,
    /// Preview of `editor_for`: (patch, parser result). Recomputed when the text changes.
    preview: Option<(Option<String>, Result<(), String>)>,
    /// Typed at this time: `preview` is out of date, and is computed again after [`REFRESH_DELAY`].
    typed_at: Option<Instant>,
    /// Unknown class / field names in the editor text (underlined).
    field_problems: Vec<Diagnostic>,
    /// The changes shown inside the editor, compared to the template.
    inline_diff: InlineDiff,
    /// Show `inline_diff`. From the settings.
    show_inline_diff: bool,
    /// Show preview diff. From the settings.
    show_preview: bool,
    /// The preview scrolls to and highlights the editor's cursor line. From the settings.
    follow_preview: bool,
    /// One label per changed block of the selected object (its `↶` revert buttons).
    preview_view: Option<preview::Preview>,
    /// `Shift+F12`: who uses a symbol.
    references: Option<navigate::ReferenceList>,
    history: navigate::History,
    /// The keyboard goes to the editor on the next frame, and the cursor to this place.
    /// (An editor's id depends on its object, so this can only be done once the object is open.)
    focus_editor: Option<FocusAt>,
    /// The keyboard is in the object tree (Vim `Space e`).
    tree_focus: bool,
    /// The flat object list after its filter, cached: it is drawn every frame.
    list: Option<ObjectList>,

    /// Normal / Vim keys of the object editor.
    editor_mode: EditorMode,
    /// Language features (shared with hkanno's options), and their window is open.
    lsp: LspOptions,
    show_lsp_options: bool,
    /// The editor's cursor line last frame (for the class around it).
    cursor_line: usize,

    /// `(behavior, values)`: enum / flag values per field, for the completion.
    field_values: Option<(usize, std::rc::Rc<language::FieldValues>)>,
    /// Object list as a tree from the root container.
    tree_view: bool,
    /// The "New" class list is open, and its filter.
    class_picker: bool,
    class_filter: String,
    /// The tree of each behavior, by template path.
    trees: fast_hash::HashMap<String, tree::TreeState>,
    /// `Space` was pressed with the keyboard in the flat list. See [`tree::TreeState::leader`].
    list_leader: bool,

    /// Left panel shows events / variables instead of objects.
    show_vars: bool,
    /// Left panel shows the text search instead of those. See [`search`].
    show_search: bool,
    search: search::Search,
    /// The search field takes the keyboard next frame.
    focus_search: bool,
    /// The actors opened in the anim data list (`DefaultMale~1`). See [`ui::anim_tree`].
    anim_folders_open: fast_hash::HashSet<String>,
    /// The button above the list was clicked: done where the list has its files.
    anim_expand_all: bool,
    /// The selection the way to it was last opened for.
    anim_revealed: Option<String>,
    /// A conflict's value to change in the editor, taken in on the next frame.
    pending_resolution: Option<gui_shared::patch::ResolveRequest>,
    /// The works opened lately, the last one first. From the settings.
    recent: Vec<RecentWork>,
    /// The behavior and the object the work last saved had open.
    session_seen: (usize, Option<String>),
    /// The window of the user patch, and what it asks for.
    user_patch: user_patch::UserPatchView,
    /// The projects of the clip data template, read once. See [`anim_data::template_projects`].
    template_projects: Option<Vec<String>>,
    /// The window of what was checked on request.
    work_check: Option<ui::work_check::WorkCheck>,
    /// The "duplicate to" window.
    duplicate_form: Option<ui::anim_data::DuplicateForm>,
    /// What a row's menu asked to duplicate: `(files, to the project)`.
    pending_duplicate: Option<(Vec<String>, String)>,
    /// The "new clips" window.
    new_clips: Option<ui::anim_data::NewClipsForm>,
    /// What each line of the open anim data file is. Recomputed with the preview.
    anim_roles: Vec<anim_lines::LineRole>,
    /// The events / variables panel shows variables (else events).
    show_variables: bool,
    /// `(behavior, names)` of the events / variables panel and the completion.
    names: Option<(usize, Rc<names::Names>)>,
    /// Objects edited since `names` last looked. See [`Self::object_changed`].
    changed_objects: Vec<String>,
    /// Directories opened / closed in the template picker's tree.
    picker_toggled: fast_hash::HashSet<String>,
    new_names: vars_edit::NewNames,
    vars_error: Option<String>,
}

/// Where the cursor goes when the editor gets the keyboard back.
#[derive(Debug, Clone, Copy)]
enum FocusAt {
    /// Where it was in that object (e.g., going back with `Ctrl+O`).
    Keep,
    /// A 0-based line (e.g., go to definition).
    Line(usize),
}

/// What the editor asked for in a frame. See [`PatchTool::apply_editor_result`].
#[derive(Debug, Default)]
struct EditorResult {
    /// The text was edited.
    changed: bool,
    command: Option<EditorCommand>,
    /// Go to `(document, line)`.
    definition: Option<(String, usize)>,
    /// The palette's "focus the object tree" was run.
    focus_tree: bool,
    /// `Some(true)`: the palette's "back". `Some(false)`: "forward".
    walk_history: Option<bool>,
    /// A problem in the Problems panel was clicked: go to this line.
    jump_to_line: Option<usize>,
    /// The `↶` of this place of the inline diff was clicked.
    inline_revert: Option<usize>,
    tab_event: Option<TabEvent>,
}

/// What a click on the tab bar asked for.
#[derive(Debug)]
enum TabEvent {
    Show(String),
    KeepOpen(String),
    Close(String),
    CloseOthers(String),
    CloseToTheRight(String),
    CloseAll,
}

/// See [`PatchTool::list`].
#[derive(Debug)]
struct ObjectList {
    /// The state of the graph it was computed from.
    generation: u64,
    filter: String,
    changed_only: bool,
    rows: Vec<tree::Node>,
}

impl PatchTool {
    /// Goes to the work of `code`: [`Self::sync_workspace`] loads it, and what it saved of the output
    /// and the source wins over these. The current work is saved first, while these are still its own.
    fn switch_work(&mut self, code: &str, output: &str, source: &str, notify: &mut Notification, i18n: &I18nMap) {
        self.save_now(notify, i18n);
        code.clone_into(&mut self.mod_code);
        output.clone_into(&mut self.output);
        source.clone_into(&mut self.mod_dir);
    }

    /// After the work is synced: takes the value in, and shows where it went. It is a proposal for now:
    /// see [`user_patch::Draft`]. So nothing of it is saved until there is an edit.
    fn take_in_resolution(&mut self, notify: &mut Notification, i18n: &I18nMap) {
        let Some(request) = self.pending_resolution.take() else { return };
        self.undo_unedited_draft();
        let before = user_patch::Draft::before(&self.workspace);
        let dirty_since = self.dirty_since;
        if self.workspace.info.name.is_empty() {
            i18n.t(I18nKey::NemesisUserPatchName).clone_into(&mut self.workspace.info.name);
        }
        // What the value points to is that mod's.
        self.workspace.add_ref(Path::new(&request.mod_dir), gui_shared::patch::USER_PATCH_CODE);
        // And what is taken from the others is theirs.
        for side in &request.sides {
            self.workspace.add_ref(Path::new(&side.mod_dir), gui_shared::patch::USER_PATCH_CODE);
        }
        let root = PathBuf::from(&self.template_dir);
        let shown = resolve::apply(&mut self.workspace, &root, &request).map_or_else(
            |err| {
                notify.error(format!("{}: {err}", i18n.t(I18nKey::NemesisResolveFailed)));
                // It is for the user to edit: all the more if it could not be taken in.
                resolve::object_of(&mut self.workspace, &root, &request)
            },
            Some,
        );
        let Some(resolve::Resolved { behavior, id, line }) = shown else { return };
        self.workspace.reload_refs(); // a behavior may have been opened
        self.reset_names();
        self.active = behavior;
        self.editor_for = None;
        self.draft = Some(before.with_proposal(&self.workspace, behavior, &id));
        self.merge_values = resolve::merge_values(&request, &id);
        self.selected = Some(id);
        // What is shown is no change of the work yet.
        self.dirty_since = dirty_since;
        self.session_seen = (self.active, self.selected.clone());
        // Picking another value of the object shown has no other sign of having gone anywhere.
        self.focus_editor = line.map(FocusAt::Line);
    }

    /// A mod was written since the last call: what the conflict check found may be old.
    pub(crate) const fn take_mod_written(&mut self) -> bool {
        std::mem::replace(&mut self.mod_written, false)
    }

    /// Follows the patch page's setting. A change takes effect on the shown object at once.
    pub(crate) fn set_parser_mode(&mut self, mode: ParserMode) {
        if self.parser_mode != mode {
            self.parser_mode = mode;
            self.preview = None;
        }
    }

    /// The patch page's output directory may have changed: the works are others then. What is open is
    /// saved where it belongs first.
    fn set_patch_output(&mut self, output: &str, notify: &mut Notification, i18n: &I18nMap) {
        let output = output.trim();
        if self.patch_output == output {
            return;
        }
        if self.restored {
            self.close_work(notify, i18n);
        }
        output.clone_into(&mut self.patch_output);
        self.cache_root = workspace::cache_root_of(output);
    }

    /// The editor's settings, shared with the hkanno editor.
    fn read_editor_options(&mut self, editor: &LspOptions) {
        self.editor_mode = editor.editor_mode;
        self.lsp.clone_from(editor);
    }

    /// Back to the settings: changed by the keymap box and the options window.
    fn write_editor_options(&mut self, editor: &mut LspOptions) {
        // Both hold the keymap: the box only changes `editor_mode`.
        self.lsp.editor_mode = self.editor_mode;
        if *editor != self.lsp {
            editor.clone_from(&self.lsp);
        }
    }

    /// What is shown of the editor, kept in the settings.
    const fn read_view_settings(&mut self, settings: &PatchEditorSettings) {
        self.show_inline_diff = settings.inline_diff;
        self.follow_preview = settings.follow_preview;
        self.show_preview = settings.show_preview;
        self.tree_view = settings.tree_view;
    }

    const fn write_view_settings(&self, settings: &mut PatchEditorSettings) {
        settings.inline_diff = self.show_inline_diff;
        settings.show_preview = self.show_preview;
        settings.follow_preview = self.follow_preview;
        settings.tree_view = self.tree_view;
    }

    pub(crate) fn ui(
        &mut self,
        ui: &mut egui::Ui,
        i18n: &I18nMap,
        notify: &mut Notification,
        paths: PatchPaths<'_>,
        settings: &mut PatchEditorSettings,
        editor: &mut LspOptions,
    ) {
        self.read_editor_options(editor);
        if self.template_dir != paths.template_dir {
            paths.template_dir.clone_into(&mut self.template_dir);
        }
        self.set_patch_output(paths.patch_output_dir, notify, i18n);
        // The first frame reopens the last work (its cache is found from these two).
        if !self.restored {
            self.restored = true;
            self.output.clone_from(&settings.output);
            self.mod_dir.clone_from(&settings.mod_dir);
            self.mod_code.clone_from(&settings.mod_code);
            self.recent.clone_from(&settings.recent);
            // It has a window of its own.
            self.recent.retain(|work| !work.mod_code.eq_ignore_ascii_case(gui_shared::patch::USER_PATCH_CODE));
            // A work that is gone: the start screen rather than an empty work.
            let code_is_free = self.mod_code.trim().eq_ignore_ascii_case(gui_shared::patch::USER_PATCH_CODE);
            if self.patch_output.is_empty()
                || code_is_free
                || !Workspace::exists(&self.cache_root, self.mod_code.trim())
            {
                self.close_work(notify, i18n);
            }
            // What is set once stays out of the way, unless it still has to be set.
            self.project_open = false;
            self.behaviors_open = true;
            self.trees =
                settings.tree_open.iter().map(|(path, open)| (path.clone(), tree::TreeState::restored(open))).collect();
        }
        self.read_view_settings(settings);
        self.changed_only = settings.changed_only;
        self.sync_workspace(notify, i18n);
        self.draw(ui, i18n, notify);

        self.write_editor_options(editor);
        if settings.mod_dir != self.mod_dir {
            settings.mod_dir.clone_from(&self.mod_dir);
        }
        if settings.recent != self.recent {
            settings.recent.clone_from(&self.recent);
        }
        self.write_view_settings(settings);
        settings.changed_only = self.changed_only;
        for (path, tree) in &mut self.trees {
            if let Some(open) = tree.take_open_changes() {
                settings.tree_open.insert(path.clone(), open);
            }
        }
        if settings.output != self.output || settings.mod_code != self.mod_code {
            settings.output.clone_from(&self.output);
            settings.mod_code.clone_from(&self.mod_code);
        }
    }

    fn draw(&mut self, ui: &mut egui::Ui, i18n: &I18nMap, notify: &mut Notification) {
        if self.is_user_patch {
            self.user_patch_bar(ui, i18n, notify);
        } else if self.loaded_for.is_none() {
            self.start_screen(ui, i18n, notify);
            return;
        } else {
            self.project_bar(ui, i18n, notify);
        }
        ui.separator();
        self.template_picker_window(ui.ctx(), i18n, notify);
        self.anim_data_window(ui.ctx(), i18n, notify);
        self.new_clips_window(ui.ctx(), i18n);
        self.duplicate_window(ui.ctx(), i18n);
        self.work_check_window(ui.ctx(), i18n);
        self.active = self.active.min(self.workspace.behaviors.len().saturating_sub(1));

        // Like VS Code: the side bar, and the editor with what belongs to it on the right of it.
        let objects_id = if self.is_user_patch { "user_patch_objects" } else { "nemesis_objects" };
        egui::Panel::left(objects_id).resizable(true).default_size(320.0).show(ui, |ui| {
            self.behaviors_section(ui, i18n, notify);
            if !self.workspace.behaviors.is_empty() {
                self.object_list(ui, i18n);
            }
        });
        if self.workspace.behaviors.is_empty() {
            egui::CentralPanel::default().show(ui, |ui| ui.weak(i18n.t(I18nKey::NemesisNoBehavior)));
            return;
        }
        self.class_picker_window(ui.ctx(), i18n);
        self.object_editor(ui, i18n);
    }

    // ── Workspace on disk ────────────────────────────────────────────────────

    /// Loads the work of the code when it changes, and saves pending edits.
    fn sync_workspace(&mut self, notify: &mut Notification, i18n: &I18nMap) {
        let code = self.mod_code.trim();
        if is_valid_mod_code(code) && self.loaded_for.as_deref() != Some(code) {
            let code = code.to_owned();
            self.save_now(notify, i18n); // the previous workspace
            match Workspace::load(&self.cache_root, &code) {
                Ok(workspace) => {
                    self.workspace = workspace;
                    self.reset_names();
                    self.tabs.clear();
                }
                Err(err) => {
                    notify.error(format!("{}: {err}", i18n.t(I18nKey::LoadFailedMessage)));
                }
            }
            self.editor_for = None;
            self.selected = None;
            self.restore_session();
            remember_recent(
                &mut self.recent,
                RecentWork { output: self.output.trim().to_owned(), mod_code: code.clone() },
            );
            self.loaded_for = Some(code);
        }

        // What is open is part of the work: back to it when it is opened again.
        if self.session_seen.0 != self.active || self.session_seen.1 != self.selected {
            self.session_seen = (self.active, self.selected.clone());
            if self.loaded_for.is_some() && !self.workspace.behaviors.is_empty() {
                self.dirty_since.get_or_insert_with(Instant::now);
            }
        }

        if self.dirty_since.is_some_and(|since| since.elapsed() >= SAVE_DELAY) {
            self.save_now(notify, i18n);
        }
    }

    fn save_now(&mut self, notify: &mut Notification, i18n: &I18nMap) {
        let Some(code) = self.loaded_for.clone() else { return };
        if self.dirty_since.take().is_none() {
            return;
        }
        self.workspace.session = self.current_session();
        // It has no `Export`: what is edited is what the next merge gets.
        if self.is_user_patch {
            match self.workspace.export(Path::new(&self.output), &code, self.parser_mode) {
                Ok((_, invalid)) if invalid.is_empty() => {}
                Ok((_, invalid)) => notify.warn(format!(
                    "{} ({}):\n{}",
                    i18n.t(I18nKey::NemesisRejected),
                    invalid.len(),
                    invalid.join("\n")
                )),
                Err(err) => notify.error(format!("{}: {err}", i18n.t(I18nKey::NemesisExportFailed))),
            }
            self.mod_written = true;
        }
        if let Err(err) = self.workspace.save(&self.cache_root, &code) {
            notify.error(format!("{}: {err}", i18n.t(I18nKey::NemesisSaveFailed)));
        }
        // The output may have changed.
        remember_recent(&mut self.recent, RecentWork { output: self.output.trim().to_owned(), mod_code: code });
    }

    /// What is open now, to write with the work.
    fn current_session(&self) -> Session {
        let behaviors = &self.workspace.behaviors;
        let name_of = |at: usize| behaviors.get(at).map_or_else(String::new, |behavior| behavior.name.clone());
        Session {
            mod_dir: self.mod_dir.clone(),
            output: self.output.clone(),
            active: name_of(self.active),
            selected: self.selected.clone(),
            tabs: self
                .tabs
                .all()
                .map(|tab| SessionTab { behavior: name_of(tab.behavior), id: tab.id.clone(), pinned: tab.pinned })
                .collect(),
        }
    }

    /// Back to what the work just loaded had open. What no longer exists is left out.
    fn restore_session(&mut self) {
        let session = std::mem::take(&mut self.workspace.session);
        let behaviors = &self.workspace.behaviors;
        let at_of = |name: &str| behaviors.iter().position(|behavior| behavior.name == name);
        for tab in &session.tabs {
            if let Some(at) = at_of(&tab.behavior)
                && behaviors[at].text(&tab.id).is_some()
            {
                self.tabs.restore(at, &tab.id, tab.pinned);
            }
        }
        self.active = at_of(&session.active).unwrap_or(0);
        self.selected = session.selected.filter(|id| behaviors.get(self.active).is_some_and(|b| b.text(id).is_some()));
        if !session.mod_dir.is_empty() {
            self.mod_dir = session.mod_dir;
        }
        // The user patch's is decided by the patch page's.
        if !session.output.is_empty() && !self.is_user_patch {
            self.output = session.output;
        }
        self.session_seen = (self.active, self.selected.clone());
    }

    /// Opens a work of the list of recent ones. If its cache is gone, the mod it was exported to is
    /// read instead; if that is gone too, it leaves the list.
    fn open_recent(&mut self, work: &RecentWork, notify: &mut Notification, i18n: &I18nMap) {
        if Workspace::exists(&self.cache_root, &work.mod_code) {
            self.switch_work(&work.mod_code, &work.output, "", notify, i18n);
        } else if let Some(dir) = workspace::patch_dir_of_code(Path::new(&work.output), &work.mod_code) {
            self.open_mod(&dir, notify, i18n);
        } else {
            notify.warn(format!("{}: {} ({})", i18n.t(I18nKey::NemesisRecentMissing), work.mod_code, work.output));
            self.recent.retain(|other| other.mod_code != work.mod_code);
        }
    }

    /// Back to the start screen. The work stays in its cache.
    fn close_work(&mut self, notify: &mut Notification, i18n: &I18nMap) {
        self.save_now(notify, i18n);
        self.workspace = Workspace::default();
        self.reset_names();
        self.tabs.clear();
        self.loaded_for = None;
        self.mod_code.clear();
        self.output.clear();
        self.mod_dir.clear();
        self.active = 0;
        self.selected = None;
        self.editor_for = None;
        self.project_open = false;
    }

    /// `New` of the start screen: the work's first save makes it exist.
    fn create_work(&mut self, code: String, output: String) {
        self.new_work = None;
        self.mod_code = code;
        self.output = output;
        self.mod_dir.clear();
        self.project_open = true; // Its name and author are still to be set.
        self.dirty_since.get_or_insert_with(Instant::now);
    }

    /// The folder dialog for [`Self::open_mod`].
    fn pick_mod(&mut self, notify: &mut Notification, i18n: &I18nMap) {
        if let Some(dir) = gui_shared::rfd::FileDialog::new().pick_folder() {
            self.open_mod(&dir, notify, i18n);
        }
    }

    /// Opens a mod (its root or a patch directory), like VS Code a folder: the work on it goes on if
    /// there is one, with its unexported edits. Else it is read, and exported back to where it is.
    fn open_mod(&mut self, selected: &Path, notify: &mut Notification, i18n: &I18nMap) {
        let not_a_mod = |notify: &mut Notification| {
            notify.error(format!("{}: {}", i18n.t(I18nKey::NemesisNotAMod), selected.display()));
        };
        let Some(root) = workspace::mod_root_of(selected) else { return not_a_mod(notify) };
        let code = workspace::code_of(selected);
        if !is_valid_mod_code(&code) {
            return not_a_mod(notify);
        }
        if code.eq_ignore_ascii_case(gui_shared::patch::USER_PATCH_CODE) {
            notify.warn(i18n.t(I18nKey::NemesisUserPatchReserved));
            return;
        }
        let source = root.display().to_string();
        let is_source = |path: &str| path.eq_ignore_ascii_case(&source);
        match Workspace::cached_session(&self.cache_root, &code) {
            Some(session) if is_source(&session.mod_dir) || is_source(&session.output) => {
                self.switch_work(&code, &source, &source, notify, i18n);
                return;
            }
            Some(_) => notify.warn(format!("{}: {code}", i18n.t(I18nKey::NemesisWorkReplaced))),
            None => {}
        }
        self.read_mod(selected, source, notify, i18n);
    }

    /// Replaces the work by what is in the mod at `selected`. Returns whether it could be read.
    fn read_mod(&mut self, selected: &Path, source: String, notify: &mut Notification, i18n: &I18nMap) -> bool {
        match Workspace::import(selected, Path::new(&self.template_dir)) {
            Ok((workspace, code, skipped)) => {
                self.save_now(notify, i18n);
                self.workspace = workspace;
                self.reset_names();
                self.tabs.clear();
                self.mod_code.clone_from(&code);
                self.output.clone_from(&source);
                self.mod_dir = source;
                self.new_work = None;
                // What was read is the work, rather than what may be in the cache.
                remember_recent(&mut self.recent, RecentWork { output: self.output.clone(), mod_code: code.clone() });
                self.loaded_for = Some(code);
                self.mark_dirty();
                self.active = 0;
                self.selected = None;
                self.editor_for = None;
                if !skipped.is_empty() {
                    notify.warn(format!("{}: {}", i18n.t(I18nKey::NemesisNoTemplateFor), skipped.join(", ")));
                }
                true
            }
            Err(err) => {
                notify.error(format!("{}: {err}", i18n.t(I18nKey::LoadFailedMessage)));
                false
            }
        }
    }

    /// `Reload from the mod`: discards the work's edits. Where it is exported to and the mods it
    /// refers to are not in the mod, so they stay.
    fn reload_from_mod(&mut self, notify: &mut Notification, i18n: &I18nMap) {
        let Some(code) = self.loaded_for.clone() else { return };
        let source = self.mod_dir.trim().to_owned();
        let Some(dir) = workspace::patch_dir_of_code(Path::new(&source), &code) else {
            notify.error(format!("{}: {source}", i18n.t(I18nKey::NemesisNotAMod)));
            return;
        };
        let (output, refs) = (self.output.clone(), self.workspace.ref_mods.clone());
        if self.read_mod(&dir, source, notify, i18n) {
            self.output = output;
            self.workspace.ref_mods = refs;
            self.workspace.reload_refs();
            self.reset_names();
        }
    }

    /// The next or previous tab, with the keyboard staying in the editor.
    fn step_tab(&mut self, current: &str, forward: bool) {
        if let Some(next) = self.tabs.neighbor(self.active, current, forward) {
            next.clone_into(self.selected.get_or_insert_default());
            self.focus_editor = Some(FocusAt::Keep);
        }
    }

    /// Vim `Ctrl+O` / `Ctrl+I`: the previous / next visited object (skips deleted ones).
    fn walk_history(&mut self, older: bool, current: &str) {
        let mut current = current.to_string();
        while let Some(target) = self.history.walk(older, &current) {
            if self.workspace.behaviors[self.active].text(&target).is_some() {
                self.selected = Some(target);
                self.focus_editor = Some(FocusAt::Keep);
                return;
            }
            current = target;
        }
    }

    /// Opens `document` at `line` (go to definition, a click in the references window).
    fn open_location(&mut self, current: &str, document: &str, line: usize) {
        if document != current {
            self.history.visit(current);
            document.clone_into(self.selected.get_or_insert_default());
        }
        self.focus_editor = Some(FocusAt::Line(line));
    }

    /// Double click or menu in the tree and the list: shows `id` in a tab that stays.
    fn keep_open(&mut self, id: &str) {
        let current = self.selected.as_deref().filter(|selected| *selected != id);
        self.tabs.open(self.active, id, current);
        self.tabs.pin(self.active, id);
        id.clone_into(self.selected.get_or_insert_default());
    }

    /// Loads `id` into the editor if it is not the open object already.
    fn open_in_editor(&mut self, id: &str) {
        if self.editor_for.as_ref().is_some_and(|(active, open)| *active == self.active && open == id) {
            return;
        }
        // The tab being left keeps its undo history and Vim state.
        let left = self.editor_for.take();
        if let Some((behavior, left)) = &left {
            self.tabs.store_state(*behavior, left, std::mem::take(&mut self.editor));
        }
        let current = left.as_ref().filter(|(behavior, _)| *behavior == self.active).map(|(_, id)| id.as_str());
        self.tabs.open(self.active, id, current);
        self.editor = self.tabs.take_state(self.active, id);
        self.refresh_trigger_names();

        let text = self.workspace.behaviors[self.active].text(id).unwrap_or_default();
        text.clone_into(&mut self.editor_text);
        self.preview = None;
        self.editor_for = Some((self.active, id.to_owned()));
    }

    /// In a clip data file, trigger lines complete with the known names.
    fn refresh_trigger_names(&mut self) {
        let behavior = &self.workspace.behaviors[self.active];
        if !behavior.adsf {
            return;
        }
        let template_dir = Path::new(&self.template_dir);
        let template = self.template_triggers.get_or_insert_with(|| anim_data::template_trigger_names(template_dir));
        self.trigger_names = anim_data::trigger_names(template, behavior);
    }

    /// Gives the editor the keyboard and moves its cursor, if that was asked for.
    fn apply_focus_request(&mut self, ctx: &egui::Context, editor: egui::Id) {
        let Some(at) = self.focus_editor.take() else { return };
        let index = match at {
            FocusAt::Line(line) => {
                egui_code_editor::text_pos::LinePos { line, column: 0 }.to_char_index(&self.editor_text)
            }
            FocusAt::Keep => egui::TextEdit::load_state(ctx, editor)
                .and_then(|state| state.cursor.char_range())
                .map_or(0, |range| range.primary.index.0),
        };
        egui_code_editor::set_cursor(ctx, editor, index);
    }

    /// The pause after typing is over: `preview` goes, for [`Self::refresh_preview`] to compute it.
    /// Until then the last results stay on screen, as in VS Code.
    fn expire_preview(&mut self, ctx: &egui::Context) {
        let Some(typed_at) = self.typed_at else { return };
        let left = REFRESH_DELAY.saturating_sub(typed_at.elapsed());
        if left.is_zero() {
            self.typed_at = None;
            self.preview = None;
        } else {
            ctx.request_repaint_after(left);
        }
    }

    /// Recomputes the patch, its check and the underlines after the text changed.
    fn refresh_preview(&mut self, id: &str) {
        if self.preview.is_some() {
            return;
        }
        let behavior = &self.workspace.behaviors[self.active];
        let patch = behavior.patch(self.mod_code.trim(), id);
        let check = patch.as_deref().map_or(Ok(()), |patch| behavior.validate(id, patch, self.parser_mode));
        self.field_problems = if behavior.is_anim_data() {
            self.anim_roles = anim_lines::roles(behavior.adsf, id, &self.editor_text);
            Vec::new() // Only the parser's, below: see `anim_lines`.
        } else if behavior.is_ref(id) {
            Vec::new() // not this mod's to fix
        } else {
            let is_new = behavior.added.contains_key(id);
            diagnostics::field_diagnostics(&self.editor_text, behavior, is_new, self.mod_code.trim())
        };
        // Keep the ▲ / ▼ position across edits.
        let current = self.preview_view.as_ref().map_or(0, |view| view.current);
        self.preview_view = patch.as_deref().map(preview::Preview::new);
        if let Some(view) = &mut self.preview_view {
            view.keep_current(current);
        }
        // The parser reads the patch, whose lines are the preview's: back to the editor's.
        if let Err(error) = &check {
            let view = self.preview_view.as_ref();
            let to_editor_line = |line| view.and_then(|view| view.editor_line_of(line, &self.editor_text));
            diagnostics::add_parser_error(&mut self.field_problems, &self.editor_text, error, to_editor_line);
        }
        self.preview = Some((patch, check));
        self.inline_diff = behavior
            .template_object(id)
            .map_or_else(InlineDiff::default, |template| inline_diff::changes(&template.text, &self.editor_text));
        self.refresh_work_check(id);
    }

    /// Enum / flag values only depend on the template: computed once per behavior.
    fn field_values(&mut self) -> Rc<language::FieldValues> {
        let active = self.active;
        let cached = self.field_values.take().filter(|(cached, _)| *cached == active);
        let (_, values) =
            cached.unwrap_or_else(|| (active, Rc::new(language::field_values(&self.workspace.behaviors[active]))));
        self.field_values = Some((active, Rc::clone(&values)));
        values
    }

    /// Does what the tab bar asked for. `current`: the object shown now.
    fn apply_tab_event(&mut self, event: Option<TabEvent>, current: &str) {
        let active = self.active;
        match event {
            Some(TabEvent::Show(id)) => self.selected = Some(id),
            Some(TabEvent::KeepOpen(id)) => self.tabs.pin(active, &id),
            Some(TabEvent::CloseToTheRight(id)) => {
                self.tabs.close_to_the_right(active, &id);
                if !self.tabs.of(active).any(|tab| tab.id == current) {
                    self.selected = Some(id);
                }
            }
            Some(TabEvent::CloseAll) => {
                self.tabs.close_all(active);
                self.selected = None;
                self.editor_for = None;
            }
            Some(TabEvent::Close(id)) => {
                let neighbor = self.tabs.close(active, &id);
                if id == current {
                    self.selected = neighbor;
                    self.editor_for = None; // its editor state went with the tab
                }
            }
            Some(TabEvent::CloseOthers(id)) => {
                self.tabs.close_others(active, &id);
                self.selected = Some(id);
            }
            None => {}
        }
    }

    /// Does what the editor asked for. Returns whether the keyboard goes to the object tree.
    fn apply_editor_result(&mut self, result: EditorResult, id: &str) -> bool {
        if result.changed {
            self.workspace.behaviors[self.active].set_text(id, self.editor_text.clone());
            self.typed_at = Some(Instant::now());
            self.tabs.pin(self.active, id); // edited: it stays open
            self.object_changed(id);
        }
        if let Some(line) = result.jump_to_line {
            self.focus_editor = Some(FocusAt::Line(line));
        }
        self.apply_tab_event(result.tab_event, id);
        if let Some(index) = result.inline_revert {
            self.revert_inline(id, index);
        }
        match result.command {
            // Vim `:w`: save the work now.
            Some(EditorCommand::Save) => self.dirty_since = Instant::now().checked_sub(SAVE_DELAY),
            Some(EditorCommand::JumpBack) => self.walk_history(true, id),
            Some(EditorCommand::JumpForward) => self.walk_history(false, id),
            Some(EditorCommand::NextTab) => self.step_tab(id, true),
            Some(EditorCommand::PreviousTab) => self.step_tab(id, false),
            Some(EditorCommand::CloseTab) => {
                self.apply_tab_event(Some(TabEvent::Close(id.to_owned())), id);
                self.focus_editor = Some(FocusAt::Keep);
            }
            Some(EditorCommand::TogglePreview) => self.show_preview = !self.show_preview,
            Some(EditorCommand::ToggleInlineDiff) => self.show_inline_diff = !self.show_inline_diff,
            Some(EditorCommand::ToggleExplorer | EditorCommand::GoToDefinition | EditorCommand::FindReferences)
            | None => {}
        }
        if let Some(older) = result.walk_history {
            self.walk_history(older, id);
        }
        if let Some((document, line)) = result.definition {
            self.open_location(id, &document, line);
        }
        result.focus_tree || result.command == Some(EditorCommand::ToggleExplorer)
    }

    /// `↶` in the inline diff: puts a changed place back to the template.
    fn revert_inline(&mut self, id: &str, index: usize) {
        let behavior = &mut self.workspace.behaviors[self.active];
        let Some(template) = behavior.template_object(id) else { return };
        self.editor_text = inline_diff::revert(&template.text, &self.editor_text, index);
        behavior.set_text(id, self.editor_text.clone());
        self.preview = None;
        self.object_changed(id);
    }

    /// `↶` in the preview: puts changed block `block` back to the template.
    fn revert_block(&mut self, id: &str, block: usize) {
        let behavior = &mut self.workspace.behaviors[self.active];
        let Some(template) = behavior.template_object(id) else { return };
        self.editor_text = nemesis_diff::revert_hunk(&template.text, &self.editor_text, block);
        behavior.set_text(id, self.editor_text.clone());
        self.preview = None;
        self.object_changed(id);
    }

    /// The text of object `id` changed, or it was added or deleted: `names` takes that one in.
    fn object_changed(&mut self, id: &str) {
        if !self.changed_objects.iter().any(|changed| changed == id) {
            self.changed_objects.push(id.to_owned());
        }
        self.mark_dirty();
    }

    /// The behaviors themselves changed (loaded, closed): `names` starts over.
    fn reset_names(&mut self) {
        self.names = None;
        self.changed_objects.clear();
    }

    /// Something to save.
    fn mark_dirty(&mut self) {
        self.dirty_since.get_or_insert_with(Instant::now);
    }

    fn add_behavior(&mut self, path: &Path, notify: &mut Notification, i18n: &I18nMap) {
        match Behavior::open(path) {
            Ok(behavior) => {
                if let Some(at) = self.workspace.behaviors.iter().position(|b| b.name == behavior.name) {
                    self.active = at; // already there
                    return;
                }
                self.workspace.behaviors.push(behavior);
                self.workspace.reload_refs();
                self.active = self.workspace.behaviors.len() - 1;
                self.selected = None; // an id of the other behavior
                self.mark_dirty();
            }
            Err(err) => notify.error(format!("{}: {err}", i18n.t(I18nKey::LoadFailedMessage))),
        }
    }

    fn export(&mut self, notify: &mut Notification, i18n: &I18nMap) {
        self.save_now(notify, i18n);
        let Some(code) = self.loaded_for.clone() else { return };
        let output = self.output.trim().to_owned();
        let exported = self.workspace.export(Path::new(&output), &code, self.parser_mode);
        self.mod_written |= exported.is_ok();
        match exported {
            Ok((written, invalid)) if invalid.is_empty() => notify.success(format!(
                "{} ({written}): {}",
                i18n.t(I18nKey::ExportedMessage),
                Path::new(&output).join("Nemesis_Engine").join("mod").join(&code).display()
            )),
            Ok((_, invalid)) => notify.warn(format!(
                "{} ({}):\n{}",
                i18n.t(I18nKey::NemesisRejected),
                invalid.len(),
                invalid.join("\n")
            )),
            Err(err) => notify.error(format!("{}: {err}", i18n.t(I18nKey::NemesisExportFailed))),
        }
        // What it wrote is what the next export replaces.
        self.dirty_since.get_or_insert_with(Instant::now);
        self.save_now(notify, i18n);
    }

    /// The keyboard goes to the object tree (Vim `Space e`, or the palette).
    fn enter_tree(&mut self, ui: &egui::Ui, editor: egui::Id) {
        self.tree_focus = true;
        self.tree_view = true;
        self.show_vars = false;
        self.filter.clear();
        ui.memory_mut(|m| m.surrender_focus(editor));
    }

    /// The keyboard goes back to the editor.
    const fn leave_tree(&mut self) {
        self.tree_focus = false;
        self.focus_editor = Some(FocusAt::Keep);
    }

    /// The flat list has the same keys as the tree, without the folding.
    fn list_keys(&mut self, ui: &egui::Ui, graph: &tree::Graph) {
        let rows = self.list.as_ref().map_or(&[][..], |list| list.rows.as_slice());
        let selected = self.selected.as_deref();
        let mut current = rows.iter().position(|&node| selected == Some(graph.id(node))).unwrap_or(0);
        let mut leave = false;
        for input in ui::tree::read_keys(ui, &mut self.list_leader) {
            leave |= input == tree::TreeInput::Leave;
            current = input.step(current, rows.len()).unwrap_or(current);
        }
        if let Some(&node) = rows.get(current) {
            graph.id(node).clone_into(self.selected.get_or_insert_default());
        }
        if leave {
            self.leave_tree();
        }
    }

    /// Recomputes the flat list when the objects, the filter or "changed only" changed.
    ///
    /// New objects come first and show where they are linked: `+ #mymod$0  hkbFoo  ← #0106`.
    ///
    /// Without a filter the labels do not matter, so typing in the editor does not recompute it.
    fn refresh_list(&mut self, graph: &tree::Graph) {
        let generation = if self.filter.is_empty() { graph.tree_generation() } else { graph.generation() };
        let fresh = self.list.as_ref().is_some_and(|list| {
            list.generation == generation && list.filter == self.filter && list.changed_only == self.changed_only
        });
        if fresh {
            return;
        }
        let filter = self.filter.to_lowercase();
        let rows = graph
            .nodes()
            .filter(|&node| !self.changed_only || graph.is_changed(node))
            .filter(|&node| filter.is_empty() || graph.label(node).to_lowercase().contains(&filter))
            .collect();
        // Anim data is grouped by actor: see `ui::anim_tree`.
        let mut rows: Vec<tree::Node> = rows;
        if self.workspace.behaviors[self.active].is_anim_data() {
            rows.sort_by_cached_key(|&node| graph.id(node).to_ascii_lowercase());
        }
        self.list = Some(ObjectList { generation, filter: self.filter.clone(), changed_only: self.changed_only, rows });
    }

    /// Deletes a new object, or drops the edits of a template object.
    fn revert_or_delete(&mut self, id: &str) {
        let behavior = &mut self.workspace.behaviors[self.active];
        behavior.edits.remove(id);
        if behavior.added.remove(id).is_some() {
            let neighbor = self.tabs.close(self.active, id);
            if self.selected.as_deref() == Some(id) {
                self.selected = neighbor;
            }
        }
        self.editor_for = None;
        self.refresh_work_check(id);
        self.object_changed(id);
    }

    fn new_object(&mut self, from: Option<&str>) {
        let code = self.mod_code.trim().to_string();
        let id = self.workspace.behaviors[self.active].add_object(&code, from);
        self.object_changed(&id);
        self.selected = Some(id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_last_opened_work_is_first_and_listed_once() {
        let work = |code: &str| RecentWork { output: "o".to_owned(), mod_code: code.to_owned() };
        let mut recent = vec![work("a"), work("b")];
        remember_recent(&mut recent, work("b"));
        assert_eq!(recent, [work("b"), work("a")]);
        remember_recent(&mut recent, work("c"));
        assert_eq!(recent, [work("c"), work("b"), work("a")]);

        for at in 0..MAX_RECENT * 2 {
            remember_recent(&mut recent, work(&at.to_string()));
        }
        assert_eq!(recent.len(), MAX_RECENT);
    }

    /// What one edit costs on the largest object of the largest template, step by step.
    #[test]
    #[ignore = "needs the template files of the repository; prints timings"]
    fn time_one_edit() {
        use std::time::Instant;
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../resource/assets/templates/meshes/actors/character/behaviors/0_master.bin");
        let mut behavior = Behavior::open(&path).unwrap();
        let largest = behavior.objects.iter().max_by_key(|o| o.text.len()).unwrap();
        let (id, text) = (largest.id.clone(), largest.text.clone());
        println!("{id} {} {} lines; {} objects", largest.class, text.lines().count(), behavior.objects.len());
        let edited = text.replacen("<hkparam", "<hkparam ", 1);

        macro_rules! time {
            ($label:expr, $e:expr) => {{
                let start = Instant::now();
                let value = $e;
                println!("{:<24}{:?}", $label, start.elapsed());
                value
            }};
        }
        let mut names = time!("Names::new", names::Names::new(&behavior));
        time!("set_text", behavior.set_text(&id, edited.clone()));
        let patch = time!("patch", behavior.patch("m", &id)).unwrap();
        let check = time!("validate", behavior.validate(&id, &patch, ParserMode::Strict));
        println!("  valid: {}", check.is_ok());
        time!("field_diagnostics", diagnostics::field_diagnostics(&edited, &behavior, false, "m"));
        time!("Preview::new", preview::Preview::new(&patch));
        time!("inline_diff", inline_diff::changes(&text, &edited));
        time!("apply_changes", names.apply_changes(&behavior, std::slice::from_ref(&id)));

        // A typo in a field name, and fixing it.
        let broken = text.replacen("name=\"", "name=\"x", 2);
        behavior.set_text(&id, broken.clone());
        let bad_patch = time!("broken: patch", behavior.patch("m", &id)).unwrap();
        let bad = time!("broken: validate", behavior.validate(&id, &bad_patch, ParserMode::Strict));
        println!("  valid: {}, error of {} bytes", bad.is_ok(), bad.as_ref().err().map_or(0, String::len));
        time!("broken: diagnostics", diagnostics::field_diagnostics(&broken, &behavior, false, "m"));
        time!("broken: names", names.apply_changes(&behavior, std::slice::from_ref(&id)));
        behavior.set_text(&id, edited);
        time!("fixed: names", names.apply_changes(&behavior, std::slice::from_ref(&id)));
        time!("fixed: graph rows", {
            let mut state = tree::TreeState::default();
            state.prepare(&names.graph, Some(&id), false);
            state.rows().len()
        });
    }

    /// Debug aid on a real mod (`PATCH_EDITOR_MOD`, its root): draws every changed object, and shows
    /// where each is in the tree. A panic names the object being drawn.
    #[test]
    #[ignore = "needs a real mod"]
    fn draw_every_changed_object_of_a_real_mod() {
        let selected = std::env::var("PATCH_EDITOR_MOD").unwrap();
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../resource/assets/templates");
        let (workspace, code, _) = Workspace::import(Path::new(&selected), &root).unwrap();
        let mut tool = PatchTool { workspace, mod_code: code, tree_view: true, ..PatchTool::default() };
        let i18n = I18nMap::new();

        for active in 0..tool.workspace.behaviors.len() {
            tool.active = active;
            let behavior = &tool.workspace.behaviors[active];
            let ids: Vec<String> = behavior.edits.keys().chain(behavior.added.keys()).cloned().collect();
            for id in ids {
                println!("{active} {id}");
                tool.selected = Some(id.clone());
                egui::__run_test_ui(|ui| {
                    tool.object_list(ui, &i18n);
                    tool.object_editor(ui, &i18n);
                });
                if tool.workspace.behaviors[active].is_anim_data() {
                    continue; // listed per actor, not in the tree
                }
                let names = tool.names();
                let path = &tool.workspace.behaviors[active].meshes_path;
                let tree = &tool.trees[path];
                let found = tree.rows().get(tree.cursor).map(|row| names.graph.id(row.node));
                assert_eq!(found, Some(id.as_str()), "`{id}` is not selected in the tree");
                assert_eq!(tool.editor_for.as_ref().map(|(_, open)| open.as_str()), Some(id.as_str()));
                assert!(!tool.editor_text.is_empty(), "`{id}` opened empty");
            }
        }
    }
}

#[cfg(test)]
mod work_tests {
    use super::*;

    fn tool_in(dir: &Path) -> PatchTool {
        let mut tool = PatchTool { restored: true, ..PatchTool::default() };
        tool.set_patch_output(&dir.display().to_string(), &mut Notification::default(), &I18nMap::new());
        tool
    }

    /// A work is kept in the patch page's output, and another one can be made next to it.
    #[test]
    fn a_new_work_can_be_made_beside_another() {
        let dir = std::env::temp_dir().join(format!("d_merge_new_work_{}", std::process::id()));
        let (mut notify, i18n) = (Notification::default(), I18nMap::new());
        let mut tool = tool_in(&dir);

        tool.create_work("aaaa".to_owned(), "D:/mods/a".to_owned());
        tool.sync_workspace(&mut notify, &i18n);
        assert_eq!(tool.loaded_for.as_deref(), Some("aaaa"));
        tool.workspace.info.name = "first".to_owned();
        tool.mark_dirty();
        tool.save_now(&mut notify, &i18n);

        // `File > New`: the first one goes away, and the second one starts empty.
        tool.close_work(&mut notify, &i18n);
        assert_eq!(tool.loaded_for, None);
        tool.create_work("bbbb".to_owned(), "D:/mods/a".to_owned());
        tool.sync_workspace(&mut notify, &i18n);
        assert_eq!(tool.loaded_for.as_deref(), Some("bbbb"));
        assert_eq!(tool.workspace.info.name, "");
        tool.save_now(&mut notify, &i18n);

        let root = workspace::cache_root_of(&dir.display().to_string());
        let both = (Workspace::exists(&root, "aaaa"), Workspace::exists(&root, "bbbb"));

        // Back to the first one.
        tool.switch_work("aaaa", "D:/mods/a", "", &mut notify, &i18n);
        tool.sync_workspace(&mut notify, &i18n);
        let name = tool.workspace.info.name.clone();
        std::fs::remove_dir_all(&dir).unwrap();

        assert_eq!(both, (true, true));
        assert_eq!(name, "first");
    }

    /// What is open is saved where it belongs when the output directory changes.
    #[test]
    fn another_output_directory_has_other_works() {
        let dir = std::env::temp_dir().join(format!("d_merge_other_output_{}", std::process::id()));
        let (mut notify, i18n) = (Notification::default(), I18nMap::new());
        let mut tool = tool_in(&dir.join("one"));
        tool.create_work("aaaa".to_owned(), "D:/mods/a".to_owned());
        tool.sync_workspace(&mut notify, &i18n);

        tool.set_patch_output(&dir.join("two").display().to_string(), &mut notify, &i18n);
        assert_eq!(tool.loaded_for, None);
        let in_one = Workspace::exists(&workspace::cache_root_of(&dir.join("one").display().to_string()), "aaaa");
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(in_one);
    }

    /// `LspOptions` holds the keymap too: it must not put back the one it was given.
    #[test]
    fn the_keymap_box_changes_the_keymap() {
        let mut tool = PatchTool::default();
        let mut editor = LspOptions { editor_mode: EditorMode::Normal, ..LspOptions::default() };
        tool.read_editor_options(&editor);
        tool.editor_mode = EditorMode::Vim;
        tool.write_editor_options(&mut editor);
        assert_eq!(editor.editor_mode, EditorMode::Vim);

        tool.read_editor_options(&editor);
        tool.lsp.hover = !tool.lsp.hover;
        tool.write_editor_options(&mut editor);
        assert_eq!(editor.hover, tool.lsp.hover);
        assert_eq!(editor.editor_mode, EditorMode::Vim);
    }
}
