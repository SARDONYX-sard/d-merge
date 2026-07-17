//! Show method for AspectRatio — renders content inside a ratio-constrained container.

impl super::aspect_ratio::AspectRatio {
    /// Renders content inside a container with the enforced aspect ratio.
    pub fn show(
        self,
        ui: &mut egui::Ui,
        content: impl FnOnce(&mut egui::Ui),
    ) -> egui::Response {
        let available_width = ui.available_width();
        let height = available_width / self.ratio;

        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(available_width, height),
            egui::Sense::hover(),
        );

        let mut child_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(rect)
                .layout(egui::Layout::top_down(egui::Align::LEFT)),
        );
        content(&mut child_ui);

        response
    }
}
