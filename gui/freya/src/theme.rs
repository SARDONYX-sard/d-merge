//! Theme and window background appearance.

use d_merge_gui_shared::settings::ui::theme::{CustomTheme, Theme as ThemeSetting};
use freya::prelude::*;
use freya_extra::{acrylic, themes};

/// How the window background is painted.
///
/// The window is always created with transparency enabled, so the background can be switched at runtime.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum Appearance {
    /// Blur what is behind the window (Windows only).
    #[cfg_attr(target_os = "windows", default)]
    Acrylic,
    /// Fully transparent background. Only the panels are painted.
    #[cfg_attr(not(target_os = "windows"), default)]
    Transparent,
    /// Opaque background color of the theme.
    Color,
}

impl Appearance {
    pub(crate) const ALL: [Self; 3] = [Self::Acrylic, Self::Transparent, Self::Color];

    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Acrylic => "Acrylic",
            Self::Transparent => "Transparent",
            Self::Color => "Color",
        }
    }

    /// Switch the native backdrop to match `self`.
    ///
    /// # Errors
    /// Returns an error message if the acrylic backdrop cannot be changed.
    pub(crate) fn apply_backdrop(self) -> Result<(), &'static str> {
        let want_acrylic = self == Self::Acrylic;
        if want_acrylic == acrylic::is_enabled() {
            return Ok(());
        }

        if want_acrylic { acrylic::enable() } else { acrylic::disable() }
    }
}

/// Registers the native window handle so that the acrylic backdrop can be toggled later.
pub(crate) fn register_window_handle(window: &mut freya::winit::window::Window) {
    #[cfg(target_os = "windows")]
    {
        use freya::winit::{
            platform::windows::WindowExtWindows as _, raw_window_handle::RawWindowHandle,
        };

        let Ok(handle) = (unsafe { window.window_handle_any_thread() }) else {
            return;
        };

        let hwnd = match handle.as_raw() {
            RawWindowHandle::Win32(handle) => match handle.hwnd.get().try_into() {
                Ok(address) => address,
                Err(_) => return,
            },
            RawWindowHandle::WinRt(handle) => handle.core_window.as_ptr() as usize,
            _ => return,
        };

        acrylic::register_window(hwnd);

        if Appearance::default() == Appearance::Acrylic
            && let Err(err) = acrylic::enable()
        {
            tracing::error!("Failed to enable acrylic backdrop: {err}");
        }
    }

    #[cfg(not(target_os = "windows"))]
    let _ = window;
}

/// Converts the persisted theme setting into a freya theme.
///
/// `System`/`Dark`/`Light` use the same colors as the egui GUI(egui-shadcn).
/// `Custom` uses the saved preset(`custom.freya_selected_theme`), or the liquid glass theme if there is none.
pub(crate) fn to_freya_theme(setting: ThemeSetting, custom: &CustomTheme) -> Theme {
    let theme = match setting {
        ThemeSetting::System => match dark_light::detect() {
            Ok(dark_light::Mode::Light) => shadcn_light(),
            _ => shadcn_dark(),
        },
        ThemeSetting::Dark => shadcn_dark(),
        ThemeSetting::Light => shadcn_light(),
        ThemeSetting::Custom => custom
            .freya_selected_theme
            .as_deref()
            .and_then(|name| {
                crate::theme_preset::load_preset(&custom.themes_dir, name)
                    .inspect_err(|err| tracing::error!("{err}"))
                    .ok()
            })
            .map_or_else(themes::liquid_glass::theme, |preset| preset.to_theme()),
    };
    with_egui_metrics(theme)
}

const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgb(r, g, b)
}

/// egui-shadcn dark (with the d-merge tweak: foreground = 200)
fn shadcn_dark() -> Theme {
    let mut theme = dark_theme();
    theme.name = "shadcn-dark";
    theme.colors = ColorsSheet {
        primary: rgb(229, 229, 229),
        secondary: rgb(23, 23, 23),
        tertiary: rgb(200, 200, 200),

        success: rgb(0, 200, 83),
        warning: rgb(255, 193, 7),
        error: rgb(255, 99, 105),
        info: rgb(120, 220, 255),

        // egui panel fill
        background: rgb(30, 30, 30),
        surface_primary: rgb(23, 23, 23),
        surface_secondary: rgb(38, 38, 38),
        // shadcn background: buttons and inputs
        surface_tertiary: rgb(10, 10, 10),
        surface_inverse: rgb(229, 229, 229),
        surface_inverse_secondary: rgb(200, 200, 200),
        surface_inverse_tertiary: rgb(161, 161, 161),

        border: Color::from_argb(38, 255, 255, 255),
        border_focus: rgb(115, 115, 115),
        border_disabled: rgb(64, 64, 64),

        text_primary: rgb(200, 200, 200),
        text_secondary: rgb(161, 161, 161),
        text_placeholder: rgb(115, 115, 115),
        text_inverse: rgb(23, 23, 23),
        text_highlight: rgb(90, 170, 255),

        focus: rgb(115, 115, 115),
        active: rgb(64, 64, 64),
        disabled: rgb(64, 64, 64),

        overlay: Color::from_argb(160, 0, 0, 0),
        shadow: Color::from_argb(100, 0, 0, 0),
    };
    theme
}

/// egui-shadcn light
fn shadcn_light() -> Theme {
    let mut theme = light_theme();
    theme.name = "shadcn-light";
    theme.colors = ColorsSheet {
        primary: rgb(23, 23, 23),
        secondary: rgb(245, 245, 245),
        tertiary: rgb(64, 64, 64),

        success: rgb(0, 150, 60),
        warning: rgb(200, 140, 0),
        error: rgb(229, 72, 77),
        info: rgb(30, 120, 200),

        background: rgb(255, 255, 255),
        surface_primary: rgb(245, 245, 245),
        surface_secondary: Color::from_argb(204, 190, 190, 190),
        surface_tertiary: rgb(255, 255, 255),
        surface_inverse: rgb(23, 23, 23),
        surface_inverse_secondary: rgb(64, 64, 64),
        surface_inverse_tertiary: rgb(115, 115, 115),

        border: rgb(229, 229, 229),
        border_focus: rgb(161, 161, 161),
        border_disabled: rgb(229, 229, 229),

        text_primary: rgb(10, 10, 10),
        text_secondary: rgb(115, 115, 115),
        text_placeholder: rgb(161, 161, 161),
        text_inverse: rgb(250, 250, 250),
        text_highlight: rgb(0, 100, 220),

        focus: rgb(161, 161, 161),
        active: rgb(229, 229, 229),
        disabled: rgb(229, 229, 229),

        overlay: Color::from_argb(100, 0, 0, 0),
        shadow: Color::from_argb(40, 0, 0, 0),
    };
    theme
}

/// Sizes of the egui GUI: 14px text, compact buttons and inputs.
pub(crate) fn with_egui_metrics(mut theme: Theme) -> Theme {
    theme.set(
        "typography",
        TypographyThemePreference {
            title: Preference::Specific(24.0),
            subtitle: Preference::Specific(18.0),
            body: Preference::Specific(14.0),
            caption: Preference::Specific(12.0),
            overline: Preference::Specific(11.0),
        },
    );
    theme.set(
        "button_layout",
        ButtonLayoutThemePreference {
            padding: Preference::Specific(Gaps::new(5., 10., 5., 10.)),
            margin: Preference::Specific(Gaps::new_all(0.)),
            corner_radius: Preference::Specific(CornerRadius::new_all(6.)),
            width: Preference::Specific(Size::Inner),
            height: Preference::Specific(Size::Inner),
        },
    );
    theme.set(
        "input_layout",
        InputLayoutThemePreference {
            corner_radius: Preference::Specific(CornerRadius::new_all(6.)),
            padding: Preference::Specific(Gaps::new(6., 10., 6., 10.)),
        },
    );
    theme
}

/// Colors used for status messages.
pub(crate) mod colors {
    use freya::prelude::Color;

    pub(crate) const GREEN: Color = Color::from_rgb(80, 220, 120);
    pub(crate) const RED: Color = Color::from_rgb(244, 67, 54);
    pub(crate) const YELLOW: Color = Color::from_rgb(255, 193, 7);
    pub(crate) const LIGHT_BLUE: Color = Color::from_rgb(120, 220, 255);
}
