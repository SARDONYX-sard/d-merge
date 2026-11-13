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

use crate::patch::{
    repeat::{expand, values, with_id},
    workspace::Behavior,
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

/// What the window asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::patch) enum RepeatAsk {
    /// `✖`: the window goes.
    Close,
    /// `Create`: see [`RepeatForm::create`].
    Create,
}

impl RepeatForm {
    /// The window with object `id` of `behavior` (index `at`) as the template. `None`: no such object.
    pub(in crate::patch) fn of(at: usize, behavior: &Behavior, id: &str) -> Option<Self> {
        let template = behavior.text(id)?.to_owned();
        Some(Self { behavior: at, from: id.to_owned(), template, values: String::new() })
    }

    /// The behavior the objects go to, in `Workspace::behaviors`.
    pub(in crate::patch) const fn behavior(&self) -> usize {
        self.behavior
    }

    /// Draws the window. `code_ok`: the mod code is one new objects can be named by.
    pub(in crate::patch) fn window(&mut self, ctx: &egui::Context, i18n: &I18nMap, code_ok: bool) -> Option<RepeatAsk> {
        let mut open = true;
        let mut create = false;
        let rows = values(&self.values).rows.len();

        let title = format!("{} {}", i18n.t(I18nKey::RepeatTitle), self.from);
        let window = egui_widgets::window_near_pointer(ctx, title).open(&mut open);
        window.default_size([640.0, 520.0]).resizable(true).show(ctx, |ui| {
            self.fields(ui, i18n);
            ui.horizontal(|ui| {
                ui.weak(format!("{rows} {}", i18n.t(I18nKey::RepeatObjects)));
                let button = egui_widgets::shadcn::Button::new(i18n.t(I18nKey::RepeatCreate)).primary();
                create = ui
                    .add_enabled(rows > 0 && code_ok, button)
                    .on_disabled_hover_text(i18n.t(I18nKey::NemesisModCodeInvalid))
                    .clicked();
            });
        });

        if create { Some(RepeatAsk::Create) } else { (!open).then_some(RepeatAsk::Close) }
    }

    /// The template and the values, each in a field of its own.
    fn fields(&mut self, ui: &mut egui::Ui, i18n: &I18nMap) {
        ui.label(i18n.t(I18nKey::RepeatTemplate));
        egui::ScrollArea::vertical().id_salt("repeat_template").max_height(240.0).show(ui, |ui| {
            ui.add(egui::TextEdit::multiline(&mut self.template).code_editor().desired_width(f32::INFINITY));
        });

        ui.label(i18n.t(I18nKey::RepeatValues));
        egui::ScrollArea::vertical().id_salt("repeat_values").max_height(160.0).show(ui, |ui| {
            let hint = "name, file
A, a.hkx
B, b.hkx";
            let field = egui::TextEdit::multiline(&mut self.values).code_editor().hint_text(hint);
            ui.add(field.desired_width(f32::INFINITY).desired_rows(5));
        });
    }

    /// Makes one new object per row of the values in `behavior`, named by `code`. Returns the first.
    pub(in crate::patch) fn create(&self, behavior: &mut Behavior, code: &str) -> Option<String> {
        let read = values(&self.values);
        let mut first = None;
        for index in 0..read.rows.len() {
            let id = behavior.add_object(code, None);
            let text = with_id(&expand(&self.template, &read, index), &id);
            behavior.added.insert(id.clone(), text);
            first.get_or_insert(id);
        }
        first
    }
}
