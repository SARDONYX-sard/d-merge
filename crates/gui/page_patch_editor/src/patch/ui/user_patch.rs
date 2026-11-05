//! The window of the whole user patch. What it is is in [`crate::patch::user_patch`].
//!
//! ```text
//! ┌ User Patch ───────────────────────────────────────────────┐
//! │ 0_master                                                  │
//! │   #0106 hkbModifierList GetUpModList                      │  <- a click shows it below
//! │   #0241 hkbClipGenerator                                  │
//! │ ───────────────────────────────────────────────────────── │
//! │ [Open in Editor] [Remove]                                 │
//! │ ↶ <!-- MOD_CODE ~upatch~ OPEN -->                         │  <- `↶`: that change goes
//! │   <hkparam name="enable">false</hkparam>                  │
//! └───────────────────────────────────────────────────────────┘
//! ```

use egui_widgets::{notify::Notification, shadcn::Button};
use gui_shared::i18n::{I18nKey, I18nMap};

use super::preview::{PreviewEvent, lines_ui};
use crate::{UserPatchOutcome, patch::PatchTool};

/// How far an object is to the right of its behavior.
const INDENT: f32 = 12.0;

/// What a click in the window asked for.
#[derive(Debug)]
enum Event {
    Show(String, String),
    Open(String, String),
    Remove(String, String),
    Revert(String, String, usize),
}

impl PatchTool {
    /// Draws the window if it is open. It belongs to no page: it stays when another is shown.
    pub(crate) fn user_patch_window(
        &mut self,
        ctx: &egui::Context,
        i18n: &I18nMap,
        notify: &mut Notification,
    ) -> UserPatchOutcome {
        if !self.user_patch.open {
            return UserPatchOutcome::default();
        }
        if self.user_patch.seen_revision != self.edit_revision {
            self.refresh_user_patch();
        }
        let mut open = true;
        let mut event = None;
        let window = egui_widgets::window_near_pointer(ctx, i18n.t(I18nKey::UserPatchTitle)).open(&mut open);
        window.default_size([640.0, 420.0]).resizable(true).show(ctx, |ui| event = self.user_patch_ui(ui, i18n));
        self.user_patch.open = open;
        self.apply_user_patch_event(event, notify)
    }

    fn apply_user_patch_event(&mut self, event: Option<Event>, notify: &mut Notification) -> UserPatchOutcome {
        let result = match event {
            None => return UserPatchOutcome::default(),
            Some(Event::Show(behavior, id)) => {
                self.show_user_patch_object(Some((behavior, id)));
                return UserPatchOutcome::default();
            }
            Some(Event::Open(behavior, id)) => {
                self.open_user_patch_object(behavior, id);
                return UserPatchOutcome { open_editor: true };
            }
            Some(Event::Remove(behavior, id)) => self.remove_user_patch_object(&behavior, &id),
            Some(Event::Revert(behavior, id, block)) => self.revert_user_patch_block(&behavior, &id, block),
        };
        if let Err(err) = result {
            notify.error(err);
        }
        UserPatchOutcome { open_editor: false }
    }

    fn user_patch_ui(&mut self, ui: &mut egui::Ui, i18n: &I18nMap) -> Option<Event> {
        let view = &mut self.user_patch;
        if view.rows.is_empty() {
            ui.weak(i18n.t(I18nKey::UserPatchEmpty));
            return None;
        }
        let mut event = None;
        let available = if view.preview.is_some() { ui.available_height() / 2.0 } else { ui.available_height() };
        // Filling the window, as a window shrinks to its contents and then resists a drag.
        let scroll = egui::ScrollArea::both().id_salt("user_patch_rows").max_height(available).auto_shrink(false);
        scroll.show(ui, |ui| {
            for group in view.rows.chunk_by(|a, b| a.behavior == b.behavior) {
                ui.add(
                    egui::Label::new(egui::RichText::new(&group[0].behavior).monospace().strong()).selectable(false),
                );
                for row in group {
                    ui.horizontal(|ui| {
                        ui.add_space(INDENT);
                        let is_selected = view
                            .selected
                            .as_ref()
                            .is_some_and(|(behavior, id)| *behavior == row.behavior && *id == row.id);
                        let text = egui::RichText::new(&row.label).monospace();
                        if ui.add(egui::Button::selectable(is_selected, text)).clicked() {
                            event = Some(Event::Show(row.behavior.clone(), row.id.clone()));
                        }
                    });
                }
            }
        });
        // Last: nothing is drawn below a list that took all the height.
        let (Some((behavior, id)), Some(preview)) = (&view.selected, &mut view.preview) else { return event };
        ui.separator();
        ui.horizontal(|ui| {
            if ui.add(Button::new(i18n.t(I18nKey::UserPatchOpenEditor)).small()).clicked() {
                event = Some(Event::Open(behavior.clone(), id.clone()));
            }
            let remove = Button::new(i18n.t(I18nKey::UserPatchRemove)).small().destructive();
            if ui.add(remove).on_hover_text(i18n.t(I18nKey::UserPatchRemoveHover)).clicked() {
                event = Some(Event::Remove(behavior.clone(), id.clone()));
            }
        });
        match lines_ui(preview, ui, i18n.t(I18nKey::NemesisRevertHunkHover)) {
            Some(PreviewEvent::Revert(block)) => Some(Event::Revert(behavior.clone(), id.clone(), block)),
            Some(PreviewEvent::JumpToEditor(_)) => Some(Event::Open(behavior.clone(), id.clone())),
            None => event,
        }
    }
}
