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

use super::preview::PreviewEvent;
use crate::patch::editor_area::EditorPane;

impl EditorPane {
    /// Draws the preview in its tab (see `super::dock`). Returns what its lines asked for (revert a
    /// block, go to a line).
    pub(super) fn preview_tab(&mut self, ui: &mut egui::Ui, i18n: &I18nMap) -> Option<PreviewEvent> {
        // Kept to the tab's width: see `egui_widgets::contained`.
        egui_widgets::contained(ui, |ui| {
            self.preview_header(ui, i18n)?;
            // The animation data colored as in its editor; the rest as XML.
            let anim = crate::patch::anim::lines::AnimDataLanguage {
                roles: &self.preview_roles,
                cursor_line: 0,
                trigger_names: &[],
            };
            let language: &dyn egui_code_editor::Language =
                if self.preview_roles.is_empty() { &egui_code_editor::languages::Xml } else { &anim };
            let view = self.preview_view.as_mut()?;
            super::preview::lines_ui(view, ui, (i18n.t(I18nKey::PatchPreviewRevertHunkHover), language))
        })
    }

    /// `Patch preview ▲ ▼ 2/5 ✔ ... ⌖`. `None`: there is no preview yet.
    fn preview_header(&mut self, ui: &mut egui::Ui, i18n: &I18nMap) -> Option<()> {
        let (patch, check) = self.preview.as_ref()?;
        let is_pandora = self.preview_view.as_ref().is_some_and(|view| view.is_pandora());

        ui.horizontal(|ui| {
            ui.strong(i18n.t(I18nKey::PatchPreviewTitle));
            if let Some(view) = &mut self.preview_view {
                super::preview::nav_ui(view, ui, i18n.t(I18nKey::JumpDiffHover));
            }

            match check {
                _ if patch.is_none() => {
                    ui.weak(i18n.t(I18nKey::NoChange));
                }
                Ok(()) => {
                    let ok = if is_pandora { I18nKey::PatchPreviewPandoraOpsOk } else { I18nKey::PatchPreviewParserOk };
                    ui.colored_label(egui::Color32::from_rgb(100, 200, 120), "✔").on_hover_text(i18n.t(ok));
                }
                // The details are in the Problems panel: a long text here would widen this panel.
                Err(err) => {
                    ui.colored_label(ui.visuals().error_fg_color, "✖").on_hover_text(err.to_string());
                }
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let follow = Button::selectable(self.follow_preview, "").icon(Icon::Crosshair).small();
                if ui.add(follow).on_hover_text(i18n.t(I18nKey::PreviewFollowCursor)).clicked() {
                    self.follow_preview = !self.follow_preview;
                }
            });
        });
        Some(())
    }
}
