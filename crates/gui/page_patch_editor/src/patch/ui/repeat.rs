//! The window of [`crate::patch::repeat`]: many new objects from one, as a loop would.
//!
//! ```text
//! ┌ Repeat #0106 ────────────────────────────────────────────┐
//! │ Template ({{name}} takes a column's value, {{i}} the row's number)
//! │ ┌──────────────────────────────────────────────────────┐ │
//! │ │ <hkparam name="name">Attack_{{name}}</hkparam>       │ │
//! │ └──────────────────────────────────────────────────────┘ │
//! │ Values (the first line names the columns)                │
//! │ ┌──────────────────────────────────────────────────────┐ │
//! │ │ name, file                                           │ │
//! │ │ A, a.hkx                                             │ │
//! │ └──────────────────────────────────────────────────────┘ │
//! │ 1 objects: #mymod$4 ...                       [Create]   │
//! └──────────────────────────────────────────────────────────┘
//! ```

use gui_shared::i18n::{I18nKey, I18nMap};
use nemesis_diff::is_existing_mod_code;

use crate::patch::{
    PatchTool,
    repeat::{expand, values, with_id},
};

/// What the window holds.
#[derive(Debug)]
pub(in crate::patch) struct RepeatForm {
    /// The behavior the objects go to, in `Workspace::behaviors`.
    behavior: usize,
    /// e.g. `#0106`: what the template was taken from.
    from: String,
    template: String,
    values: String,
}

impl PatchTool {
    /// Opens the window with object `id` of the active behavior as the template.
    pub(in crate::patch) fn open_repeat(&mut self, id: &str) {
        let Some(text) = self.workspace.behaviors.get(self.active).and_then(|behavior| behavior.text(id)) else {
            return;
        };
        let form =
            RepeatForm { behavior: self.active, from: id.to_owned(), template: text.to_owned(), values: String::new() };
        self.repeat = Some(form);
    }

    /// Draws the window if it is open, and makes the objects when asked.
    pub(in crate::patch) fn repeat_window(&mut self, ctx: &egui::Context, i18n: &I18nMap) {
        let Some(form) = &mut self.repeat else { return };
        let mut open = true;
        let mut create = false;
        let rows = values(&form.values).rows.len();
        let code_ok = is_existing_mod_code(self.mod_code.trim());
        egui_widgets::window_near_pointer(ctx, format!("{} {}", i18n.t(I18nKey::RepeatTitle), form.from))
            .open(&mut open)
            .default_size([640.0, 520.0])
            .resizable(true)
            .show(ctx, |ui| {
                ui.label(i18n.t(I18nKey::RepeatTemplate));
                egui::ScrollArea::vertical().id_salt("repeat_template").max_height(240.0).show(ui, |ui| {
                    ui.add(egui::TextEdit::multiline(&mut form.template).code_editor().desired_width(f32::INFINITY));
                });
                ui.label(i18n.t(I18nKey::RepeatValues));
                egui::ScrollArea::vertical().id_salt("repeat_values").max_height(160.0).show(ui, |ui| {
                    let hint = "name, file\nA, a.hkx\nB, b.hkx";
                    let field = egui::TextEdit::multiline(&mut form.values).code_editor().hint_text(hint);
                    ui.add(field.desired_width(f32::INFINITY).desired_rows(5));
                });
                ui.horizontal(|ui| {
                    ui.weak(format!("{rows} {}", i18n.t(I18nKey::RepeatObjects)));
                    let button = egui_widgets::shadcn::Button::new(i18n.t(I18nKey::RepeatCreate)).primary();
                    create = ui
                        .add_enabled(rows > 0 && code_ok, button)
                        .on_disabled_hover_text(i18n.t(I18nKey::NemesisModCodeInvalid))
                        .clicked();
                });
            });
        if create {
            self.create_repeated();
        } else if !open {
            self.repeat = None;
        }
    }

    /// Makes one new object per row of the values, and shows the first.
    fn create_repeated(&mut self) {
        let Some(form) = self.repeat.take() else { return };
        let code = self.mod_code.trim().to_owned();
        let read = values(&form.values);
        let Some(behavior) = self.workspace.behaviors.get_mut(form.behavior) else { return };
        let mut first = None;
        for index in 0..read.rows.len() {
            let id = behavior.add_object(&code, None);
            let text = with_id(&expand(&form.template, &read, index), &id);
            behavior.added.insert(id.clone(), text);
            first.get_or_insert(id);
        }
        self.active = form.behavior;
        self.reset_names();
        self.mark_dirty();
        self.selected = first.map(Into::into);
    }
}
