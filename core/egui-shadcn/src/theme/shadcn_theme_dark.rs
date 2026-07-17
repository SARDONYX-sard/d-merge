//! Dark theme constructor with sRGB values derived from shadcn OKLCH tokens (Nova).

/// Creates the dark theme with colors matching shadcn/ui's Nova dark mode.
pub fn dark() -> super::shadcn_theme::ShadcnTheme {
    super::shadcn_theme::ShadcnTheme {
        background: egui::Color32::from_rgb(10, 10, 10),              // oklch(0.145)
        foreground: egui::Color32::from_rgb(250, 250, 250),           // oklch(0.985)
        card: egui::Color32::from_rgb(23, 23, 23),                    // oklch(0.205)
        card_foreground: egui::Color32::from_rgb(250, 250, 250),      // oklch(0.985)
        popover: egui::Color32::from_rgb(23, 23, 23),                 // oklch(0.205)
        popover_foreground: egui::Color32::from_rgb(250, 250, 250),   // oklch(0.985)
        primary: egui::Color32::from_rgb(229, 229, 229),              // oklch(0.922)
        primary_foreground: egui::Color32::from_rgb(23, 23, 23),      // oklch(0.205)
        secondary: egui::Color32::from_rgb(38, 38, 38),               // oklch(0.269)
        secondary_foreground: egui::Color32::from_rgb(250, 250, 250), // oklch(0.985)
        muted: egui::Color32::from_rgb(38, 38, 38),                   // oklch(0.269)
        muted_foreground: egui::Color32::from_rgb(161, 161, 161),     // oklch(0.708)
        accent: egui::Color32::from_rgb(64, 64, 64),                  // oklch(0.371)
        accent_foreground: egui::Color32::from_rgb(250, 250, 250),    // oklch(0.985)
        destructive: egui::Color32::from_rgb(255, 99, 105),           // oklch(0.704 0.191 22.216)
        destructive_foreground: egui::Color32::from_rgb(255, 255, 255),
        border: egui::Color32::from_rgba_unmultiplied(255, 255, 255, 26),  // oklch(1 0 0 / 10%)
        input: egui::Color32::from_rgba_unmultiplied(255, 255, 255, 38),   // oklch(1 0 0 / 15%)
        ring: egui::Color32::from_rgb(115, 115, 115),                 // oklch(0.556)
        radius: 10.0,
    }
}
