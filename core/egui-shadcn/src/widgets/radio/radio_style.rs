//! Maps radio state to concrete style values.

/// Resolves radio button colors based on selection state and interaction.
pub fn resolve_radio_style(
    theme: &crate::theme::shadcn_theme::ShadcnTheme,
    selected: bool,
    hovered: bool,
) -> super::resolved_radio_style::ResolvedRadioStyle {
    let circle_border = if selected {
        theme.primary
    } else if hovered {
        theme.ring
    } else {
        theme.input // Nova: border-input instead of border
    };

    let dot_color = if selected { theme.primary } else { egui::Color32::TRANSPARENT };

    super::resolved_radio_style::ResolvedRadioStyle {
        circle_border,
        dot_color,
        text_color: theme.foreground,
    }
}
