//! A dialog that asks before something that can't be undone, like shadcn/ui's `AlertDialog`.
//!
//! ```text
//! ┌──────────────────────────────────────────┐
//! │ Delete all of the User Patch?            │  <- the rest of the app is dimmed, and can't be used
//! │ Each conflict goes back to the mod that  │
//! │ wins by the load order. It can't be      │
//! │ undone.                                  │
//! │                    [Cancel] [Delete all] │
//! └──────────────────────────────────────────┘
//! ```
//!
//! `Cancel` starts with the keyboard, so that `Enter` alone doesn't do it. `Esc` and a click beside the
//! dialog cancel too.
//!
//! The caller keeps whether it is open, as with [`egui::Modal`]:
//! ```no_run
//! # use egui_widgets::shadcn::{AlertDialog, AlertDialogChoice};
//! # fn delete_all() {}
//! # fn example(ctx: &egui::Context, open: &mut bool) {
//! if *open {
//!     let choice = AlertDialog::new("Delete all?", "Cancel", "Delete all").destructive().show(ctx);
//!     if choice == Some(AlertDialogChoice::Action) {
//!         delete_all();
//!     }
//!     *open = choice.is_none();
//! }
//! # }
//! ```

use egui::{Align, Layout, RichText, vec2};

use super::{Button, Icon};

/// Default of [`AlertDialog::width`].
const WIDTH: f32 = 420.0;
/// Default of [`AlertDialog::title_size`].
const TITLE_SIZE: f32 = 16.0;
/// The height of a [`Button`] of the normal size, for the row of them.
const BUTTON_ROW_HEIGHT: f32 = 32.0;

/// What was chosen in an [`AlertDialog`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertDialogChoice {
    /// Its action button: go on.
    Action,
    /// `Cancel`, `Esc`, or a click beside it.
    Cancel,
}

/// See the module docs.
#[derive(Debug, Clone, Copy)]
#[must_use = "Call `show` to draw it"]
pub struct AlertDialog<'a> {
    title: &'a str,
    description: &'a str,
    cancel: &'a str,
    action: &'a str,
    action_icon: Option<Icon>,
    destructive: bool,
    width: f32,
    title_size: f32,
}

impl<'a> AlertDialog<'a> {
    /// `cancel`, `action`: the texts of the buttons. e.g. `Cancel`, `Delete all`
    #[inline]
    pub const fn new(title: &'a str, cancel: &'a str, action: &'a str) -> Self {
        Self {
            title,
            description: "",
            cancel,
            action,
            action_icon: None,
            destructive: false,
            width: WIDTH,
            title_size: TITLE_SIZE,
        }
    }

    /// What will happen, below the title.
    #[inline]
    pub const fn description(mut self, description: &'a str) -> Self {
        self.description = description;
        self
    }

    /// An icon in the action button.
    #[inline]
    pub const fn action_icon(mut self, icon: Icon) -> Self {
        self.action_icon = Some(icon);
        self
    }

    /// The action button is tinted as what destroys something. Else it is the primary one.
    #[inline]
    pub const fn destructive(mut self) -> Self {
        self.destructive = true;
        self
    }

    /// The width of its contents.
    #[inline]
    pub const fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// The font size of the title.
    #[inline]
    pub const fn title_size(mut self, title_size: f32) -> Self {
        self.title_size = title_size;
        self
    }

    /// Draws it, for each frame it is open. `Some`: it was answered, and is to close.
    pub fn show(self, ctx: &egui::Context) -> Option<AlertDialogChoice> {
        let id = egui::Id::NULL.with(("alert_dialog", self.title));
        let response = egui::Modal::new(id).show(ctx, |ui| self.contents(ui));
        if response.inner == Some(AlertDialogChoice::Action) {
            return response.inner;
        }
        if response.inner.is_some() || response.should_close() {
            return Some(AlertDialogChoice::Cancel);
        }
        None
    }

    fn contents(&self, ui: &mut egui::Ui) -> Option<AlertDialogChoice> {
        ui.set_width(self.width);
        ui.label(RichText::new(self.title).strong().size(self.title_size));
        if !self.description.is_empty() {
            ui.add_space(4.0);
            ui.add(egui::Label::new(RichText::new(self.description).weak()).wrap());
        }
        ui.add_space(12.0);
        let row = vec2(self.width, BUTTON_ROW_HEIGHT);
        ui.allocate_ui_with_layout(row, Layout::right_to_left(Align::Center), |ui| self.buttons(ui)).inner
    }

    /// From the right, as the layout goes: the action, then `Cancel` on its left.
    fn buttons(&self, ui: &mut egui::Ui) -> Option<AlertDialogChoice> {
        let mut action = Button::new(self.action);
        if let Some(icon) = self.action_icon {
            action = action.icon(icon);
        }
        action = if self.destructive { action.destructive() } else { action.primary() };
        let action = ui.add(action);
        let cancel = ui.add(Button::new(self.cancel));
        // Each time nothing has the keyboard, and so also when it opens: see the module docs.
        if ui.ctx().memory(|memory| memory.focused().is_none()) {
            cancel.request_focus();
        }
        if action.clicked() {
            Some(AlertDialogChoice::Action)
        } else if cancel.clicked() {
            Some(AlertDialogChoice::Cancel)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Modal;

    fn key(key: egui::Key) -> Vec<egui::Event> {
        let event = |pressed| egui::Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        vec![event(true), event(false)]
    }

    /// One frame with `events`, of the dialog in a [`Modal`], as it is used. Returns its answer.
    fn frame(ctx: &egui::Context, outer: &mut bool, events: Vec<egui::Event>) -> Option<AlertDialogChoice> {
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, vec2(1000.0, 800.0));
        let input = egui::RawInput { screen_rect: Some(screen), events, ..Default::default() };
        let mut choice = None;
        let mut output = ctx.run_ui(input, |ui| {
            let ctx = ui.ctx().clone();
            Modal::new("outer").show(&ctx, outer, |ui| {
                choice = AlertDialog::new("Delete all?", "Cancel", "Delete all").destructive().show(ui.ctx());
            });
        });
        output.textures_delta.clear();
        choice
    }

    fn opened() -> (egui::Context, bool) {
        let (ctx, mut outer) = (egui::Context::default(), true);
        for _ in 0..3 {
            assert_eq!(frame(&ctx, &mut outer, Vec::new()), None);
        }
        (ctx, outer)
    }

    #[test]
    fn enter_alone_cancels() {
        let (ctx, mut outer) = opened();
        assert_eq!(frame(&ctx, &mut outer, key(egui::Key::Enter)), Some(AlertDialogChoice::Cancel));
    }

    #[test]
    fn the_action_has_to_be_gone_to() {
        let (ctx, mut outer) = opened();
        assert_eq!(frame(&ctx, &mut outer, key(egui::Key::Tab)), None);
        assert_eq!(frame(&ctx, &mut outer, Vec::new()), None);
        assert_eq!(frame(&ctx, &mut outer, key(egui::Key::Enter)), Some(AlertDialogChoice::Action));
    }

    #[test]
    fn esc_closes_the_dialog_and_not_what_it_is_over() {
        let (ctx, mut outer) = opened();
        // What a modal looks at for its own `Esc`.
        ctx.memory_mut(|memory| {
            if let Some(focused) = memory.focused() {
                memory.surrender_focus(focused);
            }
        });
        assert_eq!(frame(&ctx, &mut outer, key(egui::Key::Escape)), Some(AlertDialogChoice::Cancel));
        assert!(outer);
    }
}
