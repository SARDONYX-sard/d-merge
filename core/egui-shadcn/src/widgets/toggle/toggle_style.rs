//! Maps toggle state to concrete style values.

/// Resolves toggle colors based on pressed state, variant, and interaction.
pub fn resolve_toggle_style(
    theme: &crate::theme::shadcn_theme::ShadcnTheme,
    variant: crate::tokens::toggle_variant::ToggleVariant,
    pressed: bool,
    hovered: bool,
) -> super::resolved_toggle_style::ResolvedToggleStyle {
    let corner_radius = theme.radius; // rounded-lg (Nova)

    let border = match variant {
        crate::tokens::toggle_variant::ToggleVariant::Outline => Some(theme.border),
        crate::tokens::toggle_variant::ToggleVariant::Default => None,
    };

    let (bg, fg) = if pressed {
        (theme.accent, theme.accent_foreground)
    } else if hovered {
        (theme.muted, theme.muted_foreground)
    } else {
        (egui::Color32::TRANSPARENT, theme.foreground)
    };

    super::resolved_toggle_style::ResolvedToggleStyle {
        bg,
        fg,
        border,
        corner_radius,
    }
}
