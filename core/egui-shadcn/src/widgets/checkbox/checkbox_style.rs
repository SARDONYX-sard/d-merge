//! Maps checkbox state to concrete style values.

/// Resolves checkbox colors based on checked state and interaction.
pub fn resolve_checkbox_style(
    theme: &crate::theme::shadcn_theme::ShadcnTheme,
    checked: bool,
    hovered: bool,
    _disabled: bool,
) -> super::resolved_checkbox_style::ResolvedCheckboxStyle {
    let (box_bg, box_border, check_color) = if checked {
        (theme.primary, theme.primary, theme.primary_foreground)
    } else if hovered {
        (egui::Color32::TRANSPARENT, theme.ring, egui::Color32::TRANSPARENT)
    } else {
        (egui::Color32::TRANSPARENT, theme.border, egui::Color32::TRANSPARENT)
    };

    super::resolved_checkbox_style::ResolvedCheckboxStyle {
        box_bg,
        box_border,
        check_color,
        text_color: theme.foreground,
    }
}
