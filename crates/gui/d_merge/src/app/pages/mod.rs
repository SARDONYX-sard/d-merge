//! Top-level pages, shown one at a time by the router (see [`super::router`]).
//!
//! - Patch: in this crate ([`patch`]), it needs most of [`App`].
//! - Convert / hkanno: separate crates (`page_convert`, `page_hkanno`) that only
//!   see what they are given in a `*PageCtx`. Their state lives in [`App`], because the router
//!   recreates the route object on every navigation.

mod patch;

use page_convert::ConvertPageCtx;
use page_hkanno::HkannoPageCtx;

use crate::{app::App, theme::frame_from_theme};

impl App {
    /// Background of page panels for the current theme.
    fn panel_frame(&self) -> egui::Frame {
        frame_from_theme(self.settings.ui.theme, self.theme_manager.current_bg_color())
    }

    pub(crate) fn ui_convert_page(&mut self, ui: &mut egui::Ui) {
        let ctx = ConvertPageCtx {
            panel_frame: self.panel_frame(),
            settings: &mut self.settings.convert,
            i18n: &self.i18n,
            notify: &mut self.notify,
            runtime: &self.async_rt,
        };
        self.convert.ui(ui, ctx);
    }

    pub(crate) fn ui_hkanno_page(&mut self, ui: &mut egui::Ui) {
        let ctx = HkannoPageCtx {
            panel_frame: self.panel_frame(),
            settings: &mut self.settings.hkanno,
            i18n: &self.i18n,
            notify: &mut self.notify,
        };
        self.hkanno.ui(ui, ctx);
    }
}
