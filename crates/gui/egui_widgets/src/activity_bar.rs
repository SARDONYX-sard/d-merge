//! VS Code's activity bar: the icons of the side bar's views at the far left, the one shown with a
//! bar on its left, a count on one if it has any (e.g. the changes). And the title at the top of the
//! side bar.
//!
//! ```text
//! ┌──┬──────────────────┐
//! ▌🔍│ SEARCH           │   <- `▌`: the view shown;  `side_bar_title`
//! │Δ³│                  │   <- a badge: 3
//! │📂│                  │
//! └──┴──────────────────┘
//! ```
//! ```text
//! let items = [ActivityItem { icon: Icon::Search, hover: "Search (Ctrl+Shift+F)", shown: true, badge: 0 }, ..];
//! egui::Panel::left("activity").exact_size(ActivityBar::SIZE).show(ui, |ui| {
//!     if let Some(at) = ActivityBar::new().show(ui, items) { /* show view `at`, or hide the side bar */ }
//! });
//! ```

use crate::shadcn::{Icon, icon::paint_icon};

/// One icon of the bar.
#[derive(Debug, Clone, Copy)]
pub struct ActivityItem<'a> {
    pub icon: Icon,
    /// e.g. `Search (Ctrl+Shift+F)`
    pub hover: &'a str,
    /// Its view is the one shown.
    pub shown: bool,
    /// The count on it; `0`: none.
    pub badge: usize,
}

/// See the module docs.
#[derive(Debug, Clone, Copy)]
#[must_use = "Call `show` to draw it"]
pub struct ActivityBar {
    size: f32,
    icon_size: f32,
}

impl Default for ActivityBar {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl ActivityBar {
    /// Default of [`Self::size`]: also the width to give its panel.
    pub const SIZE: f32 = 40.0;
    /// Default of [`Self::icon_size`].
    const ICON_SIZE: f32 = 22.0;

    #[inline]
    pub const fn new() -> Self {
        Self { size: Self::SIZE, icon_size: Self::ICON_SIZE }
    }

    /// The height of an item (default 40).
    #[inline]
    pub const fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// The size of an icon (default 22).
    #[inline]
    pub const fn icon_size(mut self, size: f32) -> Self {
        self.icon_size = size;
        self
    }

    /// Draws `items` top to bottom. Returns the index of the one clicked.
    pub fn show<'a>(self, ui: &mut egui::Ui, items: impl IntoIterator<Item = ActivityItem<'a>>) -> Option<usize> {
        let mut clicked = None;
        for (at, item) in items.into_iter().enumerate() {
            // As wide as the panel lets it: its margins included.
            let (rect, response) =
                ui.allocate_exact_size(egui::vec2(ui.available_width(), self.size), egui::Sense::click());
            let visuals = ui.visuals();
            let lit = item.shown || response.hovered();
            let color = if lit { visuals.strong_text_color() } else { visuals.weak_text_color() };
            let icon = egui::Rect::from_center_size(rect.center(), egui::Vec2::splat(self.icon_size));
            paint_icon(ui.painter(), icon, item.icon, color);
            if item.shown {
                let bar = egui::Rect::from_min_size(rect.min, egui::vec2(2.0, rect.height()));
                ui.painter().rect_filled(bar, 0.0, visuals.selection.stroke.color);
            }
            if item.badge > 0 {
                badge(ui, rect, item.badge);
            }
            if response.on_hover_text(item.hover).clicked() {
                clicked = Some(at);
            }
        }
        clicked
    }
}

/// The count at the bottom right of `rect`, as VS Code's badges. e.g. `3`, or `99+`.
fn badge(ui: &egui::Ui, rect: egui::Rect, count: usize) {
    let text = if count > 99 { "99+".to_owned() } else { count.to_string() };
    let painter = ui.painter();
    let galley = painter.layout_no_wrap(text, egui::FontId::proportional(9.0), egui::Color32::WHITE);
    let size = egui::vec2(galley.size().x.max(galley.size().y) + 4.0, galley.size().y + 2.0);
    let badge = egui::Rect::from_min_size(rect.right_bottom() - size - egui::vec2(4.0, 4.0), size);
    painter.rect_filled(badge, size.y / 2.0, ui.visuals().selection.bg_fill);
    painter.galley(badge.center() - galley.size() / 2.0, galley, egui::Color32::WHITE);
}

/// `EXPLORER`: the name of a view, as VS Code puts it at the top of the side bar.
pub fn side_bar_title(ui: &mut egui::Ui, title: &str) {
    ui.label(egui::RichText::new(title.to_uppercase()).small().strong());
    ui.add_space(2.0);
}

/// `▼ TITLE ........ actions`: a side bar section's header, as VS Code's. A click on the title folds
/// it (`open`); `actions` (e.g. `[+]`) are drawn right to left at its end.
pub fn section_header(ui: &mut egui::Ui, open: &mut bool, title: &str, actions: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        let glyph = if *open { "▼" } else { "▶" };
        let text = egui::RichText::new(format!("{glyph} {}", title.to_uppercase())).small().strong();
        if ui.add(egui::Label::new(text).sense(egui::Sense::click())).clicked() {
            *open = !*open;
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), actions);
    });
}
