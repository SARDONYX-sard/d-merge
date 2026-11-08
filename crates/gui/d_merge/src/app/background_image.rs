use eframe::egui::{ColorImage, TextureOptions};

use crate::theme::EguiColorExt as _;

impl super::App {
    pub(crate) fn reload_background(&mut self, ctx: &egui::Context) {
        if self.settings.ui.background_image.path.is_empty() {
            return;
        }

        let image = match image::open(&self.settings.ui.background_image.path) {
            Ok(v) => v.to_rgba8(),
            Err(e) => {
                let err = format!("{e}");
                tracing::error!(err);
                self.notify.error(err);
                return;
            }
        };

        let size = [image.width() as usize, image.height() as usize];
        let color_image = ColorImage::from_rgba_unmultiplied(size, image.as_raw());
        self.bg_img_handle = Some(ctx.load_texture("background", color_image, TextureOptions::LINEAR));
        let blurred = crate::glass::blur_wallpaper(&image);
        self.bg_blur_handle = Some(ctx.load_texture("background_blurred", blurred, TextureOptions::LINEAR));
    }

    pub(crate) fn paint_background(&mut self, ctx: &egui::Context) {
        if !self.settings.ui.background_image.enabled {
            return;
        }

        if self.bg_img_handle.is_none() {
            self.reload_background(ctx);
        }
        // Glass: what is behind the glass is blurred.
        let texture = if self.glass().is_some() { &self.bg_blur_handle } else { &self.bg_img_handle };
        let Some(texture) = texture else {
            return;
        };

        let rect = ctx.content_rect();
        let tex = texture.size_vec2();
        let scale = (rect.width() / tex.x).max(rect.height() / tex.y);
        let size = tex * scale;

        let dest = egui::Rect::from_center_size(rect.center(), size);

        let painter = ctx.layer_painter(egui::LayerId::background());
        painter.image(
            texture.id(),
            dest,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
        // As over the blurred desktop.
        if let Some(preset) = self.glass() {
            painter.rect_filled(rect, 0.0, preset.visuals.panel_fill.to_egui_color32());
        }
    }
}
