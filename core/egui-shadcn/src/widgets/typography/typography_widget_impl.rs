//! Widget trait implementation for Typography.

impl egui::Widget for super::typography::Typography {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let theme = crate::theme::shadcn_theme_ext::ShadcnThemeExt::shadcn_theme(ui.ctx());
        let (font_size, _line_height, is_bold) = self.variant.metrics();

        let color = match self.variant {
            crate::tokens::typography_variant::TypographyVariant::Muted => theme.muted_foreground,
            crate::tokens::typography_variant::TypographyVariant::Lead => theme.muted_foreground,
            _ => theme.foreground,
        };

        let mut rich_text = egui::RichText::new(self.text).color(color).size(font_size);
        if is_bold {
            rich_text = rich_text.strong();
        }

        ui.label(rich_text)
    }
}
