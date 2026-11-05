//! Draws the completion popup. The list and accepting an item are in [`crate::completion`].

use crate::completion::CompletionPopup;

impl CompletionPopup {
    /// Draws the list below `anchor` (the cursor rect in screen coordinates).
    ///
    /// Returns `true` when a candidate was clicked (the caller then calls [`Self::accept`]).
    pub(crate) fn ui(&mut self, ctx: &egui::Context, id: egui::Id, anchor: egui::Rect) -> bool {
        let Some(list) = &self.list else { return false };
        let mut clicked = false;

        // Below the cursor, top to bottom (candidates, then the selected one's documentation), never
        // past the window's bottom: a long list scrolls instead. Above only if there is no room.
        const DOC_HEIGHT: f32 = 160.0;
        let screen = ctx.content_rect();
        let (below, above) = (screen.bottom() - anchor.bottom(), anchor.top() - screen.top());
        let (pos, pivot, room) = if below >= 160.0 || below >= above {
            (anchor.left_bottom(), egui::Align2::LEFT_TOP, below)
        } else {
            (anchor.left_top(), egui::Align2::LEFT_BOTTOM, above)
        };
        let doc = list.items.get(self.selected).map(|item| item.documentation.as_str()).filter(|d| !d.is_empty());
        let doc_room = if doc.is_some() { DOC_HEIGHT } else { 0.0 };
        let list_height = (room - doc_room - 32.0).clamp(60.0, 400.0);
        let follow = self.scrolled_to != Some(self.selected);

        egui::Area::new(id.with("completion")).order(egui::Order::Foreground).fixed_pos(pos).pivot(pivot).show(
            ctx,
            |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_max_width(480.0);
                    // Popups are a separate layer: set the always-visible, solid scroll bars here too.
                    ui.spacing_mut().scroll = egui::style::ScrollStyle::solid();
                    // Candidates
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

                    // Documentation of the selected candidate, below the list.
                    if let Some(doc) = doc {
                        ui.separator();
                        egui::ScrollArea::vertical().id_salt("doc").max_height(DOC_HEIGHT).show(ui, |ui| {
                            egui_widgets::markdown::markdown(ui, doc);
                        });
                    }
                });
            },
        );
        self.scrolled_to = Some(self.selected);

        clicked
    }
}
