//! The top of the side bar: the template files being patched, and how to add one.
//!
//! ```text
//! ▼ BEHAVIORS               [✓] [+]    <- check the whole work; templates, a file of your own, animation data
//!   0_master (3)                       <- right click: check, remove. Hover: where it goes.
//!   ⚠ draugrbehavior                   <- only d-merge will read this one
//! ```
//! The template list opens as a window with the filter focused, like VS Code's quick open, but next
//! to what was clicked.

use std::{
    path::{Path, PathBuf},
    rc::Rc,
};

use egui_widgets::{
    notify::Notification,
    shadcn::{Button, Icon, TextEdit},
};
use gui_shared::i18n::{I18nKey, I18nMap};
use nemesis_diff::is_existing_mod_code;

use crate::patch::{
    PatchTool,
    workspace::{Behavior, Target, TemplateScope},
};

/// The height of the list before it scrolls: most of the side bar is for the objects.
const LIST_HEIGHT: f32 = 130.0;

/// What a row asked for.
#[derive(Debug, Clone, Copy)]
enum RowEvent {
    Remove(usize),
    /// Check what it changes, see [`super::work_check`].
    Check(usize),
}

/// What the buttons of the header asked for.
#[derive(Debug, Clone, Copy)]
enum AddEvent {
    Template,
    File,
    AnimData,
    /// See [`PatchTool::start_whole_work_check`].
    CheckWork,
}

/// `▼ TITLE ........ actions`: a side bar section's header. A click on the title folds it.
pub(super) fn section_header(ui: &mut egui::Ui, open: &mut bool, title: &str, actions: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        let glyph = if *open { "▼" } else { "▶" };
        let text = egui::RichText::new(format!("{glyph} {}", title.to_uppercase())).small().strong();
        if ui.add(egui::Label::new(text).sense(egui::Sense::click())).clicked() {
            *open = !*open;
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), actions);
    });
}

impl PatchTool {
    /// See the module docs.
    pub(in crate::patch) fn behaviors_section(&mut self, ui: &mut egui::Ui, i18n: &I18nMap, notify: &mut Notification) {
        let mut add = None;
        let mut open = self.behaviors_open;
        let scope = self.workspace.scope;
        section_header(ui, &mut open, i18n.t(I18nKey::NemesisBehaviorsSection), |ui| add = add_menu(ui, scope, i18n));
        self.behaviors_open = open;
        match add {
            Some(AddEvent::Template) => {
                self.picker_open = true;
                self.focus_quick_pick = true;
            }
            Some(AddEvent::File) => self.add_custom_template(notify, i18n),
            Some(AddEvent::AnimData) => self.open_anim_data_for_mod(),
            Some(AddEvent::CheckWork) => self.start_whole_work_check(),
            None => {}
        }
        if self.behaviors_open {
            let event = self.behavior_rows(ui, i18n);
            self.apply_row_event(event);
        }
        ui.separator();
    }

    fn add_custom_template(&mut self, notify: &mut Notification, i18n: &I18nMap) {
        let dialog = gui_shared::rfd::FileDialog::new().add_filter("Template", &["xml", "bin"]);
        if let Some(path) = dialog.set_directory(&self.template_dir).pick_file() {
            self.add_behavior(&path, notify, i18n);
        }
    }

    /// The mod the work was read from, or else the one it makes, or else asks for one.
    fn open_anim_data_for_mod(&mut self) {
        let dirs = [self.mod_dir.trim(), self.output.trim()];
        let chosen = dirs.into_iter().find(|dir| !dir.is_empty()).map(PathBuf::from);
        if let Some(root) = chosen.or_else(|| gui_shared::rfd::FileDialog::new().pick_folder()) {
            self.open_anim_data_dialog(&root);
        }
    }

    /// The rows. Their hover says where the patches go, as there is no room for it in the row.
    fn behavior_rows(&mut self, ui: &mut egui::Ui, i18n: &I18nMap) -> Option<RowEvent> {
        if self.workspace.behaviors.is_empty() {
            ui.weak(i18n.t(I18nKey::NemesisNoBehavior));
            return None;
        }
        let code = if is_existing_mod_code(self.mod_code.trim()) { self.mod_code.trim() } else { "<code>" };
        let ext = self.workspace.export_format.is_ext();
        let mut event = None;
        egui::ScrollArea::vertical().id_salt("nemesis_behaviors").max_height(LIST_HEIGHT).show(ui, |ui| {
            for (i, behavior) in self.workspace.behaviors.iter().enumerate() {
                let response = ui
                    .add(Button::selectable(i == self.active, row_label(behavior, ext)))
                    .on_hover_text(row_hover(behavior, code, ext, i18n));
                if response.clicked() && i != self.active {
                    self.active = i;
                    // Back to where it was left in that behavior.
                    self.selected = self.tabs.last_of(i).map(Rc::from);
                }
                response.context_menu(|ui| {
                    let picked = row_menu(ui, i, i18n);
                    event = picked.or(event);
                });
            }
        });
        event
    }

    fn apply_row_event(&mut self, event: Option<RowEvent>) {
        match event {
            Some(RowEvent::Check(i)) => {
                if i != self.active {
                    self.active = i;
                    self.selected = self.tabs.last_of(i).map(Rc::from);
                }
                self.start_work_check(super::work_check::CheckScope::All);
            }
            Some(RowEvent::Remove(i)) => {
                self.workspace.behaviors.remove(i);
                self.reset_names(); // the numbers of the others shift
                self.tabs.clear();
                self.active = self.active.min(self.workspace.behaviors.len().saturating_sub(1));
                self.selected = None;
                self.editor_for = None;
                self.mark_dirty();
            }
            None => {}
        }
    }

    /// The window of the templates of `template_dir`; a click adds one as a behavior.
    pub(in crate::patch) fn template_picker_window(
        &mut self,
        ctx: &egui::Context,
        i18n: &I18nMap,
        notify: &mut Notification,
    ) {
        if !self.picker_open {
            return;
        }
        let scope = self.workspace.scope;
        let fresh = self.catalog_for.as_ref().is_some_and(|(dir, of)| *dir == self.template_dir && *of == scope);
        if !fresh {
            self.catalog = crate::patch::workspace::catalog(Path::new(&self.template_dir), scope);
            self.catalog_for = Some((self.template_dir.clone(), scope));
        }
        let mut open = !ctx.input(|i| i.key_pressed(egui::Key::Escape));
        let mut picked = None;
        quick_pick_window(ctx, i18n.t(I18nKey::NemesisAddBehavior)).open(&mut open).show(ctx, |ui| {
            picked = self.template_picker_ui(ui, i18n);
        });
        self.picker_open = open && picked.is_none();
        if let Some(path) = picked {
            self.add_behavior(&path, notify, i18n);
        }
    }

    fn template_picker_ui(&mut self, ui: &mut egui::Ui, i18n: &I18nMap) -> Option<PathBuf> {
        let field = TextEdit::singleline(&mut self.picker_filter)
            .hint_text(i18n.t(I18nKey::NemesisFilterHint))
            .desired_width(ui.available_width());
        let field = ui.add(field);
        if std::mem::take(&mut self.focus_quick_pick) {
            field.request_focus();
        }
        ui.weak(format!("{} ({})", self.template_dir, self.catalog.len()));
        if self.workspace.scope == TemplateScope::Nemesis {
            ui.weak(i18n.t(I18nKey::NemesisScopeNemesisOnly));
        }
        if self.catalog.is_empty() {
            ui.colored_label(ui.visuals().warn_fg_color, i18n.t(I18nKey::NemesisNoTemplates));
            return None;
        }
        // No filter: as a directory tree. A filter wants all the matches at once.
        let clicked = if self.picker_filter.is_empty() {
            super::template_picker::tree_ui(ui, &self.catalog, &mut self.picker_toggled, i18n)
        } else {
            super::template_picker::filtered_ui(ui, &self.catalog, &self.picker_filter, i18n)
        };
        clicked.map(Path::to_path_buf)
    }
}

/// A small window of one filter and its list.
pub(super) fn quick_pick_window<'a>(ctx: &egui::Context, title: &str) -> egui::Window<'a> {
    egui_widgets::window_near_pointer(ctx, title).collapsible(false).resizable(false).default_width(460.0)
}

/// `[+]` and its menu, and `[✓]` checking the whole work (right to left). A template file of your own
/// is never one Nemesis knows: only for the `Extended` scope.
fn add_menu(ui: &mut egui::Ui, scope: TemplateScope, i18n: &I18nMap) -> Option<AddEvent> {
    let mut event = None;
    let check = Button::new("").icon(Icon::ScanSearch).small();
    let hover = |ui: &mut egui::Ui| {
        ui.strong(i18n.t(I18nKey::NemesisCheckWork));
        ui.label(i18n.t(I18nKey::NemesisCheckWorkHover));
    };
    if ui.add(check).on_hover_ui(hover).clicked() {
        event = Some(AddEvent::CheckWork);
    }
    let items = [
        (I18nKey::NemesisAddBehavior, None, AddEvent::Template),
        (I18nKey::NemesisCustomXml, Some(I18nKey::NemesisCustomXmlHover), AddEvent::File),
        (I18nKey::NemesisAnimData, Some(I18nKey::NemesisAnimDataHover), AddEvent::AnimData),
    ];
    ui.menu_button("+", |ui| {
        for (label, hover, item) in items {
            if matches!(item, AddEvent::File) && !scope.allows(false) {
                continue;
            }
            let mut response = ui.button(i18n.t(label));
            if let Some(hover) = hover {
                response = response.on_hover_text(i18n.t(hover));
            }
            if response.clicked() {
                event = Some(item);
                ui.close();
            }
        }
    });
    event
}

/// `0_master (3)`, with the number of changed objects, and `⚠` if only d-merge will read it.
/// `ext`: see [`Behavior::target`].
fn row_label(behavior: &Behavior, ext: bool) -> String {
    let count = behavior.edits.len() + behavior.added.len();
    let warn = if is_ext_only(behavior, ext) { "⚠ " } else { "" };
    if count > 0 { format!("{warn}{} ({count})", behavior.name) } else { format!("{warn}{}", behavior.name) }
}

/// Is `behavior` written where only d-merge reads it?
fn is_ext_only(behavior: &Behavior, ext: bool) -> bool {
    behavior.target(ext) == Target::Ext && !behavior.is_anim_data()
}

/// The template, where its patches go, and the warning of [`row_label`].
fn row_hover(behavior: &Behavior, code: &str, ext: bool, i18n: &I18nMap) -> String {
    let to = behavior.patch_dir(code, ext);
    let mut hover = format!("{}\n→ {}", behavior.template_path.display(), to.display());
    if is_ext_only(behavior, ext) {
        hover.push('\n');
        hover.push_str(i18n.t(I18nKey::NemesisExtOnlyWarning));
    }
    hover
}

/// `Check`, `Remove`. Where it is written is the export's choice: see `ExportFormat`.
fn row_menu(ui: &mut egui::Ui, index: usize, i18n: &I18nMap) -> Option<RowEvent> {
    let mut event = None;
    if ui.button(i18n.t(I18nKey::NemesisCheck)).on_hover_text(i18n.t(I18nKey::NemesisCheckHover)).clicked() {
        event = Some(RowEvent::Check(index));
        ui.close();
    }
    ui.separator();
    let remove = Button::new(i18n.t(I18nKey::NemesisRemoveBehavior)).destructive().icon(Icon::Trash);
    if ui.add(remove).on_hover_text(i18n.t(I18nKey::NemesisRemoveBehaviorHover)).clicked() {
        event = Some(RowEvent::Remove(index));
        ui.close();
    }
    event
}
