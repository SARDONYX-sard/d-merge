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
//!
//! [`PatchTool`] only puts its parts together. Each part works on what it is given, and returns
//! what it asks of the others; this file hands that over:
//!
//! ```text
//! work      Work         what is made: open / save / export     tool::work, tool::work_state
//! sel       Selection    what is shown                          tool::selection
//! keys      Keys         this frame's keys                      tool::keys
//! side      SideArea     the side bar          -> SideAsk       ui::side_bar
//! editor    EditorArea   tabs, editor, diffs   -> EditorOut     tool::editing, ui::object_editor
//! problems  Problems     the work's problems                    problems
//! panels    Panels       the windows           -> WindowDone    ui::panels, ui::windows
//! bars      (functions)  top bar, start screen -> BarAsk        ui::top_bar
//! ```

mod anim;
mod changes;
mod editor;
mod editor_area;
mod new_work;
mod problems;
mod repeat;
mod template_tree;
mod tool;
mod tree;
mod ui;
mod user_patch;
mod vars_edit;
mod workspace;

use std::path::PathBuf;

use egui_code_editor::EditorCommand;
use egui_widgets::{Modal, notify::Notification};
use gui_shared::{
    i18n::{I18nKey, I18nMap},
    patch::ResolveRequest,
    settings::{
        pages::{
            patch::behavior::ParserMode,
            patch_editor::{PatchEditorSettings, RecentWork},
        },
        ui::editor::LspOptions,
    },
};

use self::{
    tool::session::Reshow,
    ui::{side_bar::SideAsk, top_bar::BarDone, windows::WindowDone},
    workspace::Workspace,
};

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

/// See the module docs.
#[derive(Debug, Default)]
pub(crate) struct PatchTool {
    /// What is being made, where it lives, when it is saved. See [`tool::work_state`].
    work: tool::work_state::Work,
    /// What is shown: the behavior and the object. See [`tool::selection`].
    sel: tool::selection::Selection,
    /// This frame's keys, and the palette's. See [`tool::keys`].
    keys: tool::keys::Keys,
    /// The side bar and its views. See [`ui::side_bar::SideArea`].
    side: ui::side_bar::SideArea,
    /// The tabs, the editor of the open object, the diffs. See [`editor_area::EditorArea`].
    editor: editor_area::EditorArea,
    /// The problems of the whole work: the status bar's counts, the Problems panel. See [`problems`].
    problems: problems::Problems,
    /// Which windows, forms and sections are open. See [`ui::panels`].
    panels: ui::panels::Panels,
    /// The window of the user patch, with its own tool. See [`user_patch`].
    user_patch: UserPatchView,
}

/// The user patch's window. See [`user_patch`].
#[derive(Debug, Default)]
struct UserPatchView {
    open: bool,
    /// Comes above what it was opened from, on the next frame.
    to_top: bool,
    /// Its editor, from the first time it is needed.
    tool: Option<Box<PatchTool>>,
}

/// Where the cursor goes when the editor gets the keyboard back.
#[derive(Debug, Clone, Copy)]
enum FocusAt {
    /// Where it was in that object (e.g., going back with `Ctrl+O`).
    Keep,
    /// A 0-based line (e.g., go to definition).
    Line(usize),
}

/// What the editor asked for in a frame. See [`editor_area::EditorArea::apply_editor_result`].
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
    /// An object's tab, by its id.
    Object(egui_widgets::TabAction, String),
    /// A diff's / blame's tab, by its index in [`editor_area::revision_view::Views`].
    View(egui_widgets::TabAction, usize),
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
        std::mem::replace(&mut self.work.mod_written, false)
    }

    /// Follows the patch page's setting. A change takes effect on the shown object at once.
    pub(crate) fn set_parser_mode(&mut self, mode: ParserMode) {
        if self.work.parser_mode != mode {
            self.work.parser_mode = mode;
            self.editor.preview_again();
        }
    }

    /// One frame of the page.
    pub(crate) fn ui(
        &mut self,
        ui: &mut egui::Ui,
        i18n: &I18nMap,
        notify: &mut Notification,
        paths: PatchPaths<'_>,
        (settings, editor): (&mut PatchEditorSettings, &mut LspOptions),
        keys: &mut keybinding::Keybindings,
    ) {
        self.editor.read_options(editor);
        if self.work.place.template_dir != paths.template_dir {
            paths.template_dir.clone_into(&mut self.work.place.template_dir);
        }
        self.set_patch_output(paths.patch_output_dir, notify, i18n);
        // The first frame reopens the last work (its cache is found from these two).
        if !self.work.restored {
            self.restore_settings(settings, notify, i18n);
        }
        self.read_view_settings(settings);
        self.side.explorer.changed_only = settings.changed_only;

        self.poll_job(ui.ctx(), notify, i18n);
        let shown = self.editor.shown(&self.sel);
        self.work.sync_workspace(shown, notify, i18n);
        self.problems.diagnose(&self.work);
        // The user patch window, above the page, has the keys while it is open.
        let keys = (!self.user_patch.open).then_some(keys);
        self.take_key_commands(ui.ctx(), keys, i18n, notify);
        self.draw(ui, i18n, notify);

        self.editor.write_options(editor);
        self.write_settings(settings);
    }

    /// The first frame: the work, the recent works and the trees as the settings kept them.
    fn restore_settings(&mut self, settings: &PatchEditorSettings, notify: &mut Notification, i18n: &I18nMap) {
        self.work.restored = true;
        let place = &mut self.work.place;
        place.output.clone_from(&settings.output);
        place.mod_dir.clone_from(&settings.mod_dir);
        place.mod_code.clone_from(&settings.mod_code);
        self.work.recent.clone_from(&settings.recent);
        // It has a window of its own.
        self.work.recent.retain(|work| !work.mod_code.eq_ignore_ascii_case(gui_shared::patch::USER_PATCH_CODE));

        // A work that is gone: the start screen rather than an empty work.
        let place = &self.work.place;
        let code_is_free = place.mod_code.trim().eq_ignore_ascii_case(gui_shared::patch::USER_PATCH_CODE);
        if place.patch_output.is_empty() || code_is_free || !Workspace::exists(&place.cache_root, place.mod_code.trim())
        {
            let shown = self.editor.shown(&self.sel);
            let closed = self.work.close_work(shown, notify, i18n);
            self.reshow(closed);
        }

        // What is set once stays out of the way, unless it still has to be set.
        self.panels.project_open = false;
        self.side.bar.behaviors_open = true;
        self.side.explorer.trees = settings
            .tree_open
            .iter()
            .map(|(path, open)| {
                let open = open.iter().map(|(id, open)| (id.as_str(), *open));
                (path.clone(), tree::TreeState::restored(open))
            })
            .collect();
    }

    /// What the page keeps in the settings: the work, the recent works, what is shown.
    fn write_settings(&mut self, settings: &mut PatchEditorSettings) {
        let place = &self.work.place;
        if settings.mod_dir != place.mod_dir {
            settings.mod_dir.clone_from(&place.mod_dir);
        }
        if settings.recent != self.work.recent {
            settings.recent.clone_from(&self.work.recent);
        }
        self.write_view_settings(settings);
        settings.changed_only = self.side.explorer.changed_only;
        for (path, tree) in &mut self.side.explorer.trees {
            if let Some(open) = tree.take_open_changes() {
                let open = open.map(|(id, open)| (id.to_owned(), open)).collect();
                settings.tree_open.insert(path.clone(), open);
            }
        }
        if settings.output != place.output || settings.mod_code != place.mod_code {
            settings.output.clone_from(&place.output);
            settings.mod_code.clone_from(&place.mod_code);
        }
    }

    /// What is shown of the editor, kept in the settings.
    const fn read_view_settings(&mut self, settings: &PatchEditorSettings) {
        self.editor.read_view(settings);
        self.side.explorer.tree_view = settings.tree_view;
    }

    const fn write_view_settings(&self, settings: &mut PatchEditorSettings) {
        self.editor.write_view(settings);
        settings.tree_view = self.side.explorer.tree_view;
    }

    /// The patch page's output directory may have changed: see [`tool::work_state::Work::set_patch_output`].
    fn set_patch_output(&mut self, output: &str, notify: &mut Notification, i18n: &I18nMap) {
        let shown = self.editor.shown(&self.sel);
        if let Some(closed) = self.work.set_patch_output(shown, output, notify, i18n) {
            self.panels.project_open = false;
            self.reshow(closed);
        }
    }

    /// Takes in the job once it is done; while it runs, the page is drawn again and again.
    fn poll_job(&mut self, ctx: &egui::Context, notify: &mut Notification, i18n: &I18nMap) {
        let shown = self.editor.shown(&self.sel);
        if let Some(reshow) = self.work.poll_job(shown, notify, i18n) {
            self.reshow(reshow);
        }
        if self.work.job.is_some() {
            ctx.request_repaint();
        }
    }

    /// Waits for the job, if one runs: see [`tool::work_state::Work::wait_job`].
    fn wait_job(&mut self, notify: &mut Notification, i18n: &I18nMap) {
        let shown = self.editor.shown(&self.sel);
        if let Some(reshow) = self.work.wait_job(shown, notify, i18n) {
            self.reshow(reshow);
        }
    }

    /// The work changed as a whole: the editor shows `reshow`. A mod read closes the form of a new
    /// work.
    fn reshow(&mut self, reshow: Reshow) {
        if matches!(reshow, Reshow::Place(_)) {
            self.panels.new_work = None;
        }
        self.editor.reshow(reshow, &mut self.work, &mut self.sel);
    }

    /// Reads this frame's keys: see [`tool::keys::Keys::frame`].
    fn take_key_commands(
        &mut self,
        ctx: &egui::Context,
        bindings: Option<&mut keybinding::Keybindings>,
        i18n: &I18nMap,
        notify: &mut Notification,
    ) {
        let parts = (&self.editor, &self.side.explorer, &self.problems);
        let context = tool::list::key_context(ctx, parts, self.work.user_patch);
        self.keys.frame(ctx, bindings, &context, (i18n, notify));
    }

    /// The bar, the side bar and the editor; or the start screen, or how far a job is.
    fn draw(&mut self, ui: &mut egui::Ui, i18n: &I18nMap, notify: &mut Notification) {
        // The work is on another thread: only how far it is.
        if let Some(job) = &self.work.job {
            ui::job_progress(ui, i18n, job);
            return;
        }

        let windows = self.panels.windows(ui.ctx(), i18n, (&mut self.work, &mut self.sel), notify);
        for done in windows {
            self.apply_window(done, notify, i18n);
        }

        let user_patch = self.work.user_patch;
        let start = !user_patch && !self.work.place.is_open();
        let parts = (&mut self.work, &mut self.panels);
        let asks = if user_patch {
            ui::user_patch::user_patch_bar(ui, i18n, (&*parts.0, parts.1), notify)
        } else if start {
            ui::start_screen::start_screen(ui, i18n, (&*parts.0, parts.1), notify)
        } else {
            ui::top_bar::project_bar(ui, i18n, parts, notify)
        };
        for ask in asks {
            self.apply_bar(ask, notify, i18n);
        }
        if start {
            return;
        }
        ui.separator();
        self.sel.active = self.sel.active.min(self.work.workspace.behaviors.len().saturating_sub(1));

        // Like VS Code: the activity bar, the side bar, and the editor with what belongs to it.
        let parts = (&mut self.work, &mut self.sel, &mut self.keys);
        let asks = self.side.show(ui, i18n, parts, &mut self.panels, notify);
        for ask in asks {
            self.apply_side(ask, ui.ctx(), notify);
        }
        if self.work.workspace.behaviors.is_empty() {
            egui::CentralPanel::default().show(ui, |ui| ui.weak(i18n.t(I18nKey::NemesisNoBehavior)));
            return;
        }

        let parts = (&mut self.work, &mut self.sel, &mut self.keys);
        let out = self.editor.show(ui, i18n, parts, &mut self.problems);
        self.apply_editor_out(ui, out, notify);
    }

    /// What a window did: see [`WindowDone`].
    fn apply_window(&mut self, done: WindowDone, notify: &mut Notification, i18n: &I18nMap) {
        match done {
            WindowDone::Reload => self.editor.reload(),
            WindowDone::Duplicated(to) => self.side.explorer.anim_tree.show_project(to),
            WindowDone::OpenCode(choice, code) => {
                let shown = self.editor.shown(&self.sel);
                self.work.open_chosen_code(shown, (&choice, &code), notify, i18n);
            }
        }
    }

    /// What a bar asked for: see [`ui::top_bar::apply`].
    fn apply_bar(&mut self, ask: ui::top_bar::BarAsk, notify: &mut Notification, i18n: &I18nMap) {
        let shown = self.editor.shown(&self.sel);
        match ui::top_bar::apply(ask, (&mut self.work, &mut self.panels), shown, notify, i18n) {
            Some(BarDone::Reshow(reshow)) => self.reshow(reshow),
            Some(BarDone::Reload) => self.editor.reload(),
            None => {}
        }
    }

    /// What the side bar asked for: see [`SideAsk`].
    fn apply_side(&mut self, ask: SideAsk, ctx: &egui::Context, notify: &mut Notification) {
        let (editor, sel) = (&mut self.editor, &mut self.sel);
        match ask {
            SideAsk::ShowBehavior(at) => editor.show_behavior(sel, at),
            SideAsk::BehaviorRemoved => editor.behaviors_changed(),
            SideAsk::KeepOpen(id) => editor.keep_open(sel, &id),
            SideAsk::OpenLocation(id, line) => {
                let current = sel.selected.clone().unwrap_or_default();
                editor.open_location(sel, &current, (&id, line));
            }

            SideAsk::OpenChange(behavior, id) => editor.open_object(sel, (behavior, &id)),
            SideAsk::Compare(behavior, id, from, to) => {
                editor.open_compare((&mut self.work, sel), (behavior, &id), (from, to));
            }
            SideAsk::Blame(behavior, id) => editor.open_blame((&mut self.work, sel), (behavior, &id)),

            SideAsk::RevertOrDelete(id) => {
                let gone = self.work.revert_or_delete(sel.active, &id);
                self.problems.refresh_in(&self.work, sel.active, &id);
                editor.object_reverted(sel, &id, gone);
            }
            SideAsk::OpenPandoraDoc => {
                self.work.make_pandora_doc(sel.active);
                editor.open_pandora_doc(sel);
            }
            SideAsk::File(action, id) => tool::list::file_action(ctx, (&self.work, sel.active), (action, &id), notify),

            SideAsk::FocusEditor => editor.focus_editor(),
            SideAsk::Reload => editor.reload(),
        }
    }

    /// What the editor asked for: see [`editor_area::EditorOut`].
    fn apply_editor_out(&mut self, ui: &egui::Ui, out: editor_area::EditorOut, notify: &mut Notification) {
        if out.reveal {
            self.side.explorer.reveal = true;
        }
        if out.leave_tree {
            self.side.explorer.tree_focus = false;
        }
        if let Some(editor_id) = out.enter_tree {
            self.side.enter_tree(ui, editor_id);
        }
        if let Some((action, id)) = out.file {
            tool::list::file_action(ui.ctx(), (&self.work, self.sel.active), (action, &id), notify);
        }
    }

    // ── The user patch ───────────────────────────────────────────────────────

    /// The user patch's tool, as it starts: see [`user_patch::work_of`].
    fn for_user_patch(paths: PatchPaths<'_>) -> Self {
        let mut tool = Self { work: user_patch::work_of(paths), ..Self::default() };
        tool.side.bar.behaviors_open = true;
        tool.side.explorer.changed_only = true;
        tool
    }

    /// Opens or closes the window of the user patch.
    pub(crate) const fn toggle_user_patch(&mut self) {
        self.user_patch.open = !self.user_patch.open;
        self.user_patch.to_top = self.user_patch.open;
    }

    /// The user patch's editor, up to date. A new output directory has another user patch: what was
    /// open is saved where it belongs first.
    fn user_patch_tool(&mut self, paths: PatchPaths<'_>, notify: &mut Notification, i18n: &I18nMap) -> &mut Self {
        let output = paths.patch_output_dir.trim();
        if let Some(tool) = self.user_patch.tool.as_mut().filter(|tool| tool.work.place.patch_output != output) {
            tool.leave_user_patch(notify, i18n);
            self.work.mod_written |= tool.work.mod_written;
            self.user_patch.tool = None;
        }

        let tool = self.user_patch.tool.get_or_insert_with(|| Box::new(Self::for_user_patch(paths)));
        tool.set_parser_mode(paths.parser_mode);
        if tool.work.place.template_dir != paths.template_dir {
            paths.template_dir.clone_into(&mut tool.work.place.template_dir);
        }
        let shown = tool.editor.shown(&tool.sel);
        tool.work.sync_workspace(shown, notify, i18n);
        // Small, and what comes next uses it at once.
        tool.wait_job(notify, i18n);
        tool
    }

    /// `Use this`: the value goes in now, and the mod is written. The window need not be open.
    ///
    /// # Errors
    /// There is no output directory to keep it in, or the value could not be taken in.
    pub(crate) fn resolve_now(
        &mut self,
        request: &ResolveRequest,
        paths: PatchPaths<'_>,
        notify: &mut Notification,
        i18n: &I18nMap,
    ) -> Result<(), String> {
        if paths.patch_output_dir.trim().is_empty() {
            return Err(i18n.t(I18nKey::NemesisNeedPatchOutput).to_owned());
        }
        let root = PathBuf::from(paths.template_dir);
        let tool = self.user_patch_tool(paths, notify, i18n);
        let invalid = tool.work.resolve_now(request, &root, i18n)?;
        tool.editor.reload();

        self.work.mod_written = true;
        if invalid.is_empty() { Ok(()) } else { Err(invalid.join("\n")) }
    }

    /// `Edit...`: the value goes in, and the window shows it.
    pub(crate) fn open_resolution(
        &mut self,
        request: ResolveRequest,
        paths: PatchPaths<'_>,
        notify: &mut Notification,
        i18n: &I18nMap,
    ) {
        self.user_patch.open = true;
        self.user_patch.to_top = true;
        if paths.patch_output_dir.trim().is_empty() {
            return; // the window says what to do
        }
        self.user_patch_tool(paths, notify, i18n).work.resolution = Some(request);
    }

    /// The value `Edit...` asked for, taken in: see [`user_patch::take_in_resolution`].
    fn take_resolution(&mut self, notify: &mut Notification, i18n: &I18nMap) {
        let Some(request) = self.work.resolution.take() else { return };
        user_patch::take_in_resolution(&request, (&mut self.work, &mut self.editor, &mut self.sel), notify, i18n);
    }

    /// The user patch's tool is left (closed, or another one): what `Edit...` proposed goes unless
    /// edited, and the rest is saved.
    fn leave_user_patch(&mut self, notify: &mut Notification, i18n: &I18nMap) {
        user_patch::undo_unedited_draft((&mut self.work, &mut self.editor, &mut self.sel));
        let shown = self.editor.shown(&self.sel);
        self.work.save_now(shown, notify, i18n);
    }

    /// The window was closed: what it wrote is for the conflict check to see now, not at each pause.
    fn close_user_patch(&mut self, notify: &mut Notification, i18n: &I18nMap) {
        let Some(tool) = &mut self.user_patch.tool else { return };
        tool.leave_user_patch(notify, i18n);
        self.work.mod_written |= std::mem::take(&mut tool.work.mod_written);
    }

    /// Draws the user patch's window if it is open. It belongs to no page: it stays when another is
    /// shown. See [`ui::user_patch`].
    pub(crate) fn user_patch_window(
        &mut self,
        ctx: &egui::Context,
        i18n: &I18nMap,
        notify: &mut Notification,
        paths: PatchPaths<'_>,
        (settings, editor): (&mut PatchEditorSettings, &mut LspOptions),
        keys: &mut keybinding::Keybindings,
    ) {
        if !self.user_patch.open {
            return;
        }
        let to_top = std::mem::take(&mut self.user_patch.to_top);
        let has_output = !paths.patch_output_dir.trim().is_empty();
        let mut tool = has_output.then(|| self.user_patch_tool(paths, notify, i18n));

        let mut open = true;
        Modal::new(i18n.t(I18nKey::UserPatchTitle))
            .size(egui::Vec2::new(0.90, 0.95))
            .to_top(to_top)
            // Its editor uses `Esc` (Vim, hovers, quick fixes): the window closes with its `x`.
            .close_on_escape(false)
            .show(ctx, &mut open, |ui| match &mut tool {
                Some(tool) => tool.user_patch_ui(ui, i18n, notify, (settings, editor), keys),
                None => {
                    ui.colored_label(ui.visuals().warn_fg_color, i18n.t(I18nKey::NemesisNeedPatchOutput));
                }
            });

        self.user_patch.open = open;
        if !open {
            self.close_user_patch(notify, i18n);
        }
    }

    /// One frame of the user patch's own tool, which keeps nothing of its work in the settings.
    fn user_patch_ui(
        &mut self,
        ui: &mut egui::Ui,
        i18n: &I18nMap,
        notify: &mut Notification,
        (settings, editor): (&mut PatchEditorSettings, &mut LspOptions),
        keys: &mut keybinding::Keybindings,
    ) {
        self.editor.read_options(editor);
        self.read_view_settings(settings);
        let shown = self.editor.shown(&self.sel);
        self.work.sync_workspace(shown, notify, i18n);
        self.take_resolution(notify, i18n);
        self.take_key_commands(ui.ctx(), Some(keys), i18n, notify);
        self.draw(ui, i18n, notify);
        self.editor.write_options(editor);
        self.write_view_settings(settings);
    }
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
        let mut behavior = workspace::Behavior::open(&path).unwrap();
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
            state.prepare(&tree::ObjectTree { graph: &names.graph, changed_only: false }, Some(&id));
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
        let mut tool = PatchTool::default();
        tool.side.explorer.tree_view = true;
        tool.work.place.loaded_for = Some(code.clone());
        (tool.work.workspace, tool.work.place.mod_code) = (workspace, code);
        let i18n = I18nMap::new();

        for active in 0..tool.work.workspace.behaviors.len() {
            tool.sel.active = active;
            let behavior = &tool.work.workspace.behaviors[active];
            let ids: Vec<String> = behavior.edits.keys().chain(behavior.added.keys()).cloned().collect();
            for id in ids {
                println!("{active} {id}");
                tool.sel.selected = Some(id.as_str().into());
                egui::__run_test_ui(|ui| tool.draw(ui, &i18n, &mut Notification::default()));
                if tool.work.workspace.behaviors[active].is_anim_data() {
                    continue; // listed per actor, not in the tree
                }
                let names = tool.work.names(active);
                let path = &tool.work.workspace.behaviors[active].meshes_path;
                let tree = &tool.side.explorer.trees[path];
                let found = tree.rows().get(tree.cursor).map(|row| names.graph.id(row.node));
                assert_eq!(found, Some(id.as_str()), "`{id}` is not selected in the tree");
                let (opened, text) = tool.editor.opened().unwrap_or_default();
                assert_eq!(opened, id.as_str());
                assert!(!text.is_empty(), "`{id}` opened empty");
            }
        }
    }
}

#[cfg(test)]
mod work_tests {
    use std::path::Path;

    use super::*;

    fn tool_in(dir: &Path) -> PatchTool {
        let mut tool = PatchTool::default();
        tool.work.restored = true;
        tool.set_patch_output(&dir.display().to_string(), &mut Notification::default(), &I18nMap::new());
        tool
    }

    /// Loads the work of the code typed, as a frame does, and waits for it.
    fn sync(tool: &mut PatchTool, notify: &mut Notification, i18n: &I18nMap) {
        let shown = tool.editor.shown(&tool.sel);
        tool.work.sync_workspace(shown, notify, i18n);
        tool.wait_job(notify, i18n);
    }

    fn save(tool: &mut PatchTool, notify: &mut Notification, i18n: &I18nMap) {
        let shown = tool.editor.shown(&tool.sel);
        tool.work.save_now(shown, notify, i18n);
    }

    /// Opens the mod at `dir`: of several codes, the one to open is asked for.
    fn open_mod(tool: &mut PatchTool, dir: &Path, notify: &mut Notification, i18n: &I18nMap) {
        let shown = tool.editor.shown(&tool.sel);
        tool.panels.code_choice = tool.work.open_mod(shown, dir, notify, i18n);
        tool.wait_job(notify, i18n);
    }

    /// A work is kept in the patch page's output, and another one can be made next to it.
    #[test]
    fn a_new_work_can_be_made_beside_another() {
        let dir = std::env::temp_dir().join(format!("d_merge_new_work_{}", std::process::id()));
        let (mut notify, i18n) = (Notification::default(), I18nMap::new());
        let mut tool = tool_in(&dir);

        tool.work.create_work("aaaa".to_owned(), "D:/mods/a".to_owned(), workspace::TemplateScope::Nemesis);
        sync(&mut tool, &mut notify, &i18n);
        assert_eq!(tool.work.place.loaded_for.as_deref(), Some("aaaa"));
        tool.work.workspace.info.name = "first".to_owned();
        tool.work.mark_dirty();
        save(&mut tool, &mut notify, &i18n);

        // `File > New`: the first one goes away, and the second one starts empty.
        tool.apply_bar(ui::top_bar::BarAsk::CloseWork, &mut notify, &i18n);
        assert_eq!(tool.work.place.loaded_for, None);
        tool.work.create_work("bbbb".to_owned(), "D:/mods/a".to_owned(), workspace::TemplateScope::Nemesis);
        sync(&mut tool, &mut notify, &i18n);
        assert_eq!(tool.work.place.loaded_for.as_deref(), Some("bbbb"));
        assert_eq!(tool.work.workspace.info.name, "");
        save(&mut tool, &mut notify, &i18n);

        let root = workspace::cache_root_of(&dir.display().to_string());
        let both = (Workspace::exists(&root, "aaaa"), Workspace::exists(&root, "bbbb"));

        // Back to the first one.
        let shown = tool.editor.shown(&tool.sel);
        tool.work.switch_work(shown, ("aaaa", "D:/mods/a", ""), &mut notify, &i18n);
        sync(&mut tool, &mut notify, &i18n);
        let name = tool.work.workspace.info.name.clone();
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
        tool.work.create_work("aaaa".to_owned(), "D:/mods/a".to_owned(), workspace::TemplateScope::Nemesis);
        sync(&mut tool, &mut notify, &i18n);

        tool.set_patch_output(&dir.join("two").display().to_string(), &mut notify, &i18n);
        assert_eq!(tool.work.place.loaded_for, None);
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

        open_mod(&mut tool, &root, &mut notify, &i18n);
        let opened_at_once = tool.work.place.loaded_for.clone();
        let choice = tool
            .panels
            .code_choice
            .as_ref()
            .map(|choice| choice.codes.iter().map(|code| code.code.clone()).collect::<Vec<_>>());

        let asked = tool.panels.code_choice.take().unwrap();
        let shown = tool.editor.shown(&tool.sel);
        tool.work.open_chosen_code(shown, (&asked, "ecnpc"), &mut notify, &i18n);
        tool.wait_job(&mut notify, &i18n);
        let opened = tool.work.place.loaded_for.clone();
        let asked_again = tool.panels.code_choice.is_some();
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

        open_mod(&mut tool, &one, &mut notify, &i18n);
        let of_one = (tool.work.place.loaded_for.clone(), tool.panels.code_choice.is_some());

        // The code of 7 letters, as Nemesis' own mod has it.
        open_mod(&mut tool, &several.join("Nemesis_Engine/mod/nemesis"), &mut notify, &i18n);
        let of_patch_dir = (tool.work.place.loaded_for.clone(), tool.panels.code_choice.is_some());
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
            tool.work.place.template_dir = dir.join("templates").display().to_string();
            open_mod(&mut tool, &mod_dir, notify, &i18n);
            sync(&mut tool, notify, &i18n);
            tool
        };

        let mut first = open(&mut notify);
        assert!(first.work.workspace.behaviors[0].edits["#0001"].contains(">false<"), "read from the mod");
        assert_eq!(first.work.workspace.export_format, workspace::ExportFormat::PandoraV5);
        first.work.mark_dirty();
        save(&mut first, &mut notify, &i18n);

        // What an old build left: the file, no edits, and no format.
        let cache = first.work.place.cache_root.join("pandora");
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
        let edits = second.work.workspace.behaviors.first().map(|behavior| behavior.edits.len());
        let edited = second.work.workspace.behaviors[0].edits.get("#0001").cloned();
        let format = second.work.workspace.export_format;
        let changed = {
            let graph = tree::Graph::new(&second.work.workspace.behaviors[0]);
            graph.node_of("#0001").is_some_and(|node| graph.is_changed(node))
        };
        std::fs::remove_dir_all(&dir).unwrap();

        assert_eq!(edits, Some(1));
        assert!(edited.unwrap().contains(">false<"));
        assert!(changed, "the list marks it");
        assert_eq!(format, workspace::ExportFormat::PandoraV5);
    }
}

#[cfg(test)]
mod user_patch_tests {
    use std::path::Path;

    use gui_shared::patch::USER_PATCH_CODE;

    use super::*;

    fn paths(output: &str) -> PatchPaths<'_> {
        PatchPaths { patch_output_dir: output, template_dir: "", parser_mode: ParserMode::Strict }
    }

    #[test]
    fn it_needs_the_output_directory() {
        let (mut notify, i18n) = (Notification::default(), I18nMap::new());
        let request = ResolveRequest {
            target: "meshes/x.bin".into(),
            path: "#1/C/a".into(),
            mod_dir: "D:/mods/a".into(),
            edit: false,
            sides: Vec::new(),
            keep_all_events: false,
        };
        let mut main = PatchTool::default();
        assert!(main.resolve_now(&request, paths(""), &mut notify, &i18n).is_err());
        assert!(main.user_patch.tool.is_none());
    }

    /// A proposal of `Edit...`: as a conflict's `Edit...` does, with a mod's clip of animation data.
    fn proposed(dir: &Path) -> (PatchTool, Notification, I18nMap) {
        let file = dir.join("mods/a/animationdatasinglefile/D~1/Idle~m$0.txt");
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, "theirs\n").unwrap();
        let output = dir.join("out").display().to_string();
        let (mut notify, i18n) = (Notification::default(), I18nMap::new());
        let mut main = PatchTool::default();
        let request = ResolveRequest {
            target: "meshes/animationdatasinglefile.bin".into(),
            path: "D~1/Idle~m$0/name".into(),
            mod_dir: dir.join("mods/a").display().to_string(),
            edit: true,
            sides: Vec::new(),
            keep_all_events: false,
        };
        main.open_resolution(request, paths(&output), &mut notify, &i18n);
        main.user_patch.tool.as_mut().unwrap().take_resolution(&mut notify, &i18n);
        (main, notify, i18n)
    }

    /// Opening it to look and closing it decides nothing.
    #[test]
    fn an_edit_that_was_not_made_changes_nothing() {
        let dir = std::env::temp_dir().join(format!("d_merge_draft_none_{}", std::process::id()));
        let (mut main, mut notify, i18n) = proposed(&dir);
        let tool = main.user_patch.tool.as_mut().unwrap();
        assert_eq!(tool.work.workspace.behaviors[0].text("D~1/Idle~m$0.txt"), Some("theirs\n"), "it shows the value");
        assert!(!tool.work.autosave.is_dirty(), "nothing to save yet");

        main.close_user_patch(&mut notify, &i18n);
        let tool = main.user_patch.tool.as_ref().unwrap();
        let (behaviors, refs, name) = (
            tool.work.workspace.behaviors.len(),
            tool.work.workspace.ref_mods.len(),
            tool.work.workspace.info.name.clone(),
        );
        let written = dir.join("out").exists();
        let wrote = main.take_mod_written();
        std::fs::remove_dir_all(&dir).unwrap();

        assert_eq!(behaviors, 0, "the behavior was opened for it");
        assert_eq!(refs, 0);
        assert_eq!(name, "");
        assert!(!written, "nothing is on the disk");
        assert!(!wrote, "the conflict check has nothing to follow");
    }

    #[test]
    fn an_edit_that_was_made_stays() {
        let dir = std::env::temp_dir().join(format!("d_merge_draft_edit_{}", std::process::id()));
        let (mut main, mut notify, i18n) = proposed(&dir);
        let tool = main.user_patch.tool.as_mut().unwrap();
        tool.work.workspace.behaviors[0].set_text("D~1/Idle~m$0.txt", "mine\n");
        tool.work.mark_dirty();

        main.close_user_patch(&mut notify, &i18n);
        let tool = main.user_patch.tool.as_ref().unwrap();
        let text = tool.work.workspace.behaviors[0].text("D~1/Idle~m$0.txt").map(str::to_owned);
        let wrote = main.take_mod_written();
        let exported =
            dir.join("out/.d_merge/user_patch/Nemesis_Engine/mod/upatch/animationdatasinglefile/D~1/Idle~m$0.txt");
        let on_disk = std::fs::read_to_string(exported).ok();
        std::fs::remove_dir_all(&dir).unwrap();

        assert_eq!(text.as_deref(), Some("mine\n"));
        assert!(wrote);
        assert_eq!(on_disk.as_deref(), Some("mine\n"));
    }

    /// Its window draws, on any page, and the main tool does not take its code.
    #[test]
    fn the_window_draws_and_clears() {
        let dir = std::env::temp_dir().join(format!("d_merge_user_patch_{}", std::process::id()));
        let output = dir.display().to_string();
        let (mut notify, i18n) = (Notification::default(), I18nMap::new());
        let mut main = PatchTool::default();
        let (mut settings, mut editor) = (PatchEditorSettings::default(), LspOptions::default());
        main.toggle_user_patch();

        let ctx = egui::Context::default();
        for _ in 0..3 {
            let mut drawn = ctx.run_ui(egui::RawInput::default(), |ui| {
                let ctx = ui.ctx().clone();
                let mut keys = keybinding::Keybindings::default();
                main.user_patch_window(
                    &ctx,
                    &i18n,
                    &mut notify,
                    paths(&output),
                    (&mut settings, &mut editor),
                    &mut keys,
                );
            });
            drawn.textures_delta.clear();
        }
        let tool = main.user_patch.tool.as_mut().unwrap();
        assert_eq!(tool.work.place.loaded_for.as_deref(), Some(USER_PATCH_CODE));

        tool.apply_bar(ui::top_bar::BarAsk::ClearUserPatch, &mut notify, &i18n);
        let cache = workspace::cache_root_of(&output).join(USER_PATCH_CODE);
        let kept = cache.is_dir();
        main.close_user_patch(&mut notify, &i18n);
        let written = main.take_mod_written();
        std::fs::remove_dir_all(&dir).unwrap();

        assert!(kept);
        assert!(written, "the conflict check follows");
    }
}
