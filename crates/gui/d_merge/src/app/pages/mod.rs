//! Top-level pages, shown one at a time by the router (see [`super::router`]).
//!
//! Each page is a separate crate (`page_patch`, `page_convert`, `page_hkanno`, `page_patch_editor`)
//! that only sees what it is given in a `*PageCtx`. Their state lives in [`App`], because the
//! router recreates the route object on every navigation.

use page_convert::ConvertPageCtx;
use page_hkanno::HkannoPageCtx;
use page_patch::PatchPageCtx;
use page_patch_editor::PatchEditorPageCtx;

use crate::{app::App, theme::frame_from_theme};

impl App {
    /// Background of page panels for the current theme.
    pub(super) fn panel_frame(&self) -> egui::Frame {
        frame_from_theme(self.settings.ui.theme, self.theme_manager.current_bg_color())
    }

    pub(crate) fn ui_patch_page(&mut self, ui: &mut egui::Ui) {
        let ctx = PatchPageCtx {
            panel_frame: self.panel_frame(),
            settings: &mut self.settings.pages.patch,
            log: &mut self.settings.log,
            i18n: &self.i18n,
            notify: &mut self.notify,
            runtime: &self.async_rt,
            show_log_window: &self.show_log_window,
        };
        self.patch.ui(ui, ctx);
        // A conflict's value to keep: on to the user patch.
        if let Some(request) = self.patch.take_resolve() {
            self.patch_editor.open_resolution(request);
            self.pending_page = Some(gui_shared::settings::ui::Page::PatchEditor);
        }
    }

    pub(crate) fn ui_convert_page(&mut self, ui: &mut egui::Ui) {
        let ctx = ConvertPageCtx {
            panel_frame: self.panel_frame(),
            settings: &mut self.settings.pages.convert,
            i18n: &self.i18n,
            notify: &mut self.notify,
            runtime: &self.async_rt,
        };
        self.convert.ui(ui, ctx);
    }

    pub(crate) fn ui_hkanno_page(&mut self, ui: &mut egui::Ui) {
        let ctx = HkannoPageCtx {
            panel_frame: self.panel_frame(),
            settings: &mut self.settings.pages.hkanno,
            lsp_settings: &mut self.settings.ui.lsp,
            i18n: &self.i18n,
            notify: &mut self.notify,
        };
        self.hkanno.ui(ui, ctx);
    }

    pub(crate) fn ui_patch_editor_page(&mut self, ui: &mut egui::Ui) {
        let ctx = PatchEditorPageCtx {
            panel_frame: self.panel_frame(),
            i18n: &self.i18n,
            notify: &mut self.notify,
            template_dir: &self.settings.pages.patch.behavior.template_dir,
            parser_mode: self.settings.pages.patch.behavior.parser_mode,
            settings: &mut self.settings.pages.patch_editor,
            editor: &mut self.settings.ui.lsp,
        };
        self.patch_editor.ui(ui, ctx);
    }
}
