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
use gui_shared::i18n::{I18nKey, I18nMap};

use self::{crc::CrcTool, patch::PatchTool};

/// What the page borrows from the app for one frame.
#[derive(Debug)]
pub struct PatchEditorPageCtx<'a> {
    pub i18n: &'a I18nMap,
    /// The app's notification bar.
    pub notify: &'a mut Notification,
    /// `settings.behavior.template_dir`: the templates the patch tool lists.
    pub template_dir: &'a str,
    /// Persisted page state (`settings.patch_editor`).
    pub settings: &'a mut gui_shared::settings::patch_editor::PatchEditorSettings,
    /// Editor settings shared with the hkanno editor: Normal / Vim keys, language features.
    pub editor: &'a mut gui_shared::settings::hkanno::HkannoSettings,
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
    /// Draws the page.
    pub fn ui(&mut self, ui: &mut egui::Ui, ctx: PatchEditorPageCtx<'_>) {
        let PatchEditorPageCtx { i18n, notify, template_dir, settings, editor, panel_frame } =
            ctx;
        egui::CentralPanel::default().frame(panel_frame).show(ui, |ui| {
            // Always-visible scroll bars (like the hkanno page): you see where you are in a long object.
            ui.spacing_mut().scroll = egui::style::ScrollStyle::solid();
            ui.horizontal(|ui| {
                for (tool, label) in
                    [(Tool::Patch, I18nKey::NemesisPatchTab), (Tool::Crc, I18nKey::NemesisCrcTab)]
                {
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
