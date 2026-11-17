// SPDX-FileCopyrightText: (C) 2026 SARDONYX
// SPDX-License-Identifier: MIT
//! A table's head cell, as shadcn's `TableHead` with a sort button: the column's name in the muted
//! color (in the middle, or as [`TableHead::align`] says), the sort arrow on the right, a border
//! under the head row.
//!
//! ```text
//! │ Name          │ Priority  ▲ │   the column sorted by: its name in the text color, an arrow
//! ├───────────────┴─────────────┤   the border right under the head, one line
//! ```
//! A click asks to sort by it (the caller turns the order).

use super::{
    icon::{Icon, paint_icon},
    theme::ShadcnTheme,
};

/// A table's head cell. See the module docs.
#[derive(Debug)]
#[must_use = "You should put this widget in a ui with `ui.add(widget);`"]
pub struct TableHead {
    text: egui::WidgetText,
    sort: Option<bool>,
    sortable: bool,
    align: egui::Align,
    /// `None`: [`Self::BORDER_WIDTH`] in the theme's `muted_foreground`.
    border: Option<egui::Stroke>,
    /// `None`: the theme's `muted`, faint.
    fill: Option<egui::Color32>,
}

impl TableHead {
    /// The sort arrow's size.
    const ARROW: f32 = 14.0;
    const PADDING: f32 = 8.0;
    /// What a sortable cell takes beside its text: the padding and the arrow. A column as wide as
    /// its name and this shows all of it.
    pub const EXTRA_WIDTH: f32 = Self::PADDING * 2.5 + Self::ARROW;
    /// Default width of [`Self::border`]: the line between the head and the body is to be seen at
    /// a glance.
    pub const BORDER_WIDTH: f32 = 2.0;

    #[inline]
    pub fn new(text: impl Into<egui::WidgetText>) -> Self {
        Self { text: text.into(), sort: None, sortable: true, align: egui::Align::Center, border: None, fill: None }
    }

    /// The line under the head row, where the body starts.
    #[inline]
    pub const fn border(mut self, border: egui::Stroke) -> Self {
        self.border = Some(border);
        self
    }

    /// The cell's background, apart from the body's rows.
    #[inline]
    pub const fn fill(mut self, fill: egui::Color32) -> Self {
        self.fill = Some(fill);
        self
    }

    /// Where the name is in the cell (beside the arrow). Default: the middle.
    #[inline]
    pub const fn align(mut self, align: egui::Align) -> Self {
        self.align = align;
        self
    }

    /// The table is sorted by this column: `Some(true)` ascending, `Some(false)` descending.
    #[inline]
    pub const fn sorted(mut self, sort: Option<bool>) -> Self {
        self.sort = sort;
        self
    }

    /// A cell that does not sort (no pointer, no arrow), e.g. of a column of flags.
    #[inline]
    pub const fn plain(mut self) -> Self {
        self.sortable = false;
        self
    }
}

impl egui::Widget for TableHead {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let theme = ShadcnTheme::get(ui.ctx());
        let sense = if self.sortable { egui::Sense::click() } else { egui::Sense::hover() };
        let (rect, response) = ui.allocate_exact_size(ui.available_size(), sense);
        if !ui.is_rect_visible(rect) {
            return response;
        }

        let painter = ui.painter();
        let pointed = self.sortable && response.hovered();
        painter.rect_filled(rect, 0.0, self.fill.unwrap_or_else(|| theme.muted.gamma_multiply(0.45)));
        if pointed {
            painter.rect_filled(rect, theme.corner_radius(), theme.muted);
        }
        // The border right under the head row, where the body starts: one line over the gaps
        // between the cells too. A cell's own painter is clipped to it, so the line is drawn on the
        // layer's.
        let border = self.border.unwrap_or_else(|| egui::Stroke::new(Self::BORDER_WIDTH, theme.muted_foreground));
        let gap = ui.spacing().item_spacing.x / 2.0;
        let under = rect.bottom() + ui.spacing().item_spacing.y / 2.0;
        let room = egui::vec2(gap, gap.mul_add(2.0, border.width));
        let line = ui.ctx().layer_painter(ui.layer_id()).with_clip_rect(ui.clip_rect().expand2(room));
        line.hline((rect.left() - gap)..=(rect.right() + gap), under, border);

        let color = if self.sort.is_some() || pointed { theme.foreground } else { theme.muted_foreground };
        let arrow = if self.sortable { Self::ARROW + Self::PADDING / 2.0 } else { 0.0 };
        let mut job = egui::text::LayoutJob::simple_singleline(
            self.text.text().to_owned(),
            egui::TextStyle::Button.resolve(ui.style()),
            color,
        );
        // Where the name goes: the cell but its padding and the arrow.
        let area = egui::Rect::from_min_max(
            egui::pos2(rect.left() + Self::PADDING, rect.top()),
            egui::pos2((rect.right() - Self::PADDING - arrow).max(rect.left() + Self::PADDING), rect.bottom()),
        );
        job.wrap = egui::text::TextWrapping::truncate_at_width(area.width());
        let galley = painter.layout_job(job);
        let x = match self.align {
            egui::Align::Min => area.left(),
            egui::Align::Center => area.center().x - galley.size().x / 2.0,
            egui::Align::Max => area.right() - galley.size().x,
        };
        painter.galley(egui::pos2(x, rect.center().y - galley.size().y / 2.0), galley, color);

        let icon = match self.sort {
            Some(true) => Some(Icon::ChevronUp),
            Some(false) => Some(Icon::ChevronDown),
            None if pointed => Some(Icon::ChevronsUpDown),
            None => None,
        };
        if let Some(icon) = icon {
            let at = egui::pos2(rect.right() - Self::PADDING / 2.0 - Self::ARROW, rect.center().y - Self::ARROW / 2.0);
            let icon_rect = egui::Rect::from_min_size(at, egui::vec2(Self::ARROW, Self::ARROW));
            let icon_color = if self.sort.is_some() { theme.foreground } else { theme.muted_foreground };
            paint_icon(painter, icon_rect, icon, icon_color);
        }

        if pointed {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        response
    }
}
