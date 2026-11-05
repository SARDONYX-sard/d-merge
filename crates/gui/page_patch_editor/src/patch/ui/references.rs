//! The references window (`Shift+F12`, Vim `gD`). The list itself is in
//! [`crate::patch::navigate`].

use crate::patch::navigate::ReferenceList;

/// Draws the window. Returns the `(document, line)` that was clicked.
pub(in crate::patch) fn references_window<'a>(
    ctx: &egui::Context,
    list: &'a mut Option<ReferenceList>,
) -> Option<(&'a str, usize)> {
    let references = list.as_ref()?;
    let mut open = true;
    let mut picked = None;
    egui_widgets::window_near_pointer(ctx, references.title()).open(&mut open).default_height(300.0).show(ctx, |ui| {
        let row_height = ui.text_style_height(&egui::TextStyle::Monospace) + 6.0;
        egui::ScrollArea::vertical().show_rows(ui, row_height, references.len(), |ui, range| {
            for index in range {
                let text = egui::RichText::new(references.label(index)).monospace();
                if ui.add(egui::Button::selectable(false, text)).clicked() {
                    picked = Some(index);
                }
            }
        });
    });
    if !open {
        *list = None;
        return None;
    }
    list.as_ref()?.location(picked?)
}
