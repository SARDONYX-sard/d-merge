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
            self.take_resolution(request);
        }
        if self.patch.take_user_patch_toggle() {
            self.patch_editor.toggle_user_patch();
        }
    }

    /// `Use this` decides here and now; `Edit...` goes on in the window of the user patch.
    fn take_resolution(&mut self, request: gui_shared::patch::ResolveRequest) {
        let paths = patch_paths(&self.settings.pages.patch);
        if request.edit {
            self.patch_editor.open_resolution(request, paths, &mut self.notify, &self.i18n);
            return;
        }
        match self.patch_editor.resolve_now(&request, paths, &mut self.notify, &self.i18n) {
            Ok(()) => self.notify.success(self.i18n.t(gui_shared::i18n::I18nKey::ConflictResolved)),
            Err(err) => {
                self.notify.error(format!("{}: {err}", self.i18n.t(gui_shared::i18n::I18nKey::NemesisResolveFailed)));
            }
        }
    }

    /// The window of the user patch: it stays when another page is shown.
    pub(crate) fn ui_user_patch_window(&mut self, ctx: &egui::Context) {
        self.patch_editor.user_patch_window(
            ctx,
            &self.i18n,
            &mut self.notify,
            patch_paths(&self.settings.pages.patch),
            (&mut self.settings.pages.patch_editor, &mut self.settings.ui.lsp),
            &mut self.keys,
        );
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
            keys: &mut self.keys,
        };
        self.hkanno.ui(ui, ctx);
    }

    pub(crate) fn ui_patch_editor_page(&mut self, ui: &mut egui::Ui) {
        let ctx = PatchEditorPageCtx {
            panel_frame: self.panel_frame(),
            i18n: &self.i18n,
            notify: &mut self.notify,
            paths: patch_paths(&self.settings.pages.patch),
            settings: &mut self.settings.pages.patch_editor,
            editor: &mut self.settings.ui.lsp,
            keys: &mut self.keys,
        };
        self.patch_editor.ui(ui, ctx);
    }
}

/// What the Patch Editor needs of the patch page's settings.
fn patch_paths(patch: &gui_shared::settings::pages::patch::PatchSettings) -> page_patch_editor::PatchPaths<'_> {
    page_patch_editor::PatchPaths {
        patch_output_dir: patch.current_output_dir(),
        template_dir: &patch.behavior.template_dir,
        parser_mode: patch.behavior.parser_mode,
    }
}
