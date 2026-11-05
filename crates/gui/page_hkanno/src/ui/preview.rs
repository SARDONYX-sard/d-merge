//! Draws the XML preview. The building and the jump map are in [`crate::preview`].

use egui_code_editor::languages::Xml;

use crate::preview::Preview;

impl Preview {
    /// Draws the XML (only the visible rows are laid out, so huge files stay fast).
    pub(crate) fn ui(&mut self, ui: &mut egui::Ui) {
        let font = egui::TextStyle::Monospace.resolve(ui.style());
        let row_height = ui.fonts_mut(|f| f.row_height(&font));
        let row_step = row_height + ui.spacing().item_spacing.y;

        let mut area = egui::ScrollArea::both().id_salt("hkanno_preview").auto_shrink(false);
        if let Some(target) = self.scroll_to.take() {
            // Put the target line in the middle of the view.
            let offset = (target as f32).mul_add(row_step, -ui.available_height() / 2.0);
            area = area.vertical_scroll_offset(offset.max(0.0));
        }

        let highlight = ui.visuals().selection.bg_fill.gamma_multiply(0.5);
        area.show_rows(ui, row_height, self.lines.len(), |ui, rows| {
            for i in rows {
                // Colored per visible row only, so huge files stay fast.
                let mut job = egui_code_editor::line_job(ui, &self.lines[i], &Xml);
                if self.highlighted == Some(i) {
                    for section in &mut job.sections {
                        section.format.background = highlight;
                    }
                }
                ui.add(egui::Label::new(job).extend());
            }
        });
    }
}
