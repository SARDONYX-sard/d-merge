//! Patch page: the original single-page UI (options, paths, mod list, patch button).

use crate::app::App;

impl App {
    /// Draws the patch page into the router's page area.
    ///
    /// Panel order matters (egui lays panels out in registration order):
    /// top panels -> bottom panel -> central mod list.
    pub(crate) fn ui_patch_page(&mut self, ui: &mut egui::Ui) {
        // ── Top panels (outermost -> innermost) ────────────────────────────────
        self.ui_top_options(ui);
        self.ui_paths(ui);

        // ── Bottom panel ───────────────────────────────────────────────────────
        // NOTE: The notification bar is global, so `super::super::router` draws it below every page.
        self.ui_bottom_panel(ui);

        // ── Central panel (must be last among panels) ──────────────────────────
        self.ui_mod_list(ui);
    }
}
