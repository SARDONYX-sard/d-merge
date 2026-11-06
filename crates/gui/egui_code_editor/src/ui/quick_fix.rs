//! Draws the quick fix menu. See [`crate::quick_fix`].

use crate::{language::QuickFix, quick_fix::QuickFixMenu};

impl QuickFixMenu {
    /// Draws the menu below `anchor` (the cursor rect). Returns the fix clicked.
    pub(crate) fn ui(&mut self, ctx: &egui::Context, id: egui::Id, anchor: egui::Rect) -> Option<QuickFix> {
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
