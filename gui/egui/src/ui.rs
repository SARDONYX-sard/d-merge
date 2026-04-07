use egui::{Align, Ui};

/// Table: 1 row height size.
pub const ROW_HEIGHT: f32 = 20.0;

/// Add a label with hover tooltip (truncated if too long).
pub fn label_with_hover(ui: &mut Ui, text: &str, width: f32) {
    let truncated = truncate_to_width(ui, text, width);
    let display = if truncated.is_empty() {
        " ".repeat((width / 6.0).max(3.0) as usize)
    } else {
        truncated
    };
    ui.add_sized(
        [width, ROW_HEIGHT],
        egui::Label::new(display).halign(Align::LEFT),
    )
    .on_hover_text(text);
}

/// Add a hyperlink with hover tooltip (truncated if too long).
pub fn hyperlink_with_hover(ui: &mut Ui, url: &str, width: f32) {
    if url.trim().is_empty() {
        ui.add_sized(
            [width, ROW_HEIGHT],
            egui::Label::new(" ".repeat((width / 6.0).max(3.0) as usize)),
        );
        return;
    }

    let truncated = truncate_to_width(ui, url, width);
    ui.add_sized(
        [width, ROW_HEIGHT],
        egui::Hyperlink::from_label_and_url(truncated, url),
    )
    .on_hover_text(url);
}

/// Truncate text to fit within given width.
fn truncate_to_width(ui: &Ui, text: &str, width: f32) -> String {
    let text_color = ui.style().visuals.text_color();
    let font_id = egui::TextStyle::Body.resolve(ui.style());

    let galley_x_size = ui.fonts_mut(|font| {
        font.layout_no_wrap(text.to_string(), font_id.clone(), text_color)
            .size()
            .x
    });
    if galley_x_size <= width {
        return text.to_string();
    }

    let mut truncated = String::new();
    for ch in text.chars() {
        let galley_x_size = ui.fonts_mut(|font| {
            font.layout_no_wrap(format!("{truncated}{ch}..."), font_id.clone(), text_color)
                .size()
                .x
        });

        if galley_x_size > width {
            truncated.push_str("...");
            break;
        }
        truncated.push(ch);
    }
    truncated
}
