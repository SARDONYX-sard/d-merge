//! The keymap status line (e.g., Vim's `NORMAL` / `INSERT` badge + `:w`, `/foo [2/5]`).

use crate::{CodeEditorState, Keymap, keymap::StatusLine};

/// Draws the status line of `state`'s keymap, or nothing for keymaps without modes.
///
/// ```text
/// [ NORMAL ]  recording @a  2d
/// [ INSERT ]
/// [COMMAND ]  :%s/foo/bar/g
/// ```
/// The badge color follows the mode (see [`crate::StatusMode::color`]).
pub fn status_line(
    ui: &mut egui::Ui,
    state: &CodeEditorState,
    keymap: Keymap,
) -> Option<egui::Response> {
    let StatusLine { mode, text } = state.keymap.status(keymap)?;
    let response = ui.horizontal(|ui| {
        // Same width for every mode, so the details do not jump around when it changes.
        let badge = format!(" {:^7} ", mode.label());
        let dark_text = egui::Color32::from_rgb(40, 44, 52);
        ui.label(
            egui::RichText::new(badge)
                .monospace()
                .strong()
                .color(dark_text)
                .background_color(mode.color()),
        );
        ui.label(egui::RichText::new(text).monospace());
    });
    Some(response.response)
}
