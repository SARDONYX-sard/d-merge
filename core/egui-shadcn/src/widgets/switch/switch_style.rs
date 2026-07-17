//! Maps switch state to concrete style values.

/// Resolves switch colors based on on/off state.
pub fn resolve_switch_style(
    theme: &crate::theme::shadcn_theme::ShadcnTheme,
    on: bool,
    anim_t: f32,
) -> super::resolved_switch_style::ResolvedSwitchStyle {
    let track_off = theme.input;
    let track_on = theme.primary;
    let track_color =
        crate::paint::interpolate_color::interpolate_color(track_off, track_on, anim_t);

    // In dark mode the off-track (input at 15% white) is very faint.
    // Add a border to define the track shape, matching shadcn's shadow-xs.
    let track_border = if !on {
        Some(theme.border)
    } else {
        None
    };

    let thumb_color = crate::paint::interpolate_color::interpolate_color(
        theme.foreground,
        theme.primary_foreground,
        anim_t,
    );

    super::resolved_switch_style::ResolvedSwitchStyle {
        track_color,
        track_border,
        thumb_color,
    }
}
