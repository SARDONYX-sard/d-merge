// SPDX-FileCopyrightText: (C) 2026 Pawel JankiewiczSARDONYX
// SPDX-License-Identifier: MIT
// ref: https://github.com/pjankiewicz/egui-shadcn/tree/fa5ceeed623eea983765fa4f886dd610d8b39470/src/widgets/combobox/combobox_show.rs
//! Searchable combobox widget.
//!
//! # Why did I fork it?
//! - Made “search” translatable
//! - Made the height adjustable

/// Searchable dropdown select.
#[must_use]
pub(crate) struct Combobox {
    id: egui::Id,
    items: Vec<String>,
    placeholder: String,
    width: Option<f32>,
    max_height: f32,
    min_height: f32,
    side: PopupSide,
    search_hint: String,
}

#[allow(unused)]
pub(crate) enum PopupSide {
    Bottom,
    Top,
    Left,
    Right,
}

impl Combobox {
    /// Creates a new combobox.
    pub(crate) fn new(id: impl egui::AsId, items: Vec<String>) -> Self {
        Self {
            id: egui::Id::new(id),
            items,
            placeholder: "Select...".to_string(),
            width: None,
            max_height: 250.0,
            min_height: 250.0,
            side: PopupSide::Bottom,
            search_hint: "Search...".to_string(),
        }
    }

    /// Sets placeholder text shown when no item is selected.
    #[allow(unused)]
    pub(crate) fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.placeholder = text.into();
        self
    }

    /// Sets trigger width.
    #[allow(unused)]
    pub(crate) const fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Sets maximum popup height before scrolling.
    #[allow(unused)]
    pub(crate) const fn max_height(mut self, height: f32) -> Self {
        self.max_height = height;
        self
    }

    /// Sets maximum popup height before scrolling.
    #[allow(unused)]
    pub(crate) const fn min_height(mut self, height: f32) -> Self {
        self.min_height = height;
        self
    }

    #[allow(unused)]
    pub(crate) const fn side(mut self, side: PopupSide) -> Self {
        self.side = side;
        self
    }

    pub(crate) fn search_hint(mut self, search_hint: impl Into<String>) -> Self {
        self.search_hint = search_hint.into();
        self
    }

    /// Shows the combobox.
    ///
    /// `selected` contains the selected item index.
    #[allow(unused)]
    pub(crate) fn show(self, ui: &mut egui::Ui, selected: &mut Option<usize>) -> egui::Response {
        let before = *selected;

        let theme = egui_shadcn::theme::shadcn_theme_ext::ShadcnThemeExt::shadcn_theme(ui.ctx());

        let display = selected
            .and_then(|idx| self.items.get(idx))
            .cloned()
            .unwrap_or_else(|| self.placeholder.clone());

        let text_color = if selected.is_some() { theme.foreground } else { theme.muted_foreground };

        let icon_size = 12.0;
        let h_padding = 10.0;
        let height = 36.0;

        let width = self.width.unwrap_or(220.0).min(ui.available_width());

        let galley =
            ui.painter().layout_no_wrap(display, egui::FontId::proportional(14.0), text_color);

        let desired = egui::vec2(width, height);

        let (trigger_rect, mut trigger) = ui.allocate_exact_size(desired, egui::Sense::click());

        if ui.is_rect_visible(trigger_rect) {
            let cr = egui::CornerRadius::same(theme.radius.round() as u8);

            let bg = if trigger.is_pointer_button_down_on() {
                egui_shadcn::paint::interpolate_color::interpolate_color(
                    theme.background,
                    theme.accent,
                    0.85,
                )
            } else if trigger.hovered() {
                theme.accent
            } else {
                theme.background
            };

            ui.painter().rect_filled(trigger_rect, cr, bg);

            ui.painter().rect_stroke(
                trigger_rect,
                cr,
                egui::Stroke::new(1.0_f32, theme.input),
                egui::epaint::StrokeKind::Inside,
            );

            let text_pos = egui::pos2(
                trigger_rect.min.x + h_padding,
                trigger_rect.center().y - galley.size().y / 2.0,
            );

            ui.painter().galley(text_pos, galley, text_color);

            let icon_rect = egui::Rect::from_min_size(
                egui::pos2(
                    trigger_rect.max.x - h_padding - icon_size,
                    trigger_rect.center().y - icon_size / 2.0,
                ),
                egui::vec2(icon_size, icon_size),
            );

            egui_shadcn::icons::paint_icon::paint_icon(
                ui.painter(),
                icon_rect,
                &egui_shadcn::icons::lucide_icon::LucideIcon::ChevronDown,
                theme.muted_foreground,
            );
        }

        if trigger.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }

        let popup_id = self.id.with("popup");
        let search_id = self.id.with("search");

        let toggle_cmd = if trigger.clicked() { Some(egui::SetOpenCommand::Toggle) } else { None };

        let cr = (theme.radius + 2.0).round() as u8;

        let themed_frame = egui::Frame::NONE
            .fill(theme.popover)
            .inner_margin(egui::Margin::same(8))
            .corner_radius(egui::CornerRadius::same(cr))
            .stroke(egui::Stroke::new(1.0_f32, theme.border))
            .shadow(egui::Shadow {
                offset: [0, 4],
                blur: 12,
                spread: 0,
                color: egui::Color32::from_black_alpha(8),
            });

        let popup_pos = match self.side {
            PopupSide::Bottom => egui::pos2(trigger_rect.min.x, trigger_rect.max.y),
            PopupSide::Top | PopupSide::Left => egui::pos2(trigger_rect.min.x, trigger_rect.min.y),
            PopupSide::Right => egui::pos2(trigger_rect.max.x, trigger_rect.min.y),
        };

        let popup = egui::Popup::new(popup_id, ui.ctx().clone(), &trigger, ui.layer_id())
            .at_position(popup_pos)
            .open_memory(toggle_cmd)
            .frame(themed_frame);

        let mut close = false;

        let mut search_text =
            ui.memory_mut(|mem| mem.data.get_temp::<String>(search_id).unwrap_or_default());

        popup.show(|ui| {
            let popup_width = trigger_rect.width().max(200.0);

            ui.set_min_width(popup_width);
            ui.set_max_width(popup_width);
            ui.set_min_height(self.min_height);

            let input_resp = egui_shadcn::widgets::input::input::Input::new(&mut search_text)
                .placeholder(self.search_hint)
                .desired_width(ui.available_width())
                .show(ui);

            if trigger.clicked() {
                input_resp.request_focus();
            }

            ui.add_space(4.0);

            let query = search_text.to_lowercase();

            let filtered: Vec<(usize, &String)> = self
                .items
                .iter()
                .enumerate()
                .filter(|(_, item)| query.is_empty() || item.to_lowercase().contains(&query))
                .collect();

            if filtered.is_empty() {
                ui.label(
                    egui::RichText::new("No results found")
                        .color(theme.muted_foreground)
                        .size(13.0),
                );
            } else {
                let check_icon_size = 12.0;
                let item_left_pad = check_icon_size + 6.0;

                egui::ScrollArea::vertical().max_height(self.max_height).show(ui, |ui| {
                    for (idx, label) in filtered {
                        let is_selected = *selected == Some(idx);

                        let galley = ui.painter().layout_no_wrap(
                            label.clone(),
                            egui::FontId::proportional(14.0),
                            theme.popover_foreground,
                        );

                        let desired = egui::vec2(ui.available_width(), galley.size().y + 8.0);

                        let (rect, r) = ui.allocate_exact_size(desired, egui::Sense::click());

                        if r.hovered() || is_selected {
                            ui.painter().rect_filled(
                                rect,
                                egui::CornerRadius::same(4),
                                theme.accent,
                            );
                        }

                        if r.hovered() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        }

                        if ui.is_rect_visible(rect) {
                            if is_selected {
                                let check_rect = egui::Rect::from_min_size(
                                    egui::pos2(
                                        rect.min.x + 4.0,
                                        rect.center().y - check_icon_size / 2.0,
                                    ),
                                    egui::vec2(check_icon_size, check_icon_size),
                                );

                                egui_shadcn::icons::paint_icon::paint_icon(
                                    ui.painter(),
                                    check_rect,
                                    &egui_shadcn::icons::lucide_icon::LucideIcon::Check,
                                    theme.popover_foreground,
                                );
                            }

                            ui.painter().galley(
                                egui::pos2(
                                    rect.min.x + item_left_pad,
                                    rect.center().y - galley.size().y / 2.0,
                                ),
                                galley,
                                theme.popover_foreground,
                            );
                        }

                        if r.clicked() {
                            *selected = Some(idx);
                            search_text.clear();
                            close = true;
                        }
                    }
                });
            }
        });

        ui.memory_mut(|mem| {
            if search_text.is_empty() {
                mem.data.remove::<String>(search_id);
            } else {
                mem.data.insert_temp(search_id, search_text);
            }
        });

        if close {
            egui::Popup::close_id(ui.ctx(), popup_id);
            ui.ctx().request_repaint();
        }

        if *selected != before {
            // Popup selection changes are not reflected in the trigger
            // response automatically. Mark the response manually so
            // callers can reliably use Response::changed().
            trigger.mark_changed();
        }

        trigger
    }
}
