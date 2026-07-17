//! Maps input state to concrete style values.

/// Resolves input colors based on focus state.
pub fn resolve_input_style(
    theme: &crate::theme::shadcn_theme::ShadcnTheme,
    focused: bool,
) -> super::resolved_input_style::ResolvedInputStyle {
    let border_color = if focused { theme.ring } else { theme.border };
    let corner_radius = theme.radius; // rounded-lg (Nova)

    super::resolved_input_style::ResolvedInputStyle {
        bg: crate::paint::interpolate_color::interpolate_color(
            theme.background,
            theme.muted,
            0.4,
        ),
        border_color,
        text_color: theme.foreground,
        placeholder_color: theme.muted_foreground,
        corner_radius,
    }
}
