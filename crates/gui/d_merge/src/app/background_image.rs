use eframe::egui::{ColorImage, TextureOptions};

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
    }

    pub(crate) fn paint_background(&mut self, ctx: &egui::Context) {
        if !self.settings.ui.background_image.enabled {
            return;
        }
        if self.bg_img_handle.is_none() {
            self.reload_background(ctx);
        }
        paint_background(&self.bg_img_handle, ctx);
    }
}

pub(crate) fn paint_background(handle: &Option<egui::TextureHandle>, ctx: &egui::Context) {
    let Some(handle) = handle else {
        return;
    };

    let dest = {
        let rect = ctx.content_rect();
        let tex = handle.size_vec2();
        let scale = (rect.width() / tex.x).max(rect.height() / tex.y);
        let size = tex * scale;
        egui::Rect::from_center_size(rect.center(), size)
    };

    ctx.layer_painter(egui::LayerId::background()).image(
        handle.id(),
        dest,
        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        egui::Color32::WHITE,
    );
}
