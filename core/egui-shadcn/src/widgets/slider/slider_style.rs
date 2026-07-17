//! Maps slider state to concrete style values.

/// Resolves slider colors based on theme.
pub fn resolve_slider_style(
    theme: &crate::theme::shadcn_theme::ShadcnTheme,
) -> super::resolved_slider_style::ResolvedSliderStyle {
    super::resolved_slider_style::ResolvedSliderStyle {
        track_color: theme.muted,
        fill_color: theme.primary,
        handle_fill: theme.background,
        handle_border: theme.ring, // Nova: border-ring on thumb
    }
}
