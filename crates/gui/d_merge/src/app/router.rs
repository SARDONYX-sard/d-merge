//! Page routing with [`egui_router`].
//!
//! # Why a wrapper ([`Root`])?
//! `EguiRouter::ui(ui, &mut state)` needs `&mut App` as its state, so the router cannot be a field
//! of `App` itself (that would borrow `App` twice). `Root` owns both side by side.
//!
//! # Frame layout
//! ```text
//! ┌ page nav (Patch | Convert | hkanno Editor | Theme) ───────┐  <- Root
//! │                                                           │
//! │   current page (drawn by the router)                      │  <- App::ui_*_page
//! │                                                           │
//! ├ notification bar ─────────────────────────────────────────┤  <- Root (shared by all pages)
//! └───────────────────────────────────────────────────────────┘
//! + floating windows (log viewer, help)                           <- Root
//! ```

use egui_router::{EguiRouter, Route, TransitionConfig};
use gui_shared::{i18n::I18nKey, settings::ui::Page};

use crate::{app::App, theme::themed_top_bottom_panel, ui::shadcn::Button};

/// The `eframe::App` given to eframe: the [`App`] state + the router showing its pages.
pub(crate) struct Root {
    app: App,
    router: EguiRouter<App>,
}

impl Root {
    pub(crate) fn new(mut app: App) -> Self {
        let start_page = app.settings.ui.page;

        let router = EguiRouter::builder()
            // Pages are tool panels, not a phone UI: switch instantly.
            // (A transition would also draw two pages at once for a few frames.)
            .transition(TransitionConfig::none().with_duration(0.0))
            .route(Page::Patch.path(), patch_route)
            .route(Page::Convert.path(), convert_route)
            .route(Page::Hkanno.path(), hkanno_route)
            .default_path(start_page.path())
            .build(&mut app);

        Self { app, router }
    }

    /// Switches to `page` and remembers it for the next startup.
    fn go(&mut self, page: Page) {
        if self.app.settings.ui.page == page {
            return;
        }
        self.app.settings.ui.page = page;

        // `replace` (not `navigate`): tabs should not build up a back-history.
        if let Err(err) = self.router.replace(&mut self.app, page.path()) {
            tracing::error!("Failed to navigate to {}: {err}", page.path());
        }
    }

    /// Tab bar at the very top.
    fn ui_page_nav(&mut self, ui: &mut egui::Ui) {
        let panel = themed_top_bottom_panel(
            egui::Panel::top("page_nav"),
            self.app.settings.ui.theme,
            self.app.theme_manager.current_bg_color(),
        );

        let mut clicked = None;
        // `[Patch] [Convert] [hkanno]  <- grow ->  Theme: [Dark ▼]`
        panel.show(ui, |ui| {
            let flex =
                egui_flex::Flex::horizontal().w_full().align_items(egui_flex::FlexAlign::Center);
            flex.show(ui, |flex| {
                for page in Page::ALL {
                    let label = self.app.i18n.t(match page {
                        Page::Patch => I18nKey::PagePatch,
                        Page::Convert => I18nKey::PageConvert,
                        Page::Hkanno => I18nKey::PageHkanno,
                    });
                    let selected = self.app.settings.ui.page == page;
                    let button = Button::selectable(selected, label);
                    if flex.add_widget(egui_flex::item(), button).inner.clicked() {
                        clicked = Some(page);
                    }
                }
                flex.grow();
                flex.add_ui(egui_flex::item(), |ui| {
                    ui.horizontal(|ui| self.app.ui_theme_box(ui));
                });
            });
        });

        if let Some(page) = clicked {
            self.go(page);
        }
    }
}

impl eframe::App for Root {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        self.app.frame_begin(&ctx);

        self.ui_page_nav(ui);
        // NOTE: Bottom panels must be registered before the central area.
        self.app.ui_notification(ui);

        egui::CentralPanel::default().frame(egui::Frame::NONE).show(ui, |ui| {
            self.router.ui(ui, &mut self.app);
        });

        self.app.frame_end(&ctx);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.app.on_exit();
    }
}

// ── Routes ───────────────────────────────────────────────────────────────────
// Each route only forwards to the page method; page state lives in `App`.

fn patch_route() -> impl Route<App> {
    |ui: &mut egui::Ui, app: &mut App| app.ui_patch_page(ui)
}

fn convert_route() -> impl Route<App> {
    |ui: &mut egui::Ui, app: &mut App| app.ui_convert_page(ui)
}

fn hkanno_route() -> impl Route<App> {
    |ui: &mut egui::Ui, app: &mut App| app.ui_hkanno_page(ui)
}
