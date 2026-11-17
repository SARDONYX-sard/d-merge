// SPDX-FileCopyrightText: (C) 2026 SARDONYX
// SPDX-License-Identifier: MIT
//! A menu's item, as shadcn's `DropdownMenuItem`: as wide as the menu, no frame, the accent while
//! pointed at. A click closes the menu.
//!
//! ```text
//! ┌──────────────────────┐
//! │ ⧉  Copy ID           │
//! │▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓│  <- pointed at
//! │ 🗑  Remove           │  <- destructive: in its color
//! └──────────────────────┘
//! ```
//! Only for a menu (`menu_button`, `context_menu`, a popup): its click closes what it is in.

use super::{
    Button,
    icon::{Icon, paint_icon},
    theme::ShadcnTheme,
};

/// `button` that opens a menu below it, as `ui.menu_button` with a shadcn button. A `menu_button`
/// in the menu is a submenu, as in egui's.
pub fn menu_button<R>(
    ui: &mut egui::Ui,
    button: Button,
    contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<Option<R>> {
    let response = ui.add(button);
    let inner = egui::Popup::menu(&response).show(contents).map(|shown| shown.inner);
    egui::InnerResponse::new(inner, response)
}

/// A menu's item. See the module docs.
#[derive(Debug)]
#[must_use = "You should put this widget in a ui with `ui.add(widget);`"]
pub struct MenuItem {
    text: egui::WidgetText,
    icon: Option<Icon>,
    destructive: bool,
    keep_open: bool,
}

impl MenuItem {
    /// Default height of an item.
    const HEIGHT: f32 = 26.0;
    const PADDING: f32 = 8.0;
    const ICON: f32 = 15.0;
    const GAP: f32 = 8.0;

    #[inline]
    pub fn new(text: impl Into<egui::WidgetText>) -> Self {
        Self { text: text.into(), icon: None, destructive: false, keep_open: false }
    }

    /// An icon before the text.
    #[inline]
    pub const fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// In the destructive color: it deletes or cannot be undone.
    #[inline]
    pub const fn destructive(mut self) -> Self {
        self.destructive = true;
        self
    }

    /// A click leaves the menu open, e.g. to pick several in a row.
    #[inline]
    pub const fn keep_open(mut self) -> Self {
        self.keep_open = true;
        self
    }
}

impl egui::Widget for MenuItem {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let theme = ShadcnTheme::get(ui.ctx());
        let font = egui::TextStyle::Button.resolve(ui.style());
        let galley = ui.painter().layout_no_wrap(self.text.text().to_owned(), font, egui::Color32::PLACEHOLDER);

        let icon_width = if self.icon.is_some() { Self::ICON + Self::GAP } else { 0.0 };
        let width = Self::PADDING * 2.0 + icon_width + galley.size().x;
        let height = Self::HEIGHT.max(galley.size().y + 4.0);
        let (rect, response) = ui.allocate_at_least(egui::vec2(width, height), egui::Sense::click());

        if ui.is_rect_visible(rect) {
            let pointed = response.hovered() && ui.is_enabled();
            let fg = match (self.destructive, pointed, ui.is_enabled()) {
                (_, _, false) => theme.muted_foreground,
                (true, _, true) => theme.destructive,
                (false, true, true) => theme.accent_foreground,
                (false, false, true) => theme.popover_foreground,
            };
            if pointed {
                let bg = if self.destructive { theme.destructive.gamma_multiply(0.15) } else { theme.accent };
                ui.painter().rect_filled(rect, theme.corner_radius(), bg);
            }

            let mut x = rect.left() + Self::PADDING;
            if let Some(icon) = self.icon {
                let icon_rect = egui::Rect::from_min_size(
                    egui::pos2(x, rect.center().y - Self::ICON / 2.0),
                    egui::vec2(Self::ICON, Self::ICON),
                );
                paint_icon(ui.painter(), icon_rect, icon, fg);
                x += icon_width;
            }
            ui.painter().galley(egui::pos2(x, rect.center().y - galley.size().y / 2.0), galley, fg);
        }

        if response.clicked() && !self.keep_open {
            ui.close();
        }
        response
    }
}
