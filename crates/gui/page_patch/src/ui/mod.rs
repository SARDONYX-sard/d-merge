//! Drawing of the patch page. The logic is in the modules next to this directory.

mod bottom_panel;
mod dnd_table;
mod label_ext;
mod mod_list;
mod options;

use crate::{PatchPage, PatchPageCtx};

/// The page and what it borrowed for this frame, for the panels to draw from.
struct PatchView<'a> {
    page: &'a mut PatchPage,
    ctx: PatchPageCtx<'a>,
}

impl PatchPage {
    /// Draws the page into the router's page area.
    ///
    /// Panel order matters (egui lays panels out in registration order):
    /// top panels -> bottom panel -> central mod list.
    pub fn ui(&mut self, ui: &mut egui::Ui, ctx: PatchPageCtx<'_>) {
        // Here, not in `poll`: the shortcut must not fire while typing on another page.
        if ui.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::R)) {
            self.fetch_mods(ctx.settings);
        }

        let mut view = PatchView { page: self, ctx };
        view.ui_top_options(ui);
        view.ui_paths(ui);
        view.ui_bottom_panel(ui);
        view.ui_mod_list(ui); // the central panel must be the last one
        view.page.first_frame = false;
    }
}
