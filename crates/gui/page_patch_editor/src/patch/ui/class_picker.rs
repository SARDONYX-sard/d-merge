//! The "New object" window: the classes of the schema, filtered, those the behavior has first.
//!
//! ```text
//! ┌ New object ──────────────────┐
//! │ [clip                      ] │   the filter, with the keyboard when opened
//! │ hkbClipGenerator             │   the behavior has some: first
//! │ hkbClipTrigger               │   (weak: it has none)
//! └──────────────────────────────┘
//! ```
//! A click picks a class; `Esc` or `✖` closes it.

use egui_widgets::shadcn::TextEdit;
use gui_shared::i18n::{I18nKey, I18nMap};
use stdx::text::contains_ignore_case;

use crate::patch::workspace::Behavior;

/// How high the list gets before it scrolls.
const MAX_HEIGHT: f32 = 420.0;

/// See the module docs.
#[derive(Debug, Default)]
pub(super) struct ClassPicker {
    is_open: bool,
    filter: String,
    /// The filter takes the keyboard on the next draw.
    focus: bool,
}

impl ClassPicker {
    /// Shows the window, its filter with the keyboard.
    pub(super) const fn open(&mut self) {
        self.is_open = true;
        self.focus = true;
    }

    pub(super) const fn close(&mut self) {
        self.is_open = false;
    }

    /// Draws the window, if open, with the classes in `behavior` first. Returns the class clicked.
    pub(super) fn window(&mut self, ctx: &egui::Context, i18n: &I18nMap, behavior: &Behavior) -> Option<String> {
        if !self.is_open {
            return None;
        }

        let mut open = !ctx.input(|i| i.key_pressed(egui::Key::Escape));
        let window = egui_widgets::quick_pick_window(ctx, i18n.t(I18nKey::NemesisNewObject)).open(&mut open);
        let picked = window.show(ctx, |ui| self.contents(ui, i18n, behavior)).and_then(|shown| shown.inner).flatten();

        self.is_open &= open;
        picked
    }

    /// The filter and the classes.
    fn contents(&mut self, ui: &mut egui::Ui, i18n: &I18nMap, behavior: &Behavior) -> Option<String> {
        let field = TextEdit::singleline(&mut self.filter)
            .hint_text(i18n.t(I18nKey::NemesisClassFilterHint))
            .desired_width(ui.available_width());
        let field = ui.add(field);
        if std::mem::take(&mut self.focus) {
            field.request_focus();
        }

        let used = |class: &str| behavior.object_of_class(class).is_some();
        let classes = {
            let filter = self.filter.as_str();
            let mut classes: Vec<&str> =
                nemesis_diff::schema::class_names().into_iter().filter(|c| contains_ignore_case(c, filter)).collect();
            classes.sort_by_key(|class| !used(class)); // stable: alphabetical within each group
            classes
        };

        let mut picked = None;
        let row_height = ui.text_style_height(&egui::TextStyle::Monospace) + 4.0;
        let scroll = egui::ScrollArea::vertical().id_salt("nemesis_class_picker").max_height(MAX_HEIGHT);
        scroll.show_rows(ui, row_height, classes.len(), |ui, range| {
            for class in &classes[range] {
                let text = egui::RichText::new(*class).monospace();
                let text = if used(class) { text } else { text.weak() };
                if ui.add(egui::Button::selectable(false, text)).clicked() {
                    picked = Some((*class).to_owned());
                }
            }
        });
        picked
    }
}
