/// A reusable confirmation dialog.
#[derive(Debug, Default)]
pub(crate) struct ConfirmDialog {
    open: bool,
    message: String,
    pub pending_action: Option<ConfirmAction>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConfirmAction {
    WriteI18nJson,
}

impl ConfirmDialog {
    pub(crate) fn open(&mut self, message: impl Into<String>, action: ConfirmAction) {
        self.message = message.into();
        self.pending_action = Some(action);
        self.open = true;
    }

    /// Renders the dialog. Calls `on_confirm` with the pending action if the user confirms.
    pub(crate) fn show(&mut self, ctx: &egui::Context, on_confirm: impl FnOnce(ConfirmAction)) {
        if !self.open {
            return;
        }

        let mut open = self.open;
        egui::Window::new("Confirm")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(4.0);
                    ui.label(&self.message);
                    ui.add_space(12.0);

                    egui::TopBottomPanel::bottom("bottom_panel").show_inside(ui, |ui| {
                        ui.columns(2, |columns| {
                            columns[0].allocate_ui_with_layout(
                                egui::Vec2::ZERO,
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.add_space(40.);
                                    if ui.button("OK").clicked() {
                                        self.open = false;
                                        if let Some(action) = self.pending_action.take() {
                                            on_confirm(action);
                                        }
                                    }
                                },
                            );
                            columns[1].allocate_ui_with_layout(
                                egui::Vec2::ZERO,
                                egui::Layout::left_to_right(egui::Align::Center),
                                |ui| {
                                    if ui.button("Cancel").clicked() {
                                        self.open = false;
                                        self.pending_action = None;
                                    }
                                    ui.add_space(40.);
                                },
                            );
                        });
                    });
                });
            });

        // Closed via the × button
        if !open {
            self.open = false;
            self.pending_action = None;
        }
    }
}
