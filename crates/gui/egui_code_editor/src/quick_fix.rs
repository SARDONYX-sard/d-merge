//! Quick fix menu (`Ctrl+.`, like VS Code): the fixes the language offers at the cursor.
//!
//! ```text
//! 0.1 SpecialFrames_Invincible{"Duratoin":0.5}
//!     ┌──────────────────────────────────┐
//!     │ > Fix IFrame key -> "Duration"   │   Up / Down choose, Enter applies, Esc closes
//!     └──────────────────────────────────┘
//! ```

use crate::language::QuickFix;

#[derive(Debug, Default)]
pub(crate) struct QuickFixMenu {
    /// The fixes and the selected one, while the menu is open.
    open: Option<(Vec<QuickFix>, usize)>,
}

impl QuickFixMenu {
    pub(crate) fn open(&mut self, fixes: Vec<QuickFix>) {
        self.open = (!fixes.is_empty()).then_some((fixes, 0));
    }

    pub(crate) const fn is_open(&self) -> bool {
        self.open.is_some()
    }

    /// Menu keys. Call before the keymap and the `TextEdit`, so they do not see them.
    /// Returns the fix chosen with `Enter`.
    pub(crate) fn handle_keys(&mut self, ui: &egui::Ui) -> Option<QuickFix> {
        use egui::{Key, Modifiers};
        let (fixes, selected) = self.open.as_mut()?;
        let (up, down, enter, escape) = ui.input_mut(|i| {
            (
                i.consume_key(Modifiers::NONE, Key::ArrowUp),
                i.consume_key(Modifiers::NONE, Key::ArrowDown),
                i.consume_key(Modifiers::NONE, Key::Enter),
                i.consume_key(Modifiers::NONE, Key::Escape),
            )
        });
        let last = fixes.len() - 1;
        if up {
            *selected = selected.checked_sub(1).unwrap_or(last);
        }
        if down {
            *selected = if *selected >= last { 0 } else { *selected + 1 };
        }
        if escape {
            self.open = None;
            return None;
        }
        if enter {
            return self.open.take().map(|(mut fixes, selected)| fixes.swap_remove(selected));
        }
        None
    }

    /// Draws the menu below `anchor` (the cursor rect). Returns the fix clicked.
    pub(crate) fn ui(
        &mut self,
        ctx: &egui::Context,
        id: egui::Id,
        anchor: egui::Rect,
    ) -> Option<QuickFix> {
        let (fixes, selected) = self.open.as_ref()?;
        let mut clicked = None;
        egui::Area::new(id.with("quick_fix"))
            .order(egui::Order::Tooltip)
            .fixed_pos(anchor.left_bottom())
            .constrain(true)
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    for (i, fix) in fixes.iter().enumerate() {
                        let label = format!("💡 {}", fix.title);
                        if ui.add(egui::Button::selectable(i == *selected, label)).clicked() {
                            clicked = Some(i);
                        }
                    }
                });
            });
        let index = clicked?;
        self.open.take().map(|(mut fixes, _)| fixes.swap_remove(index))
    }
}
