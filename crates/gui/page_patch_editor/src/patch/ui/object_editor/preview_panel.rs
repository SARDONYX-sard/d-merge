//! The patch preview beside the editor: what the object becomes in the patch, checked by d-merge's
//! parser.
//!
//! ```text
//! Patch preview  ▲ ▼ 2/5  ✔                 ⌖     <- ✔ / ✖ the parser; ⌖ follows the cursor
//!   <!-- MOD_CODE ~mymod~ OPEN -->
//!   ...
//! ```

use egui_widgets::shadcn::{Button, Icon};
use gui_shared::i18n::{I18nKey, I18nMap};

use crate::patch::{PatchTool, ui::preview::PreviewEvent};

impl PatchTool {
    /// Draws the panel. Returns what its lines asked for (revert a block, go to a line).
    pub(super) fn preview_panel(&mut self, ui: &mut egui::Ui, i18n: &I18nMap) -> Option<PreviewEvent> {
        let panel_id = if self.is_user_patch { "user_patch_preview" } else { "nemesis_preview" };
        let panel = egui::Panel::right(panel_id).default_size(520.0).resizable(true);

        panel
            .show(ui, |ui| {
                self.preview_header(ui, i18n)?;
                let view = self.pane.preview_view.as_mut()?;
                super::super::preview::lines_ui(view, ui, i18n.t(I18nKey::NemesisRevertHunkHover))
            })
            .inner
    }

    /// `Patch preview ▲ ▼ 2/5 ✔ ... ⌖`. `None`: there is no preview yet.
    fn preview_header(&mut self, ui: &mut egui::Ui, i18n: &I18nMap) -> Option<()> {
        let (patch, check) = self.pane.preview.as_ref()?;
        let is_pandora = self.pane.preview_view.as_ref().is_some_and(|view| view.is_pandora());

        ui.horizontal(|ui| {
            ui.strong(i18n.t(I18nKey::NemesisPatchPreview));
            if let Some(view) = &mut self.pane.preview_view {
                super::super::preview::nav_ui(view, ui, i18n.t(I18nKey::NemesisJumpDiffHover));
            }

            match check {
                _ if patch.is_none() => {
                    ui.weak(i18n.t(I18nKey::NemesisNoChange));
                }
                Ok(()) => {
                    let ok = if is_pandora { I18nKey::NemesisPandoraOpsOk } else { I18nKey::NemesisParserOk };
                    ui.colored_label(egui::Color32::from_rgb(100, 200, 120), "✔").on_hover_text(i18n.t(ok));
                }
                // The details are in the Problems panel: a long text here would widen this panel.
                Err(err) => {
                    ui.colored_label(ui.visuals().error_fg_color, "✖").on_hover_text(err);
                }
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let follow = Button::selectable(self.pane.follow_preview, "").icon(Icon::Crosshair).small();
                if ui.add(follow).on_hover_text(i18n.t(I18nKey::NemesisFollowCursor)).clicked() {
                    self.pane.follow_preview = !self.pane.follow_preview;
                }
            });
        });
        Some(())
    }
}
