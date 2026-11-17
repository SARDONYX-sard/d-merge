//! The top of the side bar: the template files being patched, and how to add one.
//!
//! ```text
//! ▼ BEHAVIORS                    [+]   <- templates, a file of your own, animation data
//!   0_master (3)                       <- right click: remove. Hover: where it goes.
//!   ⚠ draugrbehavior                   <- only d-merge will read this one
//! ```
//! The template list opens as a window with the filter focused, like VS Code's quick open, but next
//! to what was clicked.

use std::path::PathBuf;

use egui_widgets::{
    HoverMarkdown as _,
    notify::Notification,
    shadcn::{Button, Icon},
};
use gui_shared::i18n::{I18nKey, I18nMap};
use nemesis_diff::is_existing_mod_code;

use super::{SideArea, SideAsk};
use crate::patch::{
    tool::{selection::Selection, work_state::Work},
    ui::windows::WindowAsk,
    workspace::{Behavior, TemplateScope},
};

/// The height of the list before it scrolls: most of the side bar is for the objects.
const LIST_HEIGHT: f32 = 130.0;

/// What a row asked for.
#[derive(Debug, Clone, Copy)]
enum RowEvent {
    /// Another behavior is shown.
    Select(usize),
    Remove(usize),
}

/// What the buttons of the header asked for.
#[derive(Debug, Clone, Copy)]
enum AddEvent {
    Template,
    File,
    AnimData,
}

impl SideArea {
    /// See the module docs. What `[+]` asks for opens a window, or adds a behavior now.
    pub(super) fn behaviors_section(
        &mut self,
        ui: &mut egui::Ui,
        i18n: &I18nMap,
        (work, sel): (&mut Work, &mut Selection),
        notify: &mut Notification,
    ) -> Vec<SideAsk> {
        let mut asks = Vec::new();
        let mut add = None;
        let scope = work.workspace.scope;
        egui_widgets::section_header(ui, &mut self.bar.behaviors_open, i18n.t(I18nKey::BehaviorsTitle), |ui| {
            add = add_menu(ui, scope, i18n);
        });
        match add {
            Some(AddEvent::Template) => asks.push(SideAsk::Window(WindowAsk::TemplatePicker)),
            Some(AddEvent::File) => add_custom_template((work, sel), notify, i18n),
            Some(AddEvent::AnimData) => {
                asks.extend(anim_data_root(work).map(|root| SideAsk::Window(WindowAsk::AnimData(root))));
            }
            None => {}
        }
        if !self.bar.behaviors_open {
            ui.separator();
            return asks;
        }

        let event = {
            let code = work.place.mod_code.trim();
            let code = if is_existing_mod_code(code) { code } else { "<code>" };
            let ext = work.workspace.export_format.is_ext();
            behavior_rows(ui, i18n, (&work.workspace.behaviors, sel.active), (code, ext))
        };
        ui.separator();
        match event {
            Some(RowEvent::Select(at)) => asks.push(SideAsk::ShowBehavior(at)),
            Some(RowEvent::Remove(at)) => {
                work.workspace.behaviors.remove(at);
                work.reset_names(); // the numbers of the others shift
                work.mark_dirty();
                sel.active = sel.active.min(work.workspace.behaviors.len().saturating_sub(1));
                sel.selected = None;
                asks.push(SideAsk::BehaviorRemoved);
            }
            None => {}
        }
        asks
    }
}

/// A template file of the user's own, picked in a file dialog, as a behavior of the work.
fn add_custom_template((work, sel): (&mut Work, &mut Selection), notify: &mut Notification, i18n: &I18nMap) {
    let dialog = gui_shared::rfd::FileDialog::new().add_filter("Template", &["xml", "bin"]);
    if let Some(path) = dialog.set_directory(&work.place.template_dir).pick_file() {
        work.add_behavior(sel, &path, notify, i18n);
    }
}

/// The mod the animation data window is of: the one the work was read from, or else the one it
/// makes, or else one asked for.
fn anim_data_root(work: &Work) -> Option<PathBuf> {
    let dirs = [work.place.mod_dir.trim(), work.place.output.trim()];
    let chosen = dirs.into_iter().find(|dir| !dir.is_empty()).map(PathBuf::from);
    chosen.or_else(|| gui_shared::rfd::FileDialog::new().pick_folder())
}

/// `[+]` and its menu. A template file of your own
/// is never one Nemesis knows: only for the `Extended` scope.
fn add_menu(ui: &mut egui::Ui, scope: TemplateScope, i18n: &I18nMap) -> Option<AddEvent> {
    let mut event = None;

    let items = [
        (I18nKey::BehaviorsAdd, None, AddEvent::Template),
        (I18nKey::BehaviorsCustomXml, Some(I18nKey::BehaviorsCustomXmlHover), AddEvent::File),
        (I18nKey::AnimDataOpen, Some(I18nKey::AnimDataOpenHover), AddEvent::AnimData),
    ];
    ui.menu_button("+", |ui| {
        for (label, hover, item) in items {
            if matches!(item, AddEvent::File) && !scope.allows(false) {
                continue;
            }
            let mut response = ui.button(i18n.t(label));
            if let Some(hover) = hover {
                response = response.on_hover_markdown(i18n.t(hover));
            }
            if response.clicked() {
                event = Some(item);
                ui.close();
            }
        }
    });
    event
}

/// The rows. Their hover says where the patches go, as there is no room for it in the row.
/// `(code, ext)`: the mod code (or `<code>` while it is not one), and see [`Behavior::target`].
fn behavior_rows(
    ui: &mut egui::Ui,
    i18n: &I18nMap,
    (behaviors, active): (&[Behavior], usize),
    (code, ext): (&str, bool),
) -> Option<RowEvent> {
    if behaviors.is_empty() {
        ui.weak(i18n.t(I18nKey::BehaviorsNone));
        return None;
    }

    let mut event = None;
    egui::ScrollArea::both().id_salt("nemesis_behaviors").max_height(LIST_HEIGHT).show(ui, |ui| {
        for (i, behavior) in behaviors.iter().enumerate() {
            let row = Button::selectable(i == active, row_label(behavior));
            let response = ui.add(row).on_hover_text(row_hover(behavior, code, ext));
            if response.clicked() && i != active {
                event = Some(RowEvent::Select(i));
            }
            response.context_menu(|ui| {
                if let Some(picked) = row_menu(ui, i, i18n) {
                    event = Some(picked);
                }
            });
        }
    });
    event
}

/// `0_master (3)`, with the number of changed objects.
fn row_label(behavior: &Behavior) -> String {
    let count = behavior.edits.len() + behavior.added.len();
    if count > 0 { format!("{} ({count})", behavior.name) } else { behavior.name.clone() }
}

/// The template, and where Export writes its patches: `Nemesis_Engine` or `Nemesis_EngineExt` is
/// only the format's choice (`ExportFormat`), nothing to warn of. `ext`: see [`Behavior::target`].
fn row_hover(behavior: &Behavior, code: &str, ext: bool) -> String {
    let to = behavior.patch_dir(code, ext);
    format!("{}\n→ {}", behavior.template_path.display(), to.display())
}

/// `Remove`. Where it is written is the export's choice: see `ExportFormat`.
fn row_menu(ui: &mut egui::Ui, index: usize, i18n: &I18nMap) -> Option<RowEvent> {
    let mut event = None;
    let remove = Button::new(i18n.t(I18nKey::BehaviorsRemove)).destructive().icon(Icon::Trash);
    if ui.add(remove).on_hover_text(i18n.t(I18nKey::BehaviorsRemoveHover)).clicked() {
        event = Some(RowEvent::Remove(index));
        ui.close();
    }
    event
}
