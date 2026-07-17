//! Widget trait implementation for RadioGroup.

impl<T: Clone + PartialEq + std::fmt::Display> egui::Widget
    for super::radio_group::RadioGroup<'_, T>
{
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let mut any_clicked = false;

        let response = ui.vertical(|ui| {
            for option in self.options {
                let is_selected = option == self.selected;
                let mut sel = is_selected;
                let r = ui.add(
                    crate::widgets::radio::radio::Radio::new(&mut sel)
                        .label(option.to_string()),
                );
                if r.clicked() && !is_selected {
                    *self.selected = option.clone();
                    any_clicked = true;
                }
            }
        });

        if any_clicked {
            ui.ctx().request_repaint();
        }

        response.response
    }
}
