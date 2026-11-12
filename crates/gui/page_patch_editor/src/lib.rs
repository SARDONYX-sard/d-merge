//! Patch Editor page: tools for making behavior patch mods (Nemesis now, Pandora planned).
//!
//! ```text
//! [Patch] [CRC32]
//! ─────────────────────────────────────────────
//! (the selected tool: see `patch`, `crc`)
//! ```

mod crc;
mod patch;

use egui_widgets::{notify::Notification, shadcn::Button};
use gui_shared::{
    i18n::{I18nKey, I18nMap},
    settings::{pages, ui},
};

pub use self::patch::PatchPaths;
use self::{crc::CrcTool, patch::PatchTool};

/// What the page borrows from the app for one frame.
#[derive(Debug)]
pub struct PatchEditorPageCtx<'a> {
    pub i18n: &'a I18nMap,
    /// The app's notification bar.
    pub notify: &'a mut Notification,
    /// What the patch tool needs of the patch page's settings.
    pub paths: PatchPaths<'a>,
    /// Persisted page state (`settings.patch_editor`).
    pub settings: &'a mut pages::patch_editor::PatchEditorSettings,
    /// Editor settings shared with the hkanno editor: Normal / Vim keys, language features.
    pub editor: &'a mut ui::editor::LspOptions,
    /// Frame (background) of the page's panels, from the app theme.
    pub panel_frame: egui::Frame,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Tool {
    #[default]
    Patch,
    Crc,
}

/// Page state. Keep one in the app (the router recreates the route on every navigation).
#[derive(Debug, Default)]
pub struct PatchEditorPage {
    tool: Tool,
    patch: PatchTool,
    crc: CrcTool,
}

impl PatchEditorPage {
    /// `Use this` in a conflict: the value goes into the user patch and the mod is written.
    ///
    /// # Errors
    /// The value could not be taken in; the message says why.
    #[inline]
    pub fn resolve_now(
        &mut self,
        request: &gui_shared::patch::ResolveRequest,
        paths: PatchPaths<'_>,
        notify: &mut Notification,
        i18n: &I18nMap,
    ) -> Result<(), String> {
        self.patch.resolve_now(request, paths, notify, i18n)
    }

    /// A mod was written since the last call, so that the conflict check can follow.
    #[inline]
    pub const fn take_mod_written(&mut self) -> bool {
        self.patch.take_mod_written()
    }

    /// Opens or closes the window of the user patch.
    #[inline]
    pub const fn toggle_user_patch(&mut self) {
        self.patch.toggle_user_patch();
    }

    /// Draws the window of the user patch, on any page.
    #[inline]
    pub fn user_patch_window(
        &mut self,
        ctx: &egui::Context,
        i18n: &I18nMap,
        notify: &mut Notification,
        paths: PatchPaths<'_>,
        settings: &mut pages::patch_editor::PatchEditorSettings,
        editor: &mut ui::editor::LspOptions,
    ) {
        self.patch.user_patch_window(ctx, i18n, notify, paths, settings, editor);
    }

    /// `Edit...` in a conflict: the value goes into the user patch, and its window shows it.
    #[inline]
    pub fn open_resolution(
        &mut self,
        request: gui_shared::patch::ResolveRequest,
        paths: PatchPaths<'_>,
        notify: &mut Notification,
        i18n: &I18nMap,
    ) {
        self.patch.open_resolution(request, paths, notify, i18n);
    }

    /// Draws the page.
    pub fn ui(&mut self, ui: &mut egui::Ui, ctx: PatchEditorPageCtx<'_>) {
        let PatchEditorPageCtx { i18n, notify, paths, settings, editor, panel_frame } = ctx;
        self.patch.set_parser_mode(paths.parser_mode);
        egui::CentralPanel::default().frame(panel_frame).show(ui, |ui| {
            // Always-visible scroll bars (like the hkanno page): you see where you are in a long object.
            ui.spacing_mut().scroll = egui::style::ScrollStyle::solid();
            ui.horizontal(|ui| {
                for (tool, label) in [(Tool::Patch, I18nKey::PagePatch), (Tool::Crc, I18nKey::NemesisCrcTab)] {
                    if ui.add(Button::selectable(self.tool == tool, i18n.t(label))).clicked() {
                        self.tool = tool;
                    }
                }
            });
            ui.separator();
            match self.tool {
                Tool::Patch => self.patch.ui(ui, i18n, notify, paths, settings, editor),
                Tool::Crc => self.crc.ui(ui, i18n),
            }
        });
    }
}
