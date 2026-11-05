//! The theme box of the page bar.

use egui::Label;
use gui_shared::{
    i18n::I18nKey,
    settings::ui::theme::{CustomTheme, Theme},
};

use crate::{
    app::App,
    theme::set_theme,
    ui::shadcn::{Button, ComboBox},
};

impl App {
    /// Theme combo box (System / Dark / Light / Custom) + the custom theme editor button.
    ///
    /// Shown in the page bar (see `router`), so the theme can be changed on every page.
    pub(crate) fn ui_theme_box(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_top(|ui| {
            ui.add(Label::new(self.i18n.t(I18nKey::ThemeLabel))).on_hover_text(self.i18n.t(I18nKey::ThemeHover));

            let themes: [(Theme, &str); 4] = [
                (Theme::System, self.i18n.t(I18nKey::ThemeSelectSystem)),
                (Theme::Dark, self.i18n.t(I18nKey::ThemeSelectDark)),
                (Theme::Light, self.i18n.t(I18nKey::ThemeSelectLight)),
                (Theme::Custom, self.i18n.t(I18nKey::ThemeSelectCustom)),
            ];

            let theme = &mut self.settings.ui.theme;
            let mut index = themes.iter().position(|(t, _)| t == theme).unwrap_or(0);
            if ComboBox::from_id_salt("theme")
                .width(120.0)
                .show_index(ui, &mut index, themes.len(), |i| themes[i].1)
                .changed()
            {
                *theme = themes[index].0;
                set_theme(ui.ctx(), self.settings.ui.theme, self.theme_manager.editing.as_ref());
            }
        });
        self.ui_bg_color_picker(ui);
    }

    fn ui_bg_color_picker(&mut self, ui: &mut egui::Ui) {
        if matches!(self.settings.ui.theme, Theme::Custom) {
            let button = Button::selectable(self.show_theme_editor, self.i18n.t(I18nKey::ThemeEditorButton));
            if ui.add(button).on_hover_text(self.i18n.t(I18nKey::ThemeEditorButtonHover)).clicked() {
                self.show_theme_editor ^= true;
            }
        }

        if matches!(self.settings.ui.theme, Theme::Custom)
            && self.show_theme_editor
            && let Some(update) = self.theme_manager.show(ui.ctx(), &mut self.show_theme_editor)
        {
            // Persist only the name.
            self.settings.ui.custom_theme = CustomTheme {
                selected_theme: Some(update.selected_name.clone()),
                themes_dir: self.theme_manager.cache.dir.to_string_lossy().to_string(),
            };

            // Live-apply whenever colors changed or a new preset was loaded.
            if let Some(preset) = &update.preset {
                crate::ui::theme::apply(preset, ui.ctx());
                set_theme(ui.ctx(), self.settings.ui.theme, Some(preset));
            }
        }
    }
}
