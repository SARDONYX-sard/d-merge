//! log viewer

use std::sync::Arc;

use gui_shared::i18n::I18nKey;

use crate::{
    app::{App, background_image::paint_background, update_window_geometry},
    ui::shadcn::Button,
};

impl App {
    /// Renders the deferred log-viewer viewport.
    ///
    /// Opens as a separate OS window (egui deferred viewport) so it can be
    /// moved independently.  Visibility is toggled via the atomic bool
    /// [`App::show_log_window`]; closing the window sets it to `false`.
    pub(crate) fn ui_log_window(&self, ctx: &egui::Context) {
        use std::sync::atomic::Ordering;

        if !self.show_log_window.load(Ordering::Relaxed) {
            return;
        }

        let show_log_window = Arc::clone(&self.show_log_window);
        let log_lines = Arc::clone(&self.log_lines);
        let log_window_info = Arc::clone(&self.log_window_info);

        let (inner_size, position, maximized) = {
            let log_window_info = log_window_info.read().clone();
            let inner_size = Some(egui::Vec2::new(log_window_info.width, log_window_info.height));
            let position = Some(egui::Pos2::new(log_window_info.pos_x, log_window_info.pos_y));
            let maximized = Some(log_window_info.maximized);
            (inner_size, position, maximized)
        };

        // cloned self fields(to avoid ownership err & DeadLock)
        let bg_handle = self.bg_img_handle.clone();
        let log_viewer_id = egui::ViewportId::from_hash_of("log_viewer");
        let copy_button_name = self.i18n.t(I18nKey::EditorCopy).to_string();
        let clear_button_name = self.i18n.t(I18nKey::ClearButton).to_string();
        let frame = self.panel_frame();

        ctx.show_viewport_deferred(
            log_viewer_id,
            egui::ViewportBuilder {
                title: Some("Log viewer".to_string()),
                transparent: Some(true), // OK since egui 0.37
                position,
                maximized,
                inner_size,
                resizable: Some(true),
                ..Default::default()
            },
            move |ui, class| {
                debug_assert!(
                    class == egui::ViewportClass::Deferred,
                    "This egui backend doesn't support multiple viewports"
                );

                let ctx = &ui.ctx().clone();
                paint_background(&bg_handle, ctx);
                egui::CentralPanel::default().frame(frame).show(ui, |ui| {
                    update_window_geometry(ctx, log_viewer_id, &mut log_window_info.write());

                    ui.horizontal(|ui| {
                        if ui.add(Button::new(&clear_button_name)).clicked() {
                            log_lines.write().clear();
                        }
                        if ui.add(Button::new(&copy_button_name)).clicked() {
                            let text =
                                log_lines.read().iter().map(|log| log.raw.as_str()).collect::<Vec<_>>().join("\n");
                            ui.ctx().copy_text(text);
                        }
                    });

                    egui::ScrollArea::vertical().stick_to_bottom(true).show(ui, |ui| {
                        for line in log_lines.read().iter() {
                            ui.label(line.layout.clone());
                            ui.separator();
                        }
                    });
                });

                if ctx.input(|i| i.viewport().close_requested()) {
                    show_log_window.store(false, Ordering::Relaxed);
                }
            },
        );
    }
}
