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

mod language;
mod navigate;
mod preview;
mod tree;
mod vars_panel;
mod workspace;

use std::{path::Path, time::Instant};

use egui_code_editor::{
    CodeEditor, CodeEditorState, Diagnostic, EditorCommand, EditorFeatures, Keymap, Severity,
};
use egui_widgets::{
    notify::Notification,
    shadcn::{Button, Checkbox, TextEdit},
};
use gui_shared::{
    i18n::{I18nKey, I18nMap},
    settings::{
        hkanno::{EditorMode, HkannoLspOptions, HkannoSettings},
        patch_editor::PatchEditorSettings,
    },
};
use nemesis_diff::is_valid_mod_code;

use self::{
    language::NemesisXml,
    workspace::{Behavior, Target, TemplateEntry, Workspace},
};

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
    /// `Shift+F12`: who points to an object.
    references: navigate::References,
    history: navigate::History,
    /// The `#id` the editor's right-click menu was opened on.
    menu_target: Option<String>,

    /// Normal / Vim keys of the object editor.
    editor_mode: EditorMode,
    /// Language features (shared with hkanno's options), and their window is open.
    lsp: HkannoLspOptions,
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
    names: Option<(usize, vars_panel::Names)>,
    new_names: vars_panel::NewNames,
    vars_error: Option<String>,
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
        editor: &mut HkannoSettings,
    ) {
        self.editor_mode = editor.editor_mode;
        self.lsp.clone_from(&editor.lsp);
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
        if editor.lsp != self.lsp {
            editor.lsp.clone_from(&self.lsp);
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
                    Ok(workspace) => self.workspace = workspace,
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
                return;
            }
            current = target;
        }
    }

    fn mark_dirty(&mut self) {
        self.names = None; // events / variables may have changed
        self.dirty_since.get_or_insert_with(Instant::now);
    }

    // ── Top bar ──────────────────────────────────────────────────────────────

    fn settings_ui(&mut self, ui: &mut egui::Ui, i18n: &I18nMap, notify: &mut Notification) {
        ui.horizontal(|ui| {
            ui.label(i18n.t(I18nKey::NemesisOutputLabel))
                .on_hover_text(i18n.t(I18nKey::NemesisOutputHover));
            ui.add(TextEdit::singleline(&mut self.output).desired_width(260.0));
            if ui.add(Button::new(i18n.t(I18nKey::SelectButton))).clicked()
                && let Some(dir) = rfd::FileDialog::new().set_directory(&self.output).pick_folder()
            {
                self.output = dir.display().to_string();
            }

            ui.separator();
            ui.label(i18n.t(I18nKey::NemesisModCodeLabel));
            let code = ui.add(
                TextEdit::singleline(&mut self.mod_code)
                    .hint_text(i18n.t(I18nKey::NemesisModCodeHint))
                    .desired_width(70.0),
            );
            if !self.mod_code.is_empty() && !is_valid_mod_code(self.mod_code.trim()) {
                code.on_hover_text(i18n.t(I18nKey::NemesisModCodeInvalid));
                ui.colored_label(ui.visuals().error_fg_color, "✖");
            }

            let info = &mut self.workspace.info;
            let mut changed = false;
            for (label, value, width) in [
                (I18nKey::NemesisNameLabel, &mut info.name, 140.0),
                (I18nKey::NemesisAuthorLabel, &mut info.author, 90.0),
                (I18nKey::NemesisSiteLabel, &mut info.site, 140.0),
            ] {
                ui.label(i18n.t(label));
                changed |= ui.add(TextEdit::singleline(value).desired_width(width)).changed();
            }
            if changed {
                self.mark_dirty();
            }
        });

        ui.horizontal(|ui| {
            let label = format!("{} ▼", i18n.t(I18nKey::NemesisAddBehavior));
            if ui
                .add(Button::selectable(self.picker_open, label))
                .on_hover_text(format!(
                    "{} {}",
                    i18n.t(I18nKey::NemesisTemplatesLabel),
                    self.template_dir
                ))
                .clicked()
            {
                self.picker_open = !self.picker_open;
            }
            if ui
                .add(Button::new(i18n.t(I18nKey::NemesisCustomXml)))
                .on_hover_text(i18n.t(I18nKey::NemesisCustomXmlHover))
                .clicked()
                && let Some(path) = rfd::FileDialog::new()
                    .add_filter("Template", &["xml", "bin"])
                    .set_directory(&self.template_dir)
                    .pick_file()
            {
                self.add_behavior(&path, notify, i18n);
            }
            if ui
                .add(Button::new(i18n.t(I18nKey::NemesisImportMod)))
                .on_hover_text(i18n.t(I18nKey::NemesisImportModHover))
                .clicked()
                && let Some(dir) = rfd::FileDialog::new().pick_folder()
            {
                self.import(&dir, notify, i18n);
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let ready = self.loaded_for.is_some() && !self.workspace.behaviors.is_empty();
                if ui
                    .add_enabled(ready, Button::new(i18n.t(I18nKey::NemesisExport)).primary())
                    .on_disabled_hover_text(i18n.t(I18nKey::NemesisExportDisabledHover))
                    .clicked()
                {
                    self.export(notify, i18n);
                }
            });
        });
    }

    /// The templates of `template_dir` with a filter; a click adds one as a behavior.
    fn template_picker(&mut self, ui: &mut egui::Ui, i18n: &I18nMap, notify: &mut Notification) {
        if self.catalog_for.as_deref() != Some(self.template_dir.as_str()) {
            self.catalog = workspace::catalog(Path::new(&self.template_dir));
            self.catalog_for = Some(self.template_dir.clone());
        }
        let mut picked = None;
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add(
                    TextEdit::singleline(&mut self.picker_filter)
                        .hint_text(i18n.t(I18nKey::NemesisFilterHint))
                        .desired_width(240.0),
                );
                ui.weak(format!("{} ({})", self.template_dir, self.catalog.len()));
            });
            if self.catalog.is_empty() {
                ui.colored_label(ui.visuals().warn_fg_color, i18n.t(I18nKey::NemesisNoTemplates));
                return;
            }
            let filter = self.picker_filter.to_lowercase();
            let shown: Vec<&TemplateEntry> =
                self.catalog.iter().filter(|e| e.meshes_path.contains(&filter)).collect();
            let row_height = ui.text_style_height(&egui::TextStyle::Monospace) + 6.0;
            egui::ScrollArea::vertical().max_height(220.0).auto_shrink([false, true]).show_rows(
                ui,
                row_height,
                shown.len(),
                |ui, range| {
                    for entry in &shown[range] {
                        ui.horizontal(|ui| {
                            let text = egui::RichText::new(&entry.meshes_path).monospace();
                            if ui.add(egui::Button::selectable(false, text)).clicked() {
                                picked = Some(entry.path.clone());
                            }
                            if entry.nemesis {
                                ui.weak("Nemesis");
                            } else {
                                ui.colored_label(ui.visuals().warn_fg_color, "Ext")
                                    .on_hover_text(i18n.t(I18nKey::NemesisExtOnlyWarning));
                            }
                        });
                    }
                },
            );
        });
        if let Some(path) = picked {
            self.add_behavior(&path, notify, i18n);
            self.picker_open = false;
        }
    }

    fn add_behavior(&mut self, path: &Path, notify: &mut Notification, i18n: &I18nMap) {
        match Behavior::open(path) {
            Ok(behavior) => {
                if let Some(at) =
                    self.workspace.behaviors.iter().position(|b| b.name == behavior.name)
                {
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
                self.mod_code = code;
                // Write it to `(output, code)` instead of loading what may be there.
                self.loaded_for = (!self.output.trim().is_empty()
                    && is_valid_mod_code(&self.mod_code))
                .then(|| (self.output.trim().to_string(), self.mod_code.clone()));
                self.mark_dirty();
                self.active = 0;
                self.selected = None;
                self.editor_for = None;
                if !skipped.is_empty() {
                    notify.warn(format!(
                        "{}: {}",
                        i18n.t(I18nKey::NemesisNoTemplateFor),
                        skipped.join(", ")
                    ));
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

    /// `[draugrbehavior] [0_master] ...` (`*` = has changes)
    /// Right click a tab: remove that behavior (and its edits) from the work.
    fn behavior_tabs(&mut self, ui: &mut egui::Ui, i18n: &I18nMap) {
        let mut remove = None;
        ui.horizontal_wrapped(|ui| {
            for (i, behavior) in self.workspace.behaviors.iter().enumerate() {
                let count = behavior.edits.len() + behavior.added.len();
                let label = if count > 0 {
                    format!("{} ({count})", behavior.name)
                } else {
                    behavior.name.clone()
                };
                let response = ui
                    .add(Button::selectable(i == self.active, label))
                    .on_hover_text(behavior.template_path.display().to_string());
                if response.clicked() {
                    self.active = i;
                }
                response.context_menu(|ui| {
                    let button = Button::new(i18n.t(I18nKey::NemesisRemoveBehavior)).destructive();
                    if ui
                        .add(button)
                        .on_hover_text(i18n.t(I18nKey::NemesisRemoveBehaviorHover))
                        .clicked()
                    {
                        remove = Some(i);
                    }
                });
            }
        });
        if let Some(i) = remove {
            self.workspace.behaviors.remove(i);
            self.active = self.active.min(self.workspace.behaviors.len().saturating_sub(1));
            self.selected = None;
            self.editor_for = None;
            self.mark_dirty();
        }
    }

    /// `→ Nemesis_Engine/mod/mymod/0_master   ☐ NemesisExt` (+ a warning for NemesisExt)
    fn target_row(&mut self, ui: &mut egui::Ui, i18n: &I18nMap) {
        let code =
            if is_valid_mod_code(self.mod_code.trim()) { self.mod_code.trim() } else { "<code>" };
        let behavior = &mut self.workspace.behaviors[self.active];
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label(format!("→ {}", behavior.patch_dir(code).display()));
            if behavior.asdsf {
                return; // always `Nemesis_Engine/mod/<code>/animationsetdatasinglefile`
            }
            let supported = behavior.nemesis.is_some();
            let mut ext = behavior.force_ext || !supported;
            let hover = if supported {
                I18nKey::NemesisForceExtHover
            } else {
                I18nKey::NemesisUnsupportedTemplate
            };
            if ui
                .add_enabled(supported, Checkbox::new(&mut ext, i18n.t(I18nKey::NemesisForceExt)))
                .on_hover_text(i18n.t(hover))
                .on_disabled_hover_text(i18n.t(hover))
                .changed()
            {
                behavior.force_ext = ext;
                changed = true;
            }
            if behavior.target() == Target::Ext {
                ui.colored_label(
                    ui.visuals().warn_fg_color,
                    i18n.t(I18nKey::NemesisExtOnlyWarning),
                );
            }
        });
        if changed {
            self.mark_dirty();
        }
    }

    // ── Object list ──────────────────────────────────────────────────────────

    fn object_list(&mut self, ui: &mut egui::Ui, i18n: &I18nMap) {
        if !self.workspace.behaviors[self.active].asdsf {
            ui.horizontal(|ui| {
                for (vars, label) in
                    [(false, I18nKey::NemesisObjectsTab), (true, I18nKey::NemesisVarsTab)]
                {
                    if ui.add(Button::selectable(self.show_vars == vars, i18n.t(label))).clicked() {
                        self.show_vars = vars;
                    }
                }
            });
        }
        ui.add(
            TextEdit::singleline(&mut self.filter)
                .hint_text(i18n.t(I18nKey::NemesisFilterHint))
                .desired_width(ui.available_width()),
        );
        if self.show_vars && !self.workspace.behaviors[self.active].asdsf {
            self.vars_panel(ui, i18n);
            return;
        }
        ui.horizontal(|ui| {
            ui.add(Checkbox::new(&mut self.changed_only, i18n.t(I18nKey::NemesisChangedOnly)));
            if !self.workspace.behaviors[self.active].asdsf {
                ui.add(Checkbox::new(&mut self.tree_view, i18n.t(I18nKey::NemesisTreeView)))
                    .on_hover_text(i18n.t(I18nKey::NemesisTreeViewHover));
            }
        });

        ui.horizontal(|ui| {
            // Anim sets can only be edited, not added.
            let code_ok = is_valid_mod_code(self.mod_code.trim())
                && !self.workspace.behaviors[self.active].asdsf;
            if ui
                .add_enabled(code_ok, Button::new(i18n.t(I18nKey::NemesisNewObject)).small())
                .on_disabled_hover_text(i18n.t(I18nKey::NemesisModCodeInvalid))
                .clicked()
            {
                self.class_picker = !self.class_picker; // pick the class below
            }
            let selected = self.selected.clone();
            if ui
                .add_enabled(
                    code_ok && selected.is_some(),
                    Button::new(i18n.t(I18nKey::NemesisDuplicate)).small(),
                )
                .clicked()
            {
                self.new_object(selected.as_deref());
            }
            // A new object is deleted, a template object goes back to the template.
            let is_new = selected
                .as_ref()
                .is_some_and(|id| self.workspace.behaviors[self.active].added.contains_key(id));
            let (label, hover) = if is_new {
                (I18nKey::NemesisDeleteObject, I18nKey::NemesisDeleteObjectHover)
            } else {
                (I18nKey::NemesisRevert, I18nKey::NemesisRevertHover)
            };
            if ui
                .add_enabled(selected.is_some(), Button::new(i18n.t(label)).small().destructive())
                .on_hover_text(i18n.t(hover))
                .clicked()
                && let Some(id) = selected
            {
                self.revert_or_delete(&id);
            }
        });
        if self.class_picker {
            self.class_picker_ui(ui, i18n);
        }
        ui.separator();

        // Tree (from the root container) unless filtering: a filter wants the flat list.
        if self.tree_view && self.filter.is_empty() {
            let behavior = &self.workspace.behaviors[self.active];
            let changed_only = self.changed_only;
            egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| {
                tree::tree_ui(ui, behavior, &mut self.selected, &mut self.tree, changed_only);
            });
            return;
        }

        let _ = self.names(); // where new objects are linked (cached)
        let rows = self.rows();
        let mut menu_action = None;
        let row_height = ui.text_style_height(&egui::TextStyle::Monospace) + 4.0;
        egui::ScrollArea::vertical().auto_shrink(false).show_rows(
            ui,
            row_height,
            rows.len(),
            |ui, range| {
                for (marker, id, class, name) in &rows[range] {
                    let text =
                        egui::RichText::new(format!("{marker} {id}  {class}  {name}")).monospace();
                    let selected = self.selected.as_deref() == Some(id.as_str());
                    let response = ui.add(egui::Button::selectable(selected, text));
                    if response.clicked() {
                        self.selected = Some(id.clone());
                    }
                    let changed = *marker != ' ';
                    response.context_menu(|ui| {
                        if let Some(action) = row_menu(ui, i18n, *marker == '+', changed) {
                            menu_action = Some((id.clone(), action));
                        }
                    });
                }
            },
        );
        if let Some((id, action)) = menu_action {
            match action {
                RowAction::Duplicate => self.new_object(Some(&id)),
                RowAction::CopyId => ui.ctx().copy_text(id),
                RowAction::RevertOrDelete => self.revert_or_delete(&id),
            }
        }
    }

    /// `(marker, id, class, name)`: new objects first, then the template's (`●` = edited).
    fn rows(&self) -> Vec<(char, String, String, String)> {
        let behavior = &self.workspace.behaviors[self.active];
        let filter = self.filter.to_lowercase();
        let matches = |id: &str, class: &str, name: &str| {
            filter.is_empty()
                || id.to_lowercase().contains(&filter)
                || class.to_lowercase().contains(&filter)
                || name.to_lowercase().contains(&filter)
        };

        // A new object shows where it is linked: `← #0106` (or a warning).
        let parents = self.names.as_ref().map(|(_, names)| &names.parents);
        let added = behavior.added.iter().map(|(id, text)| {
            let class =
                text.split_once("class=\"").and_then(|(_, r)| r.split_once('"')).map(|(c, _)| c);
            let linked = match parents.and_then(|p| p.get(id)) {
                Some(from) => format!("← {}", from.join(", ")),
                None => "⚠ not linked".to_string(),
            };
            ('+', id.clone(), class.unwrap_or_default().to_string(), linked)
        });
        let template = behavior
            .objects
            .iter()
            .filter(|o| !self.changed_only || behavior.edits.contains_key(&o.id))
            .map(|o| {
                let marker = if behavior.edits.contains_key(&o.id) { '●' } else { ' ' };
                (marker, o.id.clone(), o.class.clone(), o.name.clone())
            });
        added.chain(template).filter(|(_, id, class, name)| matches(id, class, name)).collect()
    }

    /// Deletes a new object, or drops the edits of a template object.
    fn revert_or_delete(&mut self, id: &str) {
        let behavior = &mut self.workspace.behaviors[self.active];
        behavior.edits.remove(id);
        if behavior.added.remove(id).is_some() && self.selected.as_deref() == Some(id) {
            self.selected = None;
        }
        self.editor_for = None;
        self.mark_dirty();
    }

    /// `[class filter]` + the class table's classes (the template's own first); a click adds a
    /// new object of that class with every field at its default.
    fn class_picker_ui(&mut self, ui: &mut egui::Ui, i18n: &I18nMap) {
        ui.add(
            TextEdit::singleline(&mut self.class_filter)
                .hint_text(i18n.t(I18nKey::NemesisClassFilterHint))
                .desired_width(ui.available_width()),
        );
        let behavior = &self.workspace.behaviors[self.active];
        let filter = self.class_filter.to_lowercase();
        let used = |class: &str| behavior.object_of_class(class).is_some();
        let mut classes: Vec<&str> = nemesis_diff::schema::class_names()
            .into_iter()
            .filter(|c| c.to_lowercase().contains(&filter))
            .collect();
        classes.sort_by_key(|class| !used(class)); // stable: alphabetical within each group

        let mut picked = None;
        let row_height = ui.text_style_height(&egui::TextStyle::Monospace) + 4.0;
        egui::ScrollArea::vertical().id_salt("nemesis_class_picker").max_height(200.0).show_rows(
            ui,
            row_height,
            classes.len(),
            |ui, range| {
                for class in &classes[range] {
                    let text = egui::RichText::new(*class).monospace();
                    let text = if used(class) { text } else { text.weak() };
                    if ui.add(egui::Button::selectable(false, text)).clicked() {
                        picked = Some((*class).to_string());
                    }
                }
            },
        );
        if let Some(class) = picked {
            let code = self.mod_code.trim().to_string();
            if let Some(id) = self.workspace.behaviors[self.active].add_class(&code, &class) {
                self.selected = Some(id);
                self.class_picker = false;
                self.mark_dirty();
            }
        }
    }

    fn new_object(&mut self, from: Option<&str>) {
        let code = self.mod_code.trim().to_string();
        let id = self.workspace.behaviors[self.active].add_object(&code, from);
        self.selected = Some(id);
        self.mark_dirty();
    }

    // ── Editor + preview ─────────────────────────────────────────────────────

    fn object_editor(&mut self, ui: &mut egui::Ui, i18n: &I18nMap) {
        let Some(id) = self.selected.clone() else {
            egui::CentralPanel::default()
                .show(ui, |ui| ui.weak(i18n.t(I18nKey::NemesisSelectObject)));
            return;
        };
        let key = (self.active, id.clone());
        if self.editor_for.as_ref() != Some(&key) {
            let behavior = &self.workspace.behaviors[self.active];
            self.editor_text = behavior.text(&id).unwrap_or_default().to_string();
            self.editor = CodeEditorState::default();
            self.preview = None;
            self.editor_for = Some(key);
        }

        let code = self.mod_code.trim().to_string();
        if self.preview.is_none() {
            let patch = self.workspace.behaviors[self.active].patch(&code, &id);
            let behavior = &self.workspace.behaviors[self.active];
            let check = patch.as_deref().map_or(Ok(()), |patch| behavior.validate(patch));
            self.preview = Some((patch, check));
            self.field_problems =
                field_diagnostics(&self.editor_text, &self.workspace.behaviors[self.active]);
            let current = self.preview_view.as_ref().map_or(0, |v| v.current);
            self.preview_view = self
                .preview
                .as_ref()
                .and_then(|(patch, _)| patch.as_deref())
                .map(preview::Preview::new);
            // Keep the ▲ / ▼ position across edits.
            if let Some(view) = &mut self.preview_view {
                view.keep_current(current);
            }
        }

        // The patch preview (not for a new object: it is written as is).
        let mut revert = None;
        let is_new = self.workspace.behaviors[self.active].added.contains_key(&id);
        if !is_new {
            egui::Panel::bottom("nemesis_preview").resizable(true).default_size(260.0).show(
                ui,
                |ui| {
                    let Some((patch, check)) = &self.preview else { return };
                    ui.horizontal(|ui| {
                        ui.strong(i18n.t(I18nKey::NemesisPatchPreview));
                        if let Some(view) = &mut self.preview_view {
                            view.nav_ui(ui, i18n.t(I18nKey::NemesisJumpDiffHover));
                        }
                        match check {
                            _ if patch.is_none() => {
                                ui.weak(i18n.t(I18nKey::NemesisNoChange));
                            }
                            Ok(()) => {
                                ui.colored_label(egui::Color32::from_rgb(100, 200, 120), "✔")
                                    .on_hover_text(i18n.t(I18nKey::NemesisParserOk));
                            }
                            Err(err) => {
                                ui.colored_label(ui.visuals().error_fg_color, format!("✖ {err}"));
                            }
                        }
                    });
                    if let Some(view) = &mut self.preview_view {
                        revert = view.ui(ui, i18n.t(I18nKey::NemesisRevertHunkHover));
                    }
                },
            );
        }
        // `$eventID[` / `$variableID[` complete with the behavior's names.
        let names = self.names();
        let (events, variables, pointers) =
            (names.events.clone(), names.variables.clone(), names.pointers.clone());
        // Enum / flag values only depend on the template: computed once per behavior.
        if self.field_values.as_ref().is_none_or(|(active, _)| *active != self.active) {
            let values = language::field_values(&self.workspace.behaviors[self.active]);
            self.field_values = Some((self.active, std::rc::Rc::new(values)));
        }
        let values = self
            .field_values
            .as_ref()
            .map(|(_, values)| std::rc::Rc::clone(values))
            .unwrap_or_default();
        let values = values.as_ref();
        // The class (and fields) around the cursor line of last frame: field names / hover.
        let context = nemesis_diff::schema::field_context(&self.editor_text, self.cursor_line);
        let fields = context.as_ref().map(|(class, fields)| (class.as_str(), fields.as_slice()));
        let language = NemesisXml {
            events: &events,
            variables: &variables,
            values,
            pointers: &pointers,
            fields,
        };
        let features = EditorFeatures {
            completion: self.lsp.completion,
            hover: self.lsp.hover,
            diagnostics: self.lsp.diagnostics,
            format: false,
            inlay_hints: false,
            signature_help: self.lsp.signature_help,
        };
        let keymap = match self.editor_mode {
            EditorMode::Normal => Keymap::Normal,
            EditorMode::Vim => Keymap::Vim,
        };
        // `F12` / `Shift+F12` before the editor (its keymap would take them).
        let editor_id = egui_code_editor::editor_id(("nemesis_object", &id));
        let mut nav = navigate::key_request(ui, editor_id, &self.editor_text);
        egui::CentralPanel::default().show(ui, |ui| {
            // `Keys: [Vim ▼]`, and the Vim status line under the editor.
            ui.horizontal(|ui| {
                ui.label(i18n.t(I18nKey::HkannoEditorMode));
                let mut index =
                    EditorMode::ALL.iter().position(|m| *m == self.editor_mode).unwrap_or(0);
                if egui::ComboBox::from_id_salt("nemesis_editor_mode")
                    .width(80.0)
                    .show_index(ui, &mut index, EditorMode::ALL.len(), |i| {
                        EditorMode::ALL[i].label()
                    })
                    .changed()
                {
                    self.editor_mode = EditorMode::ALL[index];
                }
                // The same language feature switches as the hkanno editor's `⚙`.
                if ui
                    .add(Button::selectable(self.show_lsp_options, "⚙"))
                    .on_hover_text(i18n.t(I18nKey::HkannoLspOptionsTitle))
                    .clicked()
                {
                    self.show_lsp_options = !self.show_lsp_options;
                }
            });
            if self.show_lsp_options {
                lsp_options_window(ui.ctx(), i18n, &mut self.lsp, &mut self.show_lsp_options);
            }
            if self.editor.keymap.status(keymap).is_some() {
                egui::Panel::bottom("nemesis_keymap_status")
                    .show(ui, |ui| egui_code_editor::status_line(ui, &self.editor, keymap));
            }
            egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| {
                let output = CodeEditor::new(("nemesis_object", &id), &mut self.editor_text)
                    .language(&language)
                    .diagnostics(&self.field_problems)
                    .features(features)
                    .keymap(keymap)
                    .show(ui, &mut self.editor);
                if let Some(cursor) = output.cursor {
                    self.cursor_line = cursor.line;
                }
                // `Ctrl+click` / right-click menu on a `#id`.
                let labels = (
                    i18n.t(I18nKey::NemesisGoToDefinition),
                    i18n.t(I18nKey::NemesisFindReferences),
                );
                if let Some(request) = navigate::pointer_request(
                    &output,
                    &self.editor_text,
                    &mut self.menu_target,
                    labels,
                ) {
                    nav = Some(request);
                }
                if let Some(request) = navigate::command_request(&output, &self.editor_text) {
                    nav = Some(request);
                }
                // Vim `:w`: save the work now.
                if output.command == Some(EditorCommand::Save) {
                    self.dirty_since = Instant::now().checked_sub(SAVE_DELAY);
                }
                if output.changed {
                    self.workspace.behaviors[self.active].set_text(&id, self.editor_text.clone());
                    self.preview = None;
                    self.mark_dirty();
                }
            });
        });

        match nav {
            Some(navigate::Request::GoTo(target))
                if self.workspace.behaviors[self.active].text(&target).is_some() =>
            {
                self.history.visit(&id);
                self.selected = Some(target);
            }
            Some(navigate::Request::Back) => self.walk_history(true, &id),
            Some(navigate::Request::Forward) => self.walk_history(false, &id),
            Some(navigate::Request::References(target)) => {
                let list = navigate::references(&self.workspace.behaviors[self.active], &target);
                self.references = Some((target, list));
            }
            _ => {}
        }
        if let Some(target) = navigate::references_window(ui.ctx(), &mut self.references) {
            self.history.visit(&id);
            self.selected = Some(target);
        }

        let behavior = &mut self.workspace.behaviors[self.active];
        if let Some(index) = revert
            && let Some(template) = behavior.template_object(&id)
        {
            self.editor_text = nemesis_diff::revert_hunk(&template.text, &self.editor_text, index);
            behavior.set_text(&id, self.editor_text.clone());
            self.preview = None;
            self.mark_dirty();
        }
    }
}

/// [`nemesis_diff::check_fields`] (names) and [`nemesis_diff::schema::check_pointers`] (pointer
/// targets: they exist and have the right class) as editor underlines (the trimmed line).
fn field_diagnostics(text: &str, behavior: &workspace::Behavior) -> Vec<Diagnostic> {
    let lines: Vec<&str> = text.lines().collect();
    let class_of = |id: &str| behavior.class_of(id);
    let names = nemesis_diff::check_fields(text)
        .into_iter()
        .map(|p| (p.line, p.message, Severity::Warning));
    let pointers = nemesis_diff::schema::check_pointers(text, &class_of)
        .into_iter()
        .map(|p| (p.line, p.message, Severity::Error));
    names
        .chain(pointers)
        .map(|(line_no, message, severity)| {
            let line = lines.get(line_no).copied().unwrap_or_default();
            let start = line.chars().take_while(|c| c.is_whitespace()).count();
            Diagnostic { line: line_no, columns: start..line.chars().count(), severity, message }
        })
        .collect()
}

/// What a row's right-click menu asked for.
#[derive(Debug, Clone, Copy)]
enum RowAction {
    Duplicate,
    CopyId,
    RevertOrDelete,
}

/// `Duplicate / Copy id / Revert (or Delete for a new object)`.
fn row_menu(ui: &mut egui::Ui, i18n: &I18nMap, is_new: bool, changed: bool) -> Option<RowAction> {
    let duplicate = ui.button(i18n.t(I18nKey::NemesisDuplicate)).clicked();
    let copy = ui.button(i18n.t(I18nKey::NemesisCopyId)).clicked();
    let revert = changed && {
        ui.separator();
        let label = if is_new { I18nKey::NemesisDeleteObject } else { I18nKey::NemesisRevert };
        ui.add(Button::new(i18n.t(label)).destructive()).clicked()
    };
    [
        (duplicate, RowAction::Duplicate),
        (copy, RowAction::CopyId),
        (revert, RowAction::RevertOrDelete),
    ]
    .into_iter()
    .find_map(|(clicked, action)| clicked.then_some(action))
}

/// The language features that apply to the object editor (the hkanno options window's subset).
fn lsp_options_window(
    ctx: &egui::Context,
    i18n: &I18nMap,
    lsp: &mut HkannoLspOptions,
    open: &mut bool,
) {
    egui::Window::new(i18n.t(I18nKey::HkannoLspOptionsTitle))
        .open(open)
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| {
            let options = [
                (
                    &mut lsp.completion,
                    I18nKey::HkannoLspCompletion,
                    I18nKey::HkannoLspCompletionHover,
                ),
                (
                    &mut lsp.diagnostics,
                    I18nKey::HkannoLspDiagnostics,
                    I18nKey::HkannoLspDiagnosticsHover,
                ),
                (&mut lsp.hover, I18nKey::HkannoLspHover, I18nKey::HkannoLspHoverHover),
                (
                    &mut lsp.signature_help,
                    I18nKey::HkannoLspSignatureHelp,
                    I18nKey::HkannoLspSignatureHelpHover,
                ),
            ];
            for (value, label, hover) in options {
                ui.add(Checkbox::new(value, i18n.t(label))).on_hover_text(i18n.t(hover));
            }
        });
}
