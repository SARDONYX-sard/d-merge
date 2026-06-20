use std::sync::Arc;

use egui::{Align2, Ui};

pub(crate) const ROW_HEIGHT: f32 = 20.0;

/// Horizontal alignment for cell content.
#[derive(Clone, Copy)]
pub(crate) enum CellAlign {
    Left,
    Center,
    #[allow(unused)]
    Right,
}

impl CellAlign {
    const fn to_align2(self) -> Align2 {
        match self {
            Self::Left => Align2::LEFT_CENTER,
            Self::Center => Align2::CENTER_CENTER,
            Self::Right => Align2::RIGHT_CENTER,
        }
    }

    fn anchor_x(self, rect: &egui::Rect) -> f32 {
        match self {
            Self::Left => rect.left(),
            Self::Center => rect.center().x,
            Self::Right => rect.right(),
        }
    }
}

/// Add a label with hover tooltip, truncated to fit within `width`.
pub(crate) fn label_with_hover(ui: &mut Ui, text: &str, width: f32, align: CellAlign) {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(width, ROW_HEIGHT), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter_at(rect);
        painter.text(
            egui::pos2(align.anchor_x(&rect), rect.center().y),
            align.to_align2(),
            truncate_to_width(ui, text, width),
            egui::TextStyle::Body.resolve(ui.style()),
            ui.style().visuals.text_color(),
        );
    }
    response.on_hover_text(text);
}

/// Add a hyperlink with hover tooltip, truncated to fit within `width`.
/// Always left-aligned (URLs are not centered or right-aligned).
pub(crate) fn hyperlink_with_hover(ui: &mut Ui, url: &str, width: f32) {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(width, ROW_HEIGHT),
        egui::Sense::click() | egui::Sense::hover(),
    );

    if url.trim().is_empty() {
        return;
    }

    if ui.is_rect_visible(rect) {
        let hovered = response.hovered();

        // Change cursor to pointer on hover
        if hovered {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }

        let color = if hovered {
            ui.visuals().hyperlink_color
        } else {
            ui.visuals().hyperlink_color.gamma_multiply(0.85)
        };

        let truncated = truncate_to_width(ui, url, width);
        let font_id = egui::TextStyle::Body.resolve(ui.style());
        let text_pos = egui::pos2(rect.left(), rect.center().y);

        let painter = ui.painter_at(rect);
        let galley = painter.layout_no_wrap(truncated, font_id, color);
        painter.galley(
            text_pos - egui::vec2(0.0, galley.size().y / 2.0),
            Arc::clone(&galley),
            color,
        );

        // Draw underline when hovered
        if hovered {
            let text_width = galley.size().x.min(width);
            let underline_y = text_pos.y + 5.0;
            painter.line_segment(
                [
                    egui::pos2(rect.left(), underline_y),
                    egui::pos2(rect.left() + text_width, underline_y),
                ],
                egui::Stroke::new(1.0, color),
            );
        }
    }

    if response.clicked() {
        ui.ctx().open_url(egui::OpenUrl::new_tab(url));
    }

    response.on_hover_text(url);
}

fn truncate_to_width(ui: &Ui, text: &str, width: f32) -> String {
    let font_id = ui
        .style()
        .text_styles
        .get(&egui::TextStyle::Body)
        .cloned()
        .unwrap_or_else(|| egui::FontId::proportional(14.0));
    let text_color = ui.style().visuals.text_color();

    let measure = |s: String| -> f32 {
        ui.fonts_mut(|fonts| fonts.layout_no_wrap(s, font_id.clone(), text_color).size().x)
    };

    if measure(text.to_string()) <= width {
        return text.to_string();
    }

    const ELLIPSIS: &str = "...";
    let ellipsis_width = measure(ELLIPSIS.to_string());
    let available = width - ellipsis_width;

    if available <= 0.0 {
        return ELLIPSIS.to_string();
    }

    let chars: Vec<char> = text.chars().collect();
    let mut lo = 0_usize;
    let mut hi = chars.len();

    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        let candidate: String = chars[..mid].iter().collect();
        if measure(candidate) <= available {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }

    if lo == 0 {
        return ELLIPSIS.to_string();
    }

    format!("{}{ELLIPSIS}", chars[..lo].iter().collect::<String>())
}
