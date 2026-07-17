//! Maps select state to concrete style values.

/// Resolves select colors based on theme.
pub fn resolve_select_style(
    theme: &crate::theme::shadcn_theme::ShadcnTheme,
) -> super::resolved_select_style::ResolvedSelectStyle {
    let corner_radius = theme.radius; // rounded-lg

    super::resolved_select_style::ResolvedSelectStyle {
        trigger_bg: theme.background,
        trigger_border: theme.border,
        trigger_text: theme.foreground,
        popover_bg: theme.popover,
        popover_border: egui::Color32::from_rgba_unmultiplied(
            theme.foreground.r(),
            theme.foreground.g(),
            theme.foreground.b(),
            26, // ring-foreground/10
        ),
        item_text: theme.popover_foreground,
        item_hover_bg: theme.accent,
        corner_radius,
    }
}
