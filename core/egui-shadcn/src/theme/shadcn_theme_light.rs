//! Light theme constructor with sRGB values derived from shadcn OKLCH tokens (Nova).

/// Creates the light theme with colors matching shadcn/ui's Nova light mode.
pub fn light() -> super::shadcn_theme::ShadcnTheme {
    super::shadcn_theme::ShadcnTheme {
        background: egui::Color32::from_rgb(255, 255, 255),           // oklch(1.0)
        foreground: egui::Color32::from_rgb(10, 10, 10),              // oklch(0.145)
        card: egui::Color32::from_rgb(255, 255, 255),                 // oklch(1.0)
        card_foreground: egui::Color32::from_rgb(10, 10, 10),         // oklch(0.145)
        popover: egui::Color32::from_rgb(255, 255, 255),              // oklch(1.0)
        popover_foreground: egui::Color32::from_rgb(10, 10, 10),      // oklch(0.145)
        primary: egui::Color32::from_rgb(23, 23, 23),                 // oklch(0.205)
        primary_foreground: egui::Color32::from_rgb(250, 250, 250),   // oklch(0.985)
        secondary: egui::Color32::from_rgb(245, 245, 245),            // oklch(0.97)
        secondary_foreground: egui::Color32::from_rgb(23, 23, 23),    // oklch(0.205)
        muted: egui::Color32::from_rgb(245, 245, 245),                // oklch(0.97)
        muted_foreground: egui::Color32::from_rgb(115, 115, 115),     // oklch(0.556)
        accent: egui::Color32::from_rgb(245, 245, 245),               // oklch(0.97)
        accent_foreground: egui::Color32::from_rgb(23, 23, 23),       // oklch(0.205)
        destructive: egui::Color32::from_rgb(229, 72, 77),            // oklch(0.577 0.245 27.325)
        destructive_foreground: egui::Color32::from_rgb(255, 255, 255),// white on red
        border: egui::Color32::from_rgb(229, 229, 229),               // oklch(0.922)
        input: egui::Color32::from_rgb(229, 229, 229),                // oklch(0.922)
        ring: egui::Color32::from_rgb(161, 161, 161),                 // oklch(0.708)
        radius: 10.0,
    }
}
