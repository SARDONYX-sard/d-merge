use std::{borrow::Cow, sync::Arc};

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

/// Add a label truncated to fit within `width`; the full text is a tooltip only when truncated.
pub(crate) fn label_with_hover(ui: &mut Ui, text: &str, width: f32, align: CellAlign) {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(width, ROW_HEIGHT), egui::Sense::hover());
    // Off-screen rows (most of a long list): nothing to draw or hover.
    if !ui.is_rect_visible(rect) {
        return;
    }

    let shown = truncate_to_width(ui, text, width);
    ui.painter_at(rect).text(
        egui::pos2(align.anchor_x(&rect), rect.center().y),
        align.to_align2(),
        &shown,
        egui::TextStyle::Body.resolve(ui.style()),
        ui.style().visuals.text_color(),
    );
    if matches!(shown, Cow::Owned(_)) {
        response.on_hover_text(text);
    }
}

/// Add a hyperlink truncated to fit within `width`; the full URL is a tooltip only when truncated.
/// Always left-aligned (URLs are not centered or right-aligned).
pub(crate) fn hyperlink_with_hover(ui: &mut Ui, url: &str, width: f32) {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(width, ROW_HEIGHT),
        egui::Sense::click() | egui::Sense::hover(),
    );
    if url.trim().is_empty() || !ui.is_rect_visible(rect) {
        return;
    }

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

    let shown = truncate_to_width(ui, url, width);
    let is_truncated = matches!(shown, Cow::Owned(_));
    let font_id = egui::TextStyle::Body.resolve(ui.style());
    let text_pos = egui::pos2(rect.left(), rect.center().y);

    let painter = ui.painter_at(rect);
    let galley = painter.layout_no_wrap(shown.into_owned(), font_id, color);
    painter.galley(text_pos - egui::vec2(0.0, galley.size().y / 2.0), Arc::clone(&galley), color);

    // Draw underline when hovered
    if hovered {
        let text_width = galley.size().x.min(width);
        let underline_y = text_pos.y + 5.0;
        painter.line_segment(
            [
                egui::pos2(rect.left(), underline_y),
                egui::pos2(rect.left() + text_width, underline_y),
            ],
            egui::Stroke::new(1.0_f32, color),
        );
    }

    if response.clicked() {
        ui.ctx().open_url(egui::OpenUrl::new_tab(url));
    }
    if is_truncated {
        response.on_hover_text(url);
    }
}

/// `text` as is when it fits in `width` (no allocation), else cut with `...` (`Cow::Owned`).
///
/// Measures with per-glyph widths (no layout / `String` per try); kerning is ignored, which is
/// fine for deciding where to cut.
fn truncate_to_width<'a>(ui: &Ui, text: &'a str, width: f32) -> Cow<'a, str> {
    const ELLIPSIS: &str = "...";
    let font_id = egui::TextStyle::Body.resolve(ui.style());

    ui.fonts_mut(|fonts| {
        let mut glyph = |c| fonts.glyph_width(&font_id, c);
        if text.chars().map(&mut glyph).sum::<f32>() <= width {
            return Cow::Borrowed(text);
        }

        let available = width - ELLIPSIS.chars().map(&mut glyph).sum::<f32>();
        let mut used = 0.0;
        let mut end = 0;
        for (at, c) in text.char_indices() {
            used += glyph(c);
            if used > available {
                break;
            }
            end = at + c.len_utf8();
        }
        Cow::Owned(format!("{}{ELLIPSIS}", &text[..end]))
    })
}
