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

mod anim;
mod changes;
mod checklist;
mod editor;
mod new_work;
mod problems;
mod repeat;
mod search;
mod template_tree;
mod tool;
mod tree;
mod ui;
mod user_patch;
mod vars_edit;
mod workspace;

use std::{path::PathBuf, rc::Rc, time::Instant};

use egui_code_editor::EditorCommand;
use egui_widgets::notify::Notification;
use gui_shared::{
    i18n::{I18nKey, I18nMap},
    settings::{
        pages::{
            patch::behavior::ParserMode,
            patch_editor::{PatchEditorSettings, RecentWork},
        },
        ui::editor::LspOptions,
    },
};

use self::workspace::{Behavior, Workspace};

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
    /// The commands this frame's keys asked for: see [`Self::key_command`].
    key_commands: Vec<&'static str>,
    /// The keys of the palette and its `⚙`. See [`keybinding::KeyUi`].
    key_ui: keybinding::KeyUi,
    /// Copy path / reveal of an object asked for by a tab or the editor: done by
    /// [`Self::file_keys`], which has what it needs.
    file_asked: Option<(tool::list::FileAction, String)>,
    /// The editor of the open object and what is beside it: preview, merge, problems. See [`tool::editing`].
    pane: tool::editing::EditorPane,
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
    /// What was edited of the active behavior since [`Self::sync_pandora_doc`].
    pandora_edited: Vec<String>,
    /// A mod with several codes was opened: which of them? See [`ui::code_choice`].
    code_choice: Option<ui::code_choice::ModCodeChoice>,
    /// See [`new_work`]. `Some`: its form is shown on the start screen.
    new_work: Option<new_work::NewWork>,
    mod_code: String,
    /// `settings.behavior.template_dir` (`.bin` or `.xml` templates under `meshes/`).
    template_dir: String,
    /// The window of the templates to add. See [`ui::template_picker`].
    picker: ui::template_picker::TemplatePicker,
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
    selected: Option<Rc<str>>,
    /// The objects of the explorer: the filters, the tree or the list, the keyboard in them. See [`tool::list`].
    explorer: tool::list::Explorer,
    /// The mod code being typed to rename the work to, while that form is open.
    rename_to: Option<String>,
    /// What runs on another thread: reading a mod, renaming its code. See [`tool::job`].
    job: Option<tool::job::Job>,

    /// The patch page's parser mode: what d-merge will accept, so what is checked here.
    parser_mode: ParserMode,
    /// The "animation data" window.
    anim_data_dialog: Option<ui::anim_data::AnimDataDialog>,
    /// The objects open in tabs.
    tabs: editor::tabs::EditorTabs,

    /// The "New" class list is open, and its filter.
    class_picker: bool,
    class_filter: String,
    search: search::Search,
    /// The search field takes the keyboard next frame.
    focus_search: bool,
    /// The side bar: which view, and whether it shows. See [`ui::side_bar`].
    side: ui::side_bar::SideBar,
    /// Edits made, counted: what is found anew after one, e.g. [`changes::Pending`].
    edits: u64,
    /// The Changes view: what changed since the last revision, and what is typed there. See [`changes`].
    scm: changes::Scm,
    /// The window making many objects from one. See [`repeat`].
    repeat: Option<ui::repeat::RepeatForm>,
    /// The diffs and blames of the history open as tabs. See [`ui::revision_view`].
    views: ui::revision_view::Views,
    /// The problems of the whole work: the status bar's counts, the Problems panel. See [`problems`].
    problems: problems::Problems,
    /// The tree of the animation data files. See [`ui::anim_tree`].
    anim_tree: ui::anim_tree::AnimTreeState,
    /// A conflict's value to change in the editor, taken in on the next frame.
    pending_resolution: Option<gui_shared::patch::ResolveRequest>,
    /// The works opened lately, the last one first. From the settings.
    recent: Vec<RecentWork>,
    /// The behavior and the object the work last saved had open.
    session_seen: (usize, Option<Rc<str>>),
    /// The window of the user patch, and what it asks for.
    user_patch: user_patch::UserPatchView,
    /// The user patch written as a mod of its own. See [`user_patch::ModExport`].
    mod_export: user_patch::ModExport,
    /// The projects of the clip data template, read once. See [`anim::data::template_projects`].
    template_projects: Option<Vec<String>>,
    /// The "duplicate to" window.
    duplicate_form: Option<ui::anim_data::DuplicateForm>,
    /// What a row's menu asked to duplicate: `(files, to the project)`.
    pending_duplicate: Option<(Vec<String>, String)>,
    /// The "new clips" window.
    new_clips: Option<ui::anim_data::NewClipsForm>,
    /// The events / variables panel shows variables (else events).
    show_variables: bool,
    /// `(behavior, names)` of the events / variables panel and the completion.
    names: Option<(usize, Rc<editor::names::Names>)>,
    /// Objects edited since `names` last looked. See [`Self::object_changed`].
    changed_objects: Vec<String>,
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
    /// The `⚙` of this palette action: record its keys.
    configure: Option<&'static str>,
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
    /// A tab of [`ui::revision_view::Views`]: shown, or closed.
    ShowView(usize),
    CloseView(usize),
    /// A double click on a view's tab: it is kept.
    KeepView(usize),
    /// A view's menu: every other tab of the behavior closes, objects' and views'.
    CloseOthersOfView(usize),
    /// A view's menu: the views to its right close.
    CloseViewsRight(usize),
    /// The object's file: its path to the clipboard, or shown in the file manager.
    File(tool::list::FileAction, String),
}

/// See [`tool::list::Explorer::list`].
#[derive(Debug)]
struct ObjectList {
    /// The state of the graph it was computed from.
    generation: u64,
    filter: String,
    changed_only: bool,
    unlinked_only: bool,
    rows: Vec<tree::Node>,
}

impl PatchTool {
    /// A mod was written since the last call: what the conflict check found may be old.
    pub(crate) const fn take_mod_written(&mut self) -> bool {
        std::mem::replace(&mut self.mod_written, false)
    }

    /// Follows the patch page's setting. A change takes effect on the shown object at once.
    pub(crate) fn set_parser_mode(&mut self, mode: ParserMode) {
        if self.parser_mode != mode {
            self.parser_mode = mode;
            self.pane.preview = None;
        }
    }

    /// The editor's settings, shared with the hkanno editor.
    fn read_editor_options(&mut self, editor: &LspOptions) {
        self.pane.mode = editor.editor_mode;
        self.pane.lsp.clone_from(editor);
    }

    /// Back to the settings: changed by the keymap box and the options window.
    fn write_editor_options(&mut self, editor: &mut LspOptions) {
        // Both hold the keymap: the box only changes `editor_mode`.
        self.pane.lsp.editor_mode = self.pane.mode;
        if *editor != self.pane.lsp {
            editor.clone_from(&self.pane.lsp);
        }
    }

    /// What is shown of the editor, kept in the settings.
    const fn read_view_settings(&mut self, settings: &PatchEditorSettings) {
        self.pane.show_inline_diff = settings.inline_diff;
        self.pane.follow_preview = settings.follow_preview;
        self.pane.show_preview = settings.show_preview;
        self.explorer.tree_view = settings.tree_view;
    }

    const fn write_view_settings(&self, settings: &mut PatchEditorSettings) {
        settings.inline_diff = self.pane.show_inline_diff;
        settings.show_preview = self.pane.show_preview;
        settings.follow_preview = self.pane.follow_preview;
        settings.tree_view = self.explorer.tree_view;
    }

    pub(crate) fn ui(
        &mut self,
        ui: &mut egui::Ui,
        i18n: &I18nMap,
        notify: &mut Notification,
        paths: PatchPaths<'_>,
        (settings, editor): (&mut PatchEditorSettings, &mut LspOptions),
        keys: &mut keybinding::Keybindings,
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
            self.explorer.trees =
                settings.tree_open.iter().map(|(path, open)| (path.clone(), tree::TreeState::restored(open))).collect();
        }
        self.read_view_settings(settings);
        self.explorer.changed_only = settings.changed_only;
        if self.poll_job(notify, i18n) {
            ui.ctx().request_repaint();
        }
        self.sync_workspace(notify, i18n);
        self.diagnose_work();
        // The user patch window, above the page, has the keys while it is open.
        let keys = (!self.user_patch.open).then_some(keys);
        self.take_key_commands(ui.ctx(), keys, i18n, notify);
        self.draw(ui, i18n, notify);

        self.write_editor_options(editor);
        if settings.mod_dir != self.mod_dir {
            settings.mod_dir.clone_from(&self.mod_dir);
        }
        if settings.recent != self.recent {
            settings.recent.clone_from(&self.recent);
        }
        self.write_view_settings(settings);
        settings.changed_only = self.explorer.changed_only;
        for (path, tree) in &mut self.explorer.trees {
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
        // The work is on another thread: only how far it is.
        if let Some(job) = &self.job {
            ui.add_space(12.0);
            ui.label(format!("{}: {}", i18n.t(job.label), job.detail));
            let done = job.done.load(std::sync::atomic::Ordering::Relaxed);
            let (ratio, text) = match job.progress() {
                Some(ratio) => (ratio, format!("{done} / {}", job.total)),
                None => (0.0, done.to_string()),
            };
            ui.add(egui::ProgressBar::new(ratio).text(text).animate(true));
            return;
        }
        self.code_choice_window(ui.ctx(), i18n, notify);
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
        self.repeat_window(ui.ctx(), i18n);
        self.active = self.active.min(self.workspace.behaviors.len().saturating_sub(1));

        // Like VS Code: the activity bar, the side bar, and the editor with what belongs to it.
        self.side_bar(ui, i18n, notify);
        if self.workspace.behaviors.is_empty() {
            egui::CentralPanel::default().show(ui, |ui| ui.weak(i18n.t(I18nKey::NemesisNoBehavior)));
            return;
        }
        self.class_picker_window(ui.ctx(), i18n);
        self.object_editor(ui, i18n);
    }

    // ── Workspace on disk ────────────────────────────────────────────────────
}

#[cfg(test)]
mod tests {
    use std::path::Path;

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
        let mut names = time!("Names::new", editor::names::Names::new(&behavior));
        time!("set_text", behavior.set_text(&id, &edited));
        let patch = time!("patch", behavior.patch("m", &id)).unwrap();
        let check = time!("validate", behavior.validate(&id, &patch, ParserMode::Strict));
        println!("  valid: {}", check.is_ok());
        time!("field_diagnostics", editor::diagnostics::field_diagnostics(&edited, &behavior, false, "m"));
        time!("Preview::new", editor::preview::Preview::new(&patch));
        time!("inline_diff", egui_code_editor::InlineDiff::between(&text, &edited));
        time!("apply_changes", names.apply_changes(&behavior, std::slice::from_ref(&id)));

        // A typo in a field name, and fixing it.
        let broken = text.replacen("name=\"", "name=\"x", 2);
        behavior.set_text(&id, &broken);
        let bad_patch = time!("broken: patch", behavior.patch("m", &id)).unwrap();
        let bad = time!("broken: validate", behavior.validate(&id, &bad_patch, ParserMode::Strict));
        println!("  valid: {}, error of {} bytes", bad.is_ok(), bad.as_ref().err().map_or(0, String::len));
        time!("broken: diagnostics", editor::diagnostics::field_diagnostics(&broken, &behavior, false, "m"));
        time!("broken: names", names.apply_changes(&behavior, std::slice::from_ref(&id)));
        behavior.set_text(&id, &edited);
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
        let explorer = tool::list::Explorer { tree_view: true, ..Default::default() };
        let mut tool = PatchTool { workspace, mod_code: code, explorer, ..PatchTool::default() };
        let i18n = I18nMap::new();

        for active in 0..tool.workspace.behaviors.len() {
            tool.active = active;
            let behavior = &tool.workspace.behaviors[active];
            let ids: Vec<String> = behavior.edits.keys().chain(behavior.added.keys()).cloned().collect();
            for id in ids {
                println!("{active} {id}");
                tool.selected = Some(id.as_str().into());
                egui::__run_test_ui(|ui| {
                    tool.object_list(ui, &i18n, &mut egui_widgets::notify::Notification::default());
                    tool.object_editor(ui, &i18n);
                });
                if tool.workspace.behaviors[active].is_anim_data() {
                    continue; // listed per actor, not in the tree
                }
                let names = tool.names();
                let path = &tool.workspace.behaviors[active].meshes_path;
                let tree = &tool.explorer.trees[path];
                let found = tree.rows().get(tree.cursor).map(|row| names.graph.id(row.node));
                assert_eq!(found, Some(id.as_str()), "`{id}` is not selected in the tree");
                assert_eq!(tool.pane.opened.as_ref().map(|(_, open)| open.as_str()), Some(id.as_str()));
                assert!(!tool.pane.text.is_empty(), "`{id}` opened empty");
            }
        }
    }
}

#[cfg(test)]
mod work_tests {
    use std::path::Path;

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

        tool.create_work("aaaa".to_owned(), "D:/mods/a".to_owned(), workspace::TemplateScope::Nemesis);
        tool.sync_workspace(&mut notify, &i18n);
        tool.wait_job(&mut notify, &i18n);
        assert_eq!(tool.loaded_for.as_deref(), Some("aaaa"));
        tool.workspace.info.name = "first".to_owned();
        tool.mark_dirty();
        tool.save_now(&mut notify, &i18n);

        // `File > New`: the first one goes away, and the second one starts empty.
        tool.close_work(&mut notify, &i18n);
        assert_eq!(tool.loaded_for, None);
        tool.create_work("bbbb".to_owned(), "D:/mods/a".to_owned(), workspace::TemplateScope::Nemesis);
        tool.sync_workspace(&mut notify, &i18n);
        tool.wait_job(&mut notify, &i18n);
        assert_eq!(tool.loaded_for.as_deref(), Some("bbbb"));
        assert_eq!(tool.workspace.info.name, "");
        tool.save_now(&mut notify, &i18n);

        let root = workspace::cache_root_of(&dir.display().to_string());
        let both = (Workspace::exists(&root, "aaaa"), Workspace::exists(&root, "bbbb"));

        // Back to the first one.
        tool.switch_work("aaaa", "D:/mods/a", "", &mut notify, &i18n);
        tool.sync_workspace(&mut notify, &i18n);
        tool.wait_job(&mut notify, &i18n);
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
        tool.create_work("aaaa".to_owned(), "D:/mods/a".to_owned(), workspace::TemplateScope::Nemesis);
        tool.sync_workspace(&mut notify, &i18n);
        tool.wait_job(&mut notify, &i18n);

        tool.set_patch_output(&dir.join("two").display().to_string(), &mut notify, &i18n);
        assert_eq!(tool.loaded_for, None);
        let in_one = Workspace::exists(&workspace::cache_root_of(&dir.join("one").display().to_string()), "aaaa");
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(in_one);
    }

    /// A mod manager's mod with a patch directory for each of `codes`.
    fn mod_with_codes(root: &Path, codes: &[&str]) {
        for code in codes {
            std::fs::create_dir_all(root.join("Nemesis_Engine/mod").join(code)).unwrap();
        }
    }

    #[test]
    fn a_mod_with_several_codes_asks_which_one() {
        let dir = std::env::temp_dir().join(format!("d_merge_code_choice_{}", std::process::id()));
        let root = dir.join("Elden Counter");
        mod_with_codes(&root, &["ecnpc", "ecdmg", "nemesis", "ab-cd"]);
        let (mut notify, i18n) = (Notification::default(), I18nMap::new());
        let mut tool = tool_in(&dir.join("out"));

        tool.open_mod(&root, &mut notify, &i18n);

        tool.wait_job(&mut notify, &i18n);
        let opened_at_once = tool.loaded_for.clone();
        let choice = tool
            .code_choice
            .as_ref()
            .map(|choice| choice.codes.iter().map(|code| code.code.clone()).collect::<Vec<_>>());

        tool.open_chosen_code("ecnpc", &mut notify, &i18n);
        tool.wait_job(&mut notify, &i18n);
        let opened = tool.loaded_for.clone();
        let asked_again = tool.code_choice.is_some();
        std::fs::remove_dir_all(&dir).unwrap();

        assert_eq!(opened_at_once, None);
        assert_eq!(choice.unwrap(), ["ecdmg", "ecnpc", "nemesis"], "what can't be opened isn't offered");
        assert_eq!(opened.as_deref(), Some("ecnpc"));
        assert!(!asked_again);
    }

    #[test]
    fn a_mod_with_one_code_or_a_patch_directory_opens_at_once() {
        let dir = std::env::temp_dir().join(format!("d_merge_code_direct_{}", std::process::id()));
        let (one, several) = (dir.join("one"), dir.join("several"));
        mod_with_codes(&one, &["para"]);
        mod_with_codes(&several, &["nemesis", "tudm"]);
        let (mut notify, i18n) = (Notification::default(), I18nMap::new());
        let mut tool = tool_in(&dir.join("out"));

        tool.open_mod(&one, &mut notify, &i18n);

        tool.wait_job(&mut notify, &i18n);
        let of_one = (tool.loaded_for.clone(), tool.code_choice.is_some());

        // The code of 7 letters, as Nemesis' own mod has it.
        tool.open_mod(&several.join("Nemesis_Engine/mod/nemesis"), &mut notify, &i18n);
        tool.wait_job(&mut notify, &i18n);
        let of_patch_dir = (tool.loaded_for.clone(), tool.code_choice.is_some());
        std::fs::remove_dir_all(&dir).unwrap();

        assert_eq!(of_one, (Some("para".to_owned()), false));
        assert_eq!(of_patch_dir, (Some("nemesis".to_owned()), false));
    }

    /// A work kept from when a Pandora mod was read without being applied: only its file is in the cache.
    /// Opening it goes on with that work, so that is where the edits have to come from.
    #[test]
    fn a_pandora_mod_shows_its_changes_also_from_an_old_work() {
        let dir = std::env::temp_dir().join(format!("d_merge_pandora_cache_{}", std::process::id()));
        let meshes = dir.join("templates/meshes/actors/character/behaviors");
        std::fs::create_dir_all(&meshes).unwrap();
        std::fs::write(
            meshes.join("0_master.xml"),
            "<hkpackfile><hksection name=\"__data__\">\n<hkobject name=\"#0001\" class=\"hkbStateMachine\" signature=\"0x1\">\n\t<hkparam name=\"enable\">true</hkparam>\n</hkobject>\n</hksection></hkpackfile>\n",
        )
        .unwrap();
        let root = dir.join("mods/Pandora");
        let mod_dir = root.join("Pandora_Engine/mod/pandora");
        std::fs::create_dir_all(mod_dir.join("patches")).unwrap();
        std::fs::write(
            mod_dir.join("patches/0_master.xml"),
            "<patch>\n\t<Replace path=\"#0001/enable\">\n\t\t<hkparam name=\"enable\">false</hkparam>\n\t</Replace>\n</patch>\n",
        )
        .unwrap();

        let (mut notify, i18n) = (Notification::default(), I18nMap::new());
        let open = |notify: &mut Notification| {
            let mut tool = tool_in(&dir.join("out"));
            tool.template_dir = dir.join("templates").display().to_string();
            tool.open_mod(&mod_dir, notify, &i18n);
            tool.wait_job(notify, &i18n);
            tool.sync_workspace(notify, &i18n);
            tool.wait_job(notify, &i18n);
            tool
        };

        let mut first = open(&mut notify);
        assert!(first.workspace.behaviors[0].edits["#0001"].contains(">false<"), "read from the mod");
        assert_eq!(first.workspace.export_format, workspace::ExportFormat::PandoraV5);
        first.mark_dirty();
        first.save_now(&mut notify, &i18n);

        // What an old build left: the file, no edits, and no format.
        let cache = first.cache_root.join("pandora");
        let behavior_dir =
            std::fs::read_dir(&cache).unwrap().flatten().map(|entry| entry.path()).find(|path| path.is_dir()).unwrap();
        for file in std::fs::read_dir(&behavior_dir).unwrap().flatten() {
            let name = file.file_name().to_string_lossy().into_owned();
            if name.starts_with('#') {
                std::fs::remove_file(file.path()).unwrap();
            }
        }
        let json = cache.join("workspace.json");
        let text = std::fs::read_to_string(&json).unwrap();
        std::fs::write(&json, text.replace("\"export_format\":\"PandoraV5\",", "")).unwrap();

        let second = open(&mut notify);
        let edits = second.workspace.behaviors.first().map(|behavior| behavior.edits.len());
        let edited = second.workspace.behaviors[0].edits.get("#0001").cloned();
        let format = second.workspace.export_format;
        let changed = {
            let graph = tree::Graph::new(&second.workspace.behaviors[0]);
            graph.node_of("#0001").is_some_and(|node| graph.is_changed(node))
        };
        std::fs::remove_dir_all(&dir).unwrap();

        assert_eq!(edits, Some(1));
        assert!(edited.unwrap().contains(">false<"));
        assert!(changed, "the list marks it");
        assert_eq!(format, workspace::ExportFormat::PandoraV5);
    }

    /// Vim `]c`: in the text while the inline diff shows, else in the preview.
    #[test]
    fn the_change_keys_go_to_what_shows_the_changes() {
        let pane = tool::editing::EditorPane {
            inline_diff: egui_code_editor::InlineDiff::between("a\nb\nc\nd\n", "a\nx\nc\ny\n"),
            preview_view: Some(editor::preview::Preview::new(
                "<!-- MOD_CODE ~m~ OPEN -->\nb\n<!-- CLOSE -->\n<!-- MOD_CODE ~m~ OPEN -->\nd\n<!-- CLOSE -->\n",
            )),
            show_inline_diff: true,
            show_preview: true,
            ..Default::default()
        };
        let mut tool = PatchTool { pane, ..PatchTool::default() };
        let current = |tool: &PatchTool| tool.pane.preview_view.as_ref().map(|view| view.current);

        tool.step_change(true);
        assert!(matches!(tool.pane.focus, Some(FocusAt::Line(1))));
        tool.pane.cursor_line = 1;
        tool.step_change(true);
        assert!(matches!(tool.pane.focus, Some(FocusAt::Line(3))));
        tool.pane.cursor_line = 3;
        tool.step_change(false);
        assert!(matches!(tool.pane.focus, Some(FocusAt::Line(1))));
        assert_eq!(current(&tool), Some(0), "the preview stays");

        tool.pane.focus = None;
        tool.pane.show_inline_diff = false;
        tool.step_change(true);
        assert_eq!(current(&tool), Some(1));
        assert!(tool.pane.focus.is_none(), "the cursor stays");

        tool.pane.show_preview = false;
        tool.step_change(true);
        assert_eq!(current(&tool), Some(1), "neither shows");
    }

    /// The preview says what `Export` would write, and its `↶` takes back what it shows.
    #[test]
    fn the_preview_follows_the_export_format() {
        let dir = std::env::temp_dir().join(format!("d_merge_pandora_preview_{}", std::process::id()));
        let meshes = dir.join("templates/meshes/actors/character/behaviors");
        std::fs::create_dir_all(&meshes).unwrap();
        std::fs::write(
            meshes.join("0_master.xml"),
            "<hkpackfile><hksection name=\"__data__\">\n<hkobject name=\"#0001\" class=\"hkbStateMachine\" signature=\"0x1\">\n\t<hkparam name=\"enable\">true</hkparam>\n</hkobject>\n</hksection></hkpackfile>\n",
        )
        .unwrap();
        let mod_dir = dir.join("mods/Pandora/Pandora_Engine/mod/pandora");
        std::fs::create_dir_all(mod_dir.join("patches")).unwrap();
        std::fs::write(
            mod_dir.join("patches/0_master.xml"),
            "<patch>\n\t<Replace path=\"#0001/enable\">\n\t\t<hkparam name=\"enable\">false</hkparam>\n\t</Replace>\n</patch>\n",
        )
        .unwrap();
        let (mut notify, i18n) = (Notification::default(), I18nMap::new());
        let mut tool = tool_in(&dir.join("out"));
        tool.template_dir = dir.join("templates").display().to_string();
        tool.open_mod(&mod_dir, &mut notify, &i18n);
        tool.wait_job(&mut notify, &i18n);
        tool.sync_workspace(&mut notify, &i18n);
        tool.wait_job(&mut notify, &i18n);
        tool.pane.text = tool.workspace.behaviors[0].text("#0001").unwrap().to_owned();

        let shown = |tool: &mut PatchTool| {
            tool.pane.preview = None;
            tool.refresh_preview("#0001");
            let lines = tool.pane.preview_view.as_ref().map(|view| view.lines().iter().map(|(_, line)| line.clone()));
            lines.map_or_else(String::new, |lines| lines.collect::<Vec<_>>().join("\n"))
        };
        let as_pandora = shown(&mut tool);
        tool.workspace.export_format = workspace::ExportFormat::Nemesis;
        let as_nemesis = shown(&mut tool);
        tool.workspace.export_format = workspace::ExportFormat::PandoraV5;
        shown(&mut tool);
        tool.revert_block("#0001", 0);
        let edits = tool.workspace.behaviors[0].edits.len();
        std::fs::remove_dir_all(&dir).unwrap();

        assert!(as_pandora.starts_with("<Replace path=\"#0001/enable\">"), "{as_pandora}");
        assert!(as_nemesis.contains("<!-- MOD_CODE ~"), "{as_nemesis}");
        assert_eq!(edits, 0, "back to the template");
    }

    /// `LspOptions` holds the keymap too: it must not put back the one it was given.
    #[test]
    fn the_keymap_box_changes_the_keymap() {
        let mut tool = PatchTool::default();
        let mut editor =
            LspOptions { editor_mode: gui_shared::settings::ui::editor::EditorMode::Normal, ..LspOptions::default() };
        tool.read_editor_options(&editor);
        tool.pane.mode = gui_shared::settings::ui::editor::EditorMode::Vim;
        tool.write_editor_options(&mut editor);
        assert_eq!(editor.editor_mode, gui_shared::settings::ui::editor::EditorMode::Vim);

        tool.read_editor_options(&editor);
        tool.pane.lsp.hover = !tool.pane.lsp.hover;
        tool.write_editor_options(&mut editor);
        assert_eq!(editor.hover, tool.pane.lsp.hover);
        assert_eq!(editor.editor_mode, gui_shared::settings::ui::editor::EditorMode::Vim);
    }
}
