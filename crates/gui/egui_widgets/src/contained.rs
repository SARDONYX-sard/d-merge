//! What a resizable panel holds, kept to the panel's width.
//!
//! A resizable panel (`egui::Panel::left(..).resizable(true)`) takes the width of what it holds when
//! that is wider, and keeps it: a widget a pixel too wide widens the panel, a little more each time
//! it is drawn again (a list shown, a button pressed). [`contained`] draws the contents in a child
//! that does not tell the panel its width: what is too wide is cut off at the panel's edge instead.
//!
//! ```text
//! panel width 320      contents 333 wide
//! without: 320 -> 333 -> 346 -> ...  (each time they are drawn)
//! with:    320, the 13 beyond are cut
//! ```

/// Draws `add_contents` in all the space left of `ui`, never wider: see the module docs. Only the
/// height it took is taken in `ui`.
pub fn contained<R>(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let rect = ui.available_rect_before_wrap();
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect).layout(*ui.layout()));
    child.set_clip_rect(rect.intersect(ui.clip_rect()));
    let inner = add_contents(&mut child);

    let bottom = child.min_rect().bottom().max(rect.top());
    let used = egui::Rect::from_min_max(rect.min, egui::pos2(rect.right(), bottom));
    ui.allocate_rect(used, egui::Sense::hover());
    inner
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The width of a resizable panel after `frames` frames of contents `wide` wide.
    fn panel_width(wide: f32, frames: usize, guarded: bool) -> f32 {
        let ctx = egui::Context::default();
        let mut width = 0.0;
        for _ in 0..frames {
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                let panel = egui::Panel::left("panel").resizable(true).default_size(200.0);
                width = panel
                    .show(ui, |ui| {
                        let draw = |ui: &mut egui::Ui| {
                            ui.allocate_exact_size(egui::vec2(wide, 10.0), egui::Sense::hover());
                        };
                        if guarded { contained(ui, draw) } else { draw(ui) }
                    })
                    .response
                    .rect
                    .width();
            });
            output.textures_delta.clear();
        }
        width
    }

    #[test]
    fn contents_too_wide_do_not_widen_the_panel() {
        let unguarded = panel_width(260.0, 3, false);
        let guarded = panel_width(260.0, 3, true);
        assert!(unguarded > 250.0, "egui widens it: {unguarded}");
        assert!(guarded < 210.0, "kept: {guarded}");
    }
}
