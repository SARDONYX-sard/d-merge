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
//! Work is cached under the output dir and saved a moment after each edit (see [`workspace`]).

mod diagnostics;
mod language;
mod names;
mod navigate;
mod preview;
mod template_tree;
mod tree;
mod ui;
mod vars_edit;
mod workspace;

use std::{path::Path, rc::Rc, time::Instant};

use egui_code_editor::{CodeEditorState, Diagnostic, EditorCommand};
use egui_widgets::notify::Notification;
use gui_shared::{
    i18n::{I18nKey, I18nMap},
    settings::{
        pages::patch_editor::PatchEditorSettings,
        ui::editor::{EditorMode, LspOptions},
    },
};
use nemesis_diff::is_valid_mod_code;

use self::workspace::{Behavior, TemplateEntry, Workspace};

/// How long after an edit the workspace is written to its cache.
const SAVE_DELAY: std::time::Duration = std::time::Duration::from_secs(1);

#[derive(Debug, Default)]
pub(crate) struct PatchTool {
    output: String,
    mod_code: String,
    /// `settings.behavior.template_dir` (`.bin` or `.xml` templates under `meshes/`).
    template_dir: String,
    /// Templates found in `template_dir`, for the picker.
    catalog: Vec<TemplateEntry>,
    catalog_for: Option<String>,
    picker_open: bool,
    picker_filter: String,
    /// `output` / `code` were taken from the settings.
    restored: bool,
    workspace: Workspace,
    /// `(output, code)` the workspace belongs to (`None` = not on disk yet).
    loaded_for: Option<(String, String)>,
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
    /// Preview of `editor_for`: (patch, parser result). Recomputed when the text changes.
    preview: Option<(Option<String>, Result<(), String>)>,
    /// Unknown class / field names in the editor text (underlined).
    field_problems: Vec<Diagnostic>,
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
    tree: tree::TreeState,

    /// Left panel shows events / variables instead of objects.
    show_vars: bool,
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
    /// `template_dir`: `settings.behavior.template_dir` (absolute or relative).
    pub(crate) fn ui(
        &mut self,
        ui: &mut egui::Ui,
        i18n: &I18nMap,
        notify: &mut Notification,
        template_dir: &str,
        settings: &mut PatchEditorSettings,
        editor: &mut LspOptions,
    ) {
        self.editor_mode = editor.editor_mode;
        self.lsp.clone_from(editor);
        if self.template_dir != template_dir {
            template_dir.clone_into(&mut self.template_dir);
        }
        // The first frame reopens the last work (its cache is found from these two).
        if !self.restored {
            self.restored = true;
            self.output.clone_from(&settings.output);
            self.mod_code.clone_from(&settings.mod_code);
        }
        self.sync_workspace(notify, i18n);
        self.draw(ui, i18n, notify);

        // Changed by the editor's keymap box / options window.
        editor.editor_mode = self.editor_mode;
        if *editor != self.lsp {
            editor.clone_from(&self.lsp);
        }
        if settings.output != self.output || settings.mod_code != self.mod_code {
            settings.output.clone_from(&self.output);
            settings.mod_code.clone_from(&self.mod_code);
        }
    }

    fn draw(&mut self, ui: &mut egui::Ui, i18n: &I18nMap, notify: &mut Notification) {
        self.settings_ui(ui, i18n, notify);
        if self.picker_open {
            self.template_picker(ui, i18n, notify);
        }
        self.behavior_tabs(ui, i18n);
        ui.separator();

        if self.workspace.behaviors.is_empty() {
            ui.weak(i18n.t(I18nKey::NemesisNoBehavior));
            return;
        }
        self.active = self.active.min(self.workspace.behaviors.len() - 1);
        self.target_row(ui, i18n);

        egui::Panel::left("nemesis_objects")
            .resizable(true)
            .default_size(300.0)
            .show(ui, |ui| self.object_list(ui, i18n));
        self.object_editor(ui, i18n);
    }

    // ── Workspace on disk ────────────────────────────────────────────────────

    /// Loads the cache of `(output, code)` when they change, and saves pending edits.
    fn sync_workspace(&mut self, notify: &mut Notification, i18n: &I18nMap) {
        let key = (self.output.trim().to_string(), self.mod_code.trim().to_string());
        let usable = !key.0.is_empty() && is_valid_mod_code(&key.1);

        if usable && self.loaded_for.as_ref() != Some(&key) {
            self.save_now(notify, i18n); // the previous workspace
            if self.loaded_for.is_none() && !self.workspace.behaviors.is_empty() {
                // Work done before the output was set: keep it and write it there.
                self.dirty_since = Some(Instant::now());
            } else {
                match Workspace::load(Path::new(&key.0), &key.1) {
                    Ok(workspace) => {
                        self.workspace = workspace;
                        self.reset_names();
                    }
                    Err(err) => {
                        notify.error(format!("{}: {err}", i18n.t(I18nKey::NemesisLoadFailed)));
                    }
                }
                self.editor_for = None;
                self.selected = None;
            }
            self.loaded_for = Some(key);
        }

        if self.dirty_since.is_some_and(|since| since.elapsed() >= SAVE_DELAY) {
            self.save_now(notify, i18n);
        }
    }

    fn save_now(&mut self, notify: &mut Notification, i18n: &I18nMap) {
        let Some((output, code)) = &self.loaded_for else { return };
        if self.dirty_since.take().is_none() {
            return;
        }
        if let Err(err) = self.workspace.save(Path::new(output), code) {
            notify.error(format!("{}: {err}", i18n.t(I18nKey::NemesisSaveFailed)));
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

    /// Loads `id` into the editor if it is not the open object already.
    fn open_in_editor(&mut self, id: &str) {
        if self.editor_for.as_ref().is_some_and(|(active, open)| *active == self.active && open == id) {
            return;
        }
        let text = self.workspace.behaviors[self.active].text(id).unwrap_or_default();
        text.clone_into(&mut self.editor_text);
        self.editor = CodeEditorState::default();
        self.preview = None;
        self.editor_for = Some((self.active, id.to_owned()));
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

    /// Recomputes the patch, its check and the underlines after the text changed.
    fn refresh_preview(&mut self, id: &str) {
        if self.preview.is_some() {
            return;
        }
        let behavior = &self.workspace.behaviors[self.active];
        let patch = behavior.patch(self.mod_code.trim(), id);
        let check = patch.as_deref().map_or(Ok(()), |patch| behavior.validate(patch));
        self.field_problems =
            diagnostics::field_diagnostics(&self.editor_text, behavior, behavior.added.contains_key(id));
        // Keep the ▲ / ▼ position across edits.
        let current = self.preview_view.as_ref().map_or(0, |view| view.current);
        self.preview_view = patch.as_deref().map(preview::Preview::new);
        if let Some(view) = &mut self.preview_view {
            view.keep_current(current);
        }
        self.preview = Some((patch, check));
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

    /// Does what the editor asked for. Returns whether the keyboard goes to the object tree.
    fn apply_editor_result(&mut self, result: EditorResult, id: &str) -> bool {
        if result.changed {
            self.workspace.behaviors[self.active].set_text(id, self.editor_text.clone());
            self.preview = None;
            self.object_changed(id);
        }
        match result.command {
            // Vim `:w`: save the work now.
            Some(EditorCommand::Save) => self.dirty_since = Instant::now().checked_sub(SAVE_DELAY),
            Some(EditorCommand::JumpBack) => self.walk_history(true, id),
            Some(EditorCommand::JumpForward) => self.walk_history(false, id),
            Some(EditorCommand::ToggleExplorer | EditorCommand::GoToDefinition | EditorCommand::FindReferences)
            | None => {}
        }
        if let Some((document, line)) = result.definition {
            self.open_location(id, &document, line);
        }
        result.focus_tree || result.command == Some(EditorCommand::ToggleExplorer)
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
                self.active = self.workspace.behaviors.len() - 1;
                self.mark_dirty();
            }
            Err(err) => notify.error(format!("{}: {err}", i18n.t(I18nKey::NemesisLoadFailed))),
        }
    }

    fn import(&mut self, dir: &Path, notify: &mut Notification, i18n: &I18nMap) {
        match Workspace::import(dir, Path::new(&self.template_dir)) {
            Ok((workspace, code, skipped)) => {
                self.save_now(notify, i18n);
                self.workspace = workspace;
                self.reset_names();
                self.mod_code = code;
                // Write it to `(output, code)` instead of loading what may be there.
                self.loaded_for = (!self.output.trim().is_empty() && is_valid_mod_code(&self.mod_code))
                    .then(|| (self.output.trim().to_string(), self.mod_code.clone()));
                self.mark_dirty();
                self.active = 0;
                self.selected = None;
                self.editor_for = None;
                if !skipped.is_empty() {
                    notify.warn(format!("{}: {}", i18n.t(I18nKey::NemesisNoTemplateFor), skipped.join(", ")));
                }
            }
            Err(err) => notify.error(format!("{}: {err}", i18n.t(I18nKey::NemesisLoadFailed))),
        }
    }

    fn export(&mut self, notify: &mut Notification, i18n: &I18nMap) {
        self.save_now(notify, i18n);
        let Some((output, code)) = &self.loaded_for else { return };
        match self.workspace.export(Path::new(output), code) {
            Ok((written, invalid)) if invalid.is_empty() => notify.success(format!(
                "{} ({written}): {}",
                i18n.t(I18nKey::NemesisExported),
                Path::new(output).join("Nemesis_Engine").join("mod").join(code).display()
            )),
            Ok((_, invalid)) => notify.warn(format!(
                "{} ({}):\n{}",
                i18n.t(I18nKey::NemesisRejected),
                invalid.len(),
                invalid.join("\n")
            )),
            Err(err) => notify.error(format!("{}: {err}", i18n.t(I18nKey::NemesisExportFailed))),
        }
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
        for input in ui::tree::read_keys(ui, &mut self.tree.leader) {
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
        self.list = Some(ObjectList { generation, filter: self.filter.clone(), changed_only: self.changed_only, rows });
    }

    /// Deletes a new object, or drops the edits of a template object.
    fn revert_or_delete(&mut self, id: &str) {
        let behavior = &mut self.workspace.behaviors[self.active];
        behavior.edits.remove(id);
        if behavior.added.remove(id).is_some() && self.selected.as_deref() == Some(id) {
            self.selected = None;
        }
        self.editor_for = None;
        self.object_changed(id);
    }

    fn new_object(&mut self, from: Option<&str>) {
        let code = self.mod_code.trim().to_string();
        let id = self.workspace.behaviors[self.active].add_object(&code, from);
        self.object_changed(&id);
        self.selected = Some(id);
    }
}
