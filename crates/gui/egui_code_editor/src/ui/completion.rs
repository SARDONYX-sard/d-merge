//! Draws the completion popup. The list and accepting an item are in [`crate::completion`].

use crate::completion::CompletionPopup;

impl CompletionPopup {
    /// Draws the list below `anchor` (the cursor rect in screen coordinates).
    ///
    /// Returns `true` when a candidate was clicked (the caller then calls [`Self::accept`]).
    pub(crate) fn ui(&mut self, ctx: &egui::Context, id: egui::Id, anchor: egui::Rect) -> bool {
        let Some(list) = &self.list else { return false };
        let mut clicked = false;

        // Below the cursor, never past the window's bottom: a long list scrolls instead. Above only
        // if there is no room. The selected one's documentation is beside the list, as VS Code's.
        const DOC_WIDTH: f32 = 360.0;
        const DOC_HEIGHT: f32 = 240.0;
        let screen = ctx.content_rect();
        let (below, above) = (screen.bottom() - anchor.bottom(), anchor.top() - screen.top());
        let is_below = below >= 160.0 || below >= above;

        // The list.
        let follow = self.scrolled_to != Some(self.selected);
        let (area, list_height) = {
            let (pos, pivot, room) = if is_below {
                (anchor.left_bottom(), egui::Align2::LEFT_TOP, below)
            } else {
                (anchor.left_top(), egui::Align2::LEFT_BOTTOM, above)
            };
            let list_height = (room - 32.0).clamp(60.0, 400.0);
            let area = egui::Area::new(id.with("completion")).order(egui::Order::Foreground).fixed_pos(pos);
            (area.pivot(pivot), list_height)
        };
        let shown = area.show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_max_width(480.0);
                // Popups are a separate layer: set the always-visible, solid scroll bars here too.
                ui.spacing_mut().scroll = egui::style::ScrollStyle::solid();
                egui::ScrollArea::vertical()
                    .id_salt("items")
                    .max_height(list_height)
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                    .show(ui, |ui| {
                        for (i, item) in list.items.iter().enumerate() {
                            let label = format!("{} {}", item.icon, item.label);
                            let response = ui.add(egui::Button::selectable(i == self.selected, label));
                            // Keep the keyboard-selected one in view.
                            if i == self.selected && follow {
                                response.scroll_to_me(None);
                            }
                            if response.hovered() {
                                self.selected = i;
                            }
                            clicked |= response.clicked();
                        }
                    });
            });
        });

        // The documentation: right of the list, or left of it if the window has no room there; level
        // with the list's side next to the cursor.
        let doc = list.items.get(self.selected).map(|item| item.documentation.as_ref()).filter(|d| !d.is_empty());
        if let Some(doc) = doc {
            let list_rect = shown.response.rect;
            let right = screen.right() - list_rect.right() >= DOC_WIDTH;
            let (x, align_x) = if right {
                (list_rect.right() + 2.0, egui::Align::LEFT)
            } else {
                (list_rect.left() - 2.0, egui::Align::RIGHT)
            };
            let (y, align_y) =
                if is_below { (list_rect.top(), egui::Align::TOP) } else { (list_rect.bottom(), egui::Align::BOTTOM) };
            egui::Area::new(id.with("completion_doc"))
                .order(egui::Order::Foreground)
                .fixed_pos(egui::pos2(x, y))
                .pivot(egui::Align2([align_x, align_y]))
                .show(ctx, |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        ui.set_max_width(DOC_WIDTH);
                        egui::ScrollArea::vertical().id_salt("doc").max_height(DOC_HEIGHT).show(ui, |ui| {
                            egui_widgets::markdown::markdown(ui, doc);
                        });
                    });
                });
        }

        self.scrolled_to = Some(self.selected);
        clicked
    }
}
