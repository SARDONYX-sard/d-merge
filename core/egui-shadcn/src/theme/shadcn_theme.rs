//! Core theme struct holding all shadcn color tokens.

/// All color tokens from the shadcn/ui design system, pre-converted to sRGB.
#[derive(Debug, Clone)]
pub struct ShadcnTheme {
    pub background: egui::Color32,
    pub foreground: egui::Color32,
    pub card: egui::Color32,
    pub card_foreground: egui::Color32,
    pub popover: egui::Color32,
    pub popover_foreground: egui::Color32,
    pub primary: egui::Color32,
    pub primary_foreground: egui::Color32,
    pub secondary: egui::Color32,
    pub secondary_foreground: egui::Color32,
    pub muted: egui::Color32,
    pub muted_foreground: egui::Color32,
    pub accent: egui::Color32,
    pub accent_foreground: egui::Color32,
    pub destructive: egui::Color32,
    pub destructive_foreground: egui::Color32,
    pub border: egui::Color32,
    pub input: egui::Color32,
    pub ring: egui::Color32,
    pub radius: f32,
}

impl Default for ShadcnTheme {
    fn default() -> Self {
        super::shadcn_theme_light::light()
    }
}
