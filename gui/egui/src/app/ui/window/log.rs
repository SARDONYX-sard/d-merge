//! log viewer

use std::sync::Arc;

use d_merge_gui_shared::i18n::I18nKey;

use crate::{
    app::{App, update_window_geometry},
    ui::shadcn_compat::button,
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

        let (inner_size, position) = {
            let log_window_info = log_window_info.read().clone();
            let inner_size = Some(egui::Vec2::new(log_window_info.width, log_window_info.height));
            let position = Some(egui::Pos2::new(log_window_info.pos_x, log_window_info.pos_y));
            (inner_size, position)
        };

        let log_viewer_id = egui::ViewportId::from_hash_of("log_viewer");
        let clear_button_name = self.i18n.t(I18nKey::ClearButton).to_string();

        ctx.show_viewport_deferred(
            log_viewer_id,
            egui::ViewportBuilder {
                title: Some("Log viewer".to_string()),
                transparent: Some(false), // Cannot now: https://github.com/emilk/egui/issues/3632
                position,
                maximized: Some(self.settings.log.window.maximized),
                inner_size,
                resizable: Some(true),
                ..Default::default()
            },
            move |ctx, class| {
                debug_assert!(
                    class == egui::ViewportClass::Deferred,
                    "This egui backend doesn't support multiple viewports"
                );

                egui::CentralPanel::default().frame(egui::Frame::new()).show(ctx, |ui| {
                    update_window_geometry(ctx, log_viewer_id, &mut log_window_info.write());

                    ui.horizontal(|ui| {
                        if ui.add(button(&clear_button_name)).clicked() {
                            log_lines.write().clear();
                        }
                        if ui.add(button("Copy")).clicked() {
                            let text = log_lines
                                .read()
                                .iter()
                                .map(|log| log.raw.as_str())
                                .collect::<Vec<_>>()
                                .join("\n");
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
