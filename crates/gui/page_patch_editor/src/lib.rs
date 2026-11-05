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

use self::{crc::CrcTool, patch::PatchTool};

/// What the page borrows from the app for one frame.
#[derive(Debug)]
pub struct PatchEditorPageCtx<'a> {
    pub i18n: &'a I18nMap,
    /// The app's notification bar.
    pub notify: &'a mut Notification,
    /// `settings.behavior.template_dir`: the templates the patch tool lists.
    pub template_dir: &'a str,
    /// `settings.behavior.parser_mode`: patches are checked as d-merge will read them.
    pub parser_mode: pages::patch::behavior::ParserMode,
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

/// What the window of the user patch asks of the app.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct UserPatchOutcome {
    /// Show the Patch Editor: an object was opened in it.
    pub open_editor: bool,
}

impl PatchEditorPage {
    /// `Use this` in a conflict: the value goes into the user patch and the mod is written, without
    /// the editor.
    ///
    /// # Errors
    /// The value could not be taken in; the message says why.
    pub fn resolve_now(
        &mut self,
        request: &gui_shared::patch::ResolveRequest,
        template_dir: &str,
        i18n: &I18nMap,
    ) -> Result<(), String> {
        self.patch.resolve_now(request, template_dir, i18n.t(I18nKey::NemesisUserPatchName))
    }

    /// A mod was written since the last call, so that the conflict check can follow.
    pub const fn take_mod_written(&mut self) -> bool {
        self.patch.take_mod_written()
    }

    /// Opens or closes the window of the user patch.
    pub fn toggle_user_patch(&mut self) {
        self.patch.toggle_user_patch();
    }

    /// Draws the window of the user patch, on any page.
    pub fn user_patch_window(
        &mut self,
        ctx: &egui::Context,
        i18n: &I18nMap,
        notify: &mut Notification,
    ) -> UserPatchOutcome {
        let outcome = self.patch.user_patch_window(ctx, i18n, notify);
        if outcome.open_editor {
            self.tool = Tool::Patch;
        }
        outcome
    }

    /// Takes a conflict's value into the user patch, and shows it. See [`patch`]'s `resolve`.
    pub fn open_resolution(&mut self, request: gui_shared::patch::ResolveRequest) {
        self.tool = Tool::Patch;
        self.patch.open_resolution(request);
    }

    /// Draws the page.
    pub fn ui(&mut self, ui: &mut egui::Ui, ctx: PatchEditorPageCtx<'_>) {
        let PatchEditorPageCtx { i18n, notify, template_dir, parser_mode, settings, editor, panel_frame } = ctx;
        self.patch.set_parser_mode(parser_mode);
        egui::CentralPanel::default().frame(panel_frame).show(ui, |ui| {
            // Always-visible scroll bars (like the hkanno page): you see where you are in a long object.
            ui.spacing_mut().scroll = egui::style::ScrollStyle::solid();
            ui.horizontal(|ui| {
                for (tool, label) in [(Tool::Patch, I18nKey::NemesisPatchTab), (Tool::Crc, I18nKey::NemesisCrcTab)] {
                    if ui.add(Button::selectable(self.tool == tool, i18n.t(label))).clicked() {
                        self.tool = tool;
                    }
                }
            });
            ui.separator();
            match self.tool {
                Tool::Patch => self.patch.ui(ui, i18n, notify, template_dir, settings, editor),
                Tool::Crc => self.crc.ui(ui, i18n),
            }
        });
    }
}
