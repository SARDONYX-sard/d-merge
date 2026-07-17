//! Utility for centering content on both axes.

/// Centers content horizontally and vertically within all available space.
///
/// ```no_run
/// # egui::__run_test_ui(|ui| {
/// egui_shadcn::center(ui, |ui| {
///     ui.label("Centered!");
/// });
/// # });
/// ```
pub fn center<R>(
    ui: &mut egui::Ui,
    content: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    super::flex::Flex::row()
        .justify_center()
        .align_center()
        .w_full()
        .h_full()
        .show(ui, |f| f.ui(content).inner)
}
