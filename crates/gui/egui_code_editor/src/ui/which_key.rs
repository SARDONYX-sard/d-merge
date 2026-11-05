//! Draws the list of what can follow a key that waits for another. See
//! [`crate::keymap::Prefix`] for the list, and [`wait`] for when it shows.

use crate::keymap::Prefix;

/// Someone who knows the keys has pressed the next one by then: the list would only flash.
const DELAY_SECONDS: f64 = 0.8;
/// Between the list and the edges of the editor.
const MARGIN: f32 = 8.0;

/// When the current prefix started to wait. `Some(..)` only while one does.
pub(crate) type Waiting = Option<(Prefix, f64)>;

/// Tracks the wait. Returns how long is left, or `None` if the list can show.
pub(crate) fn wait(waiting: &mut Waiting, prefix: Prefix, now: f64) -> Option<f64> {
    let since = match *waiting {
        Some((current, since)) if current == prefix => since,
        _ => {
            *waiting = Some((prefix, now));
            now
        }
    };
    let left = DELAY_SECONDS - (now - since);
    (left > 0.0).then_some(left)
}

/// `area`: the visible part of the editor. The list is at its bottom right, over the text, and lets
/// the pointer through.
pub(crate) fn show(ctx: &egui::Context, id: egui::Id, area: egui::Rect, prefix: Prefix, host_commands: bool) {
    egui::Area::new(id.with("which_key"))
        .order(egui::Order::Foreground)
        .interactable(false)
        .pivot(egui::Align2::RIGHT_BOTTOM)
        .fixed_pos(area.right_bottom() - egui::vec2(MARGIN, MARGIN))
        .constrain(true)
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                ui.strong(prefix.title());
                egui::Grid::new(id.with("which_key_grid")).num_columns(2).show(ui, |ui| {
                    for hint in prefix.hints(host_commands) {
                        ui.label(egui::RichText::new(hint.key()).monospace().strong());
                        ui.label(hint.text);
                        ui.end_row();
                    }
                });
            });
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shows_after_the_delay_and_starts_over_on_another_prefix() {
        let mut waiting = None;
        assert!(wait(&mut waiting, Prefix::Leader, 10.0).is_some(), "just pressed");
        assert!(wait(&mut waiting, Prefix::Leader, 10.1).is_some());
        assert_eq!(wait(&mut waiting, Prefix::Leader, 10.0 + DELAY_SECONDS), None);
        assert_eq!(wait(&mut waiting, Prefix::Leader, 12.0), None, "and stays");
        assert!(wait(&mut waiting, Prefix::G, 12.1).is_some(), "another key");
    }
}
