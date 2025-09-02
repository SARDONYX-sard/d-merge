use std::path::Path;

use egui::{Context, FontData, FontDefinitions, FontFamily};

/// Setup custom system fonts.
pub fn setup_custom_fonts<A>(ctx: &Context, font_path: Option<A>)
where
    A: AsRef<Path>,
{
    let font_path = font_path
        .as_ref()
        .map_or_else(|| Path::new("c:/Windows/Fonts/msyh.ttc"), |p| p.as_ref());
    __setup_custom_fonts(ctx, font_path);
}

fn __setup_custom_fonts(ctx: &Context, font_path: &Path) {
    let mut fonts = FontDefinitions::default();

    match std::fs::read(font_path) {
        Ok(font) => {
            fonts
                .font_data
                .insert("sys_font".to_owned(), FontData::from_owned(font).into());
            fonts
                .families
                .entry(FontFamily::Proportional)
                .or_default()
                .insert(0, "sys_font".to_owned());
            fonts
                .families
                .entry(FontFamily::Monospace)
                .or_default()
                .insert(0, "sys_font".to_owned());
        }
        Err(_err) => {
            #[cfg(target_os = "windows")]
            tracing::error!("Failed to load font from {}: {_err}", font_path.display());
        }
    }

    ctx.set_fonts(fonts);
}
