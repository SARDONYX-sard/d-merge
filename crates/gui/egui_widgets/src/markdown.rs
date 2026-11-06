//! Tiny Markdown renderer for tooltips (hover docs, completion docs).
//!
//! Supports only what the hkanno docs use:
//! - `# heading`
//! - `- list item`
//! - fenced code blocks (```` ``` ````)
//! - inline `` `code` ``, `**bold**` and `[text](url)` links
//! - `⟦class|hkbClipGenerator⟧` and `⟦name|Idle⟧`: the colors of class and object names, the same
//!   as in lists, so that a tooltip reads like the list it is about
//!
//! Anything else is shown as plain text. A full CommonMark renderer would be overkill here.

/// The color of a class name in lists and tooltips.
#[inline]
pub const fn class_color(visuals: &egui::Visuals) -> egui::Color32 {
    if visuals.dark_mode { egui::Color32::from_rgb(86, 182, 194) } else { egui::Color32::from_rgb(0, 110, 130) }
}

/// The color of an object's name in lists and tooltips.
#[inline]
pub const fn name_color(visuals: &egui::Visuals) -> egui::Color32 {
    if visuals.dark_mode { egui::Color32::from_rgb(229, 192, 123) } else { egui::Color32::from_rgb(150, 90, 0) }
}

/// Default of [`Markdown::line_height`]: as VS Code's hover (`line-height: 1.5em`).
const LINE_HEIGHT: f32 = 1.5;

/// Renders `markdown` into `ui`, see [`Markdown`].
#[inline]
pub fn markdown(ui: &mut egui::Ui, markdown: &str) -> Vec<egui::Response> {
    Markdown::new(markdown).show(ui)
}

/// A text to render, see the module docs.
#[derive(Debug, Clone, Copy)]
#[must_use = "Call `show` to draw it"]
pub struct Markdown<'a> {
    text: &'a str,
    line_height: f32,
}

impl<'a> Markdown<'a> {
    #[inline]
    pub const fn new(text: &'a str) -> Self {
        Self { text, line_height: LINE_HEIGHT }
    }

    /// The height of a line of text, in multiples of the font size.
    #[inline]
    pub const fn line_height(mut self, line_height: f32) -> Self {
        self.line_height = line_height;
        self
    }

    /// Returns the responses of the links, in order, e.g. to focus the first one for keyboard use
    /// (a focused link opens with `Enter`; `Tab` moves to the next one).
    #[inline]
    pub fn show(self, ui: &mut egui::Ui) -> Vec<egui::Response> {
        render(ui, self.text, self.line_height)
    }
}

/// A line that wraps, with each of its rows `height` tall.
fn text_line(ui: &mut egui::Ui, height: f32, add_contents: impl FnOnce(&mut egui::Ui)) {
    let size = egui::vec2(ui.available_size_before_wrap().x, height);
    let layout = egui::Layout::left_to_right(egui::Align::Center).with_main_wrap(true);
    ui.allocate_ui_with_layout(size, layout, |ui| {
        ui.spacing_mut().item_spacing.y = 0.0; // the height of a row is all there is between rows
        add_contents(ui);
    });
}

fn render(ui: &mut egui::Ui, markdown: &str, line_height: f32) -> Vec<egui::Response> {
    let row_height = egui::TextStyle::Body.resolve(ui.style()).size * line_height;
    let mut links = Vec::new();
    let mut in_code_block = false;
    let mut code = String::new();

    for line in markdown.lines() {
        let trimmed = line.trim_start();

        // e.g., "```hkanno" opens, "```" closes.
        if trimmed.starts_with("```") {
            if in_code_block {
                code_block(ui, code.trim_end_matches('\n'));
                code.clear();
            }
            in_code_block = !in_code_block;
            continue;
        }
        if in_code_block {
            code.push_str(line);
            code.push('\n');
            continue;
        }

        if let Some(heading) = trimmed.strip_prefix("# ") {
            ui.label(egui::RichText::new(heading).strong().size(16.0));
        } else if let Some(item) = trimmed.strip_prefix("- ") {
            text_line(ui, row_height, |ui| {
                ui.label("•");
                inline(ui, item, &mut links);
            });
        } else if trimmed.is_empty() {
            ui.add_space(4.0);
        } else {
            text_line(ui, row_height, |ui| inline(ui, trimmed, &mut links));
        }
    }

    // Unclosed fence: still show what we have.
    if in_code_block && !code.is_empty() {
        code_block(ui, code.trim_end_matches('\n'));
    }
    links
}

fn code_block(ui: &mut egui::Ui, code: &str) {
    egui::Frame::NONE
        .fill(ui.visuals().code_bg_color)
        .inner_margin(egui::Margin::same(4))
        .corner_radius(egui::CornerRadius::same(4))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(code).monospace());
        });
}

/// One line with `` `code` ``, `**bold**` and `[text](url)` spans.
///
/// e.g., `**hkbClipGenerator.mode**: `Enum(Int8)`` -> bold, plain `: `, code.
///
/// # Why no label wraps here
/// A wrapping label in a `horizontal_wrapped` starts at the left of the row, with its first row
/// indented by what is already there, and it drew over that in some popups. Without wrapping, each
/// label takes its own place behind the last one, or on the next row if it doesn't fit. So that a line
/// still breaks between words, [`plain`] adds them one by one.
fn inline(ui: &mut egui::Ui, text: &str, links: &mut Vec<egui::Response>) {
    ui.spacing_mut().item_spacing.x = 0.0;
    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);

    let mut rest = text;
    while !rest.is_empty() {
        // Find the next special span.
        let next = [rest.find('`'), rest.find('['), rest.find("**"), rest.find('⟦')].into_iter().flatten().min();
        let Some(next) = next else {
            plain(ui, rest);
            return;
        };

        if next > 0 {
            plain(ui, &rest[..next]);
        }
        rest = &rest[next..];

        if let Some(after) = rest.strip_prefix('`')
            && let Some(end) = after.find('`')
        {
            ui.label(egui::RichText::new(&after[..end]).code());
            rest = &after[end + 1..];
        } else if let Some(after) = rest.strip_prefix("**")
            && let Some(end) = after.find("**")
        {
            ui.label(egui::RichText::new(&after[..end]).strong());
            rest = &after[end + 2..];
        } else if let Some((role, value, after)) = parse_colored(rest) {
            let color = if role == "class" { class_color(ui.visuals()) } else { name_color(ui.visuals()) };
            ui.colored_label(color, value);
            rest = after;
        } else if let Some((label, url, after)) = parse_link(rest) {
            let link = ui.hyperlink_to(label, url);
            if link.has_focus() {
                // Links have no focus look of their own; keyboard users need to see it.
                let stroke = ui.visuals().selection.stroke;
                ui.painter().rect_stroke(link.rect.expand(1.0), 2.0, stroke, egui::StrokeKind::Outside);
            }
            links.push(link);
            rest = after;
        } else {
            // A lone `` ` ``, `[` or `*`: print it and move on.
            let (c, after) = rest.split_at(rest.chars().next().map_or(0, char::len_utf8));
            ui.label(c);
            rest = after;
        }
    }
}

/// Plain text, which can break after each space. Only while it is shown, so few labels per frame.
///
/// e.g. `#0106, #0107` -> `#0106, ` and `#0107`
fn plain(ui: &mut egui::Ui, text: &str) {
    for word in text.split_inclusive(' ') {
        ui.label(word);
    }
}

/// `⟦class|hkbFoo⟧rest` -> `(class, hkbFoo, rest)`
fn parse_colored(text: &str) -> Option<(&str, &str, &str)> {
    let (role, after_role) = text.strip_prefix('⟦')?.split_once('|')?;
    let (value, rest) = after_role.split_once('⟧')?;
    Some((role, value, rest))
}

/// `[label](url)rest` -> `(label, url, rest)`
fn parse_link(text: &str) -> Option<(&str, &str, &str)> {
    let after_open = text.strip_prefix('[')?;
    let (label, after_label) = after_open.split_once("](")?;
    let (url, rest) = after_label.split_once(')')?;
    Some((label, url, rest))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The screen rects of each row of text, per label, in each frame of `markdown` drawn in a popup
    /// like the hover popup of the editor: its width comes from its content, up to `width`.
    fn text_rows(text: &str, width: f32) -> Vec<Vec<Vec<egui::Rect>>> {
        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1200.0, 900.0));
        let mut frames = Vec::new();
        for _ in 0..4 {
            let input = egui::RawInput { screen_rect: Some(screen), ..Default::default() };
            let mut output = ctx.run_ui(input, |ui| {
                egui::Area::new(egui::Id::NULL.with("popup"))
                    .order(egui::Order::Tooltip)
                    .fixed_pos(egui::pos2(20.0, 20.0))
                    .constrain(true)
                    .show(ui.ctx(), |ui| {
                        egui::Frame::popup(ui.style()).show(ui, |ui| {
                            egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                                ui.set_max_width(width);
                                markdown(ui, text);
                            });
                        });
                    });
            });
            output.textures_delta.clear();
            let rows: Vec<Vec<egui::Rect>> = output
                .shapes
                .iter()
                .filter_map(|clipped| match &clipped.shape {
                    egui::Shape::Text(text) => Some(
                        text.galley
                            .rows
                            .iter()
                            .filter(|row| !row.row.glyphs.is_empty())
                            .map(|row| row.rect_without_leading_space().translate(text.pos.to_vec2()))
                            .collect(),
                    ),
                    _ => None,
                })
                .collect();
            frames.push(rows);
        }
        frames
    }

    /// e.g. `#0099: hkbClipGenerator Idle ← #0139, #0179`: a long text after other text in a line.
    /// Whether its first word fits behind the others depends on their widths, so that this happened
    /// only with some objects.
    #[test]
    fn text_never_overlaps() {
        let references: Vec<String> = (0..12).map(|i| format!("#bfco${}", 99 + i * 40)).collect();
        for name_len in 1..70 {
            let name = "n".repeat(name_len);
            let text = format!(
                "**#bfco$99**: ⟦class|hkbStateMachineStateInfo⟧ ⟦name|{name}⟧ ← {}\n\n`code` and more",
                references.join(", ")
            );
            for labels in text_rows(&text, 480.0) {
                for (i, label) in labels.iter().enumerate() {
                    for other in &labels[i + 1..] {
                        for (a, b) in label.iter().flat_map(|a| other.iter().map(move |b| (a, b))) {
                            let overlap = a.intersect(*b);
                            assert!(
                                overlap.width() < 1.0 || overlap.height() < 1.0,
                                "name of {name_len}: {a:?} and {b:?} overlap"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn words_are_as_far_apart_as_in_one_label() {
        let ctx = egui::Context::default();
        let (mut one_label, mut rows) = (0.0, Vec::new());
        for _ in 0..2 {
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                let font = egui::TextStyle::Body.resolve(ui.style());
                one_label =
                    ui.painter().layout_no_wrap("one two three".to_owned(), font, egui::Color32::WHITE).size().x;
                ui.set_max_width(400.0);
                markdown(ui, "one two three");
            });
            output.textures_delta.clear();
            rows = output
                .shapes
                .iter()
                .filter_map(|clipped| match &clipped.shape {
                    egui::Shape::Text(text) => Some(text.galley.rect.translate(text.pos.to_vec2())),
                    _ => None,
                })
                .collect();
        }
        assert_eq!(rows.len(), 3, "{rows:?}");
        let width = rows[2].right() - rows[0].left();
        assert!((width - one_label).abs() < 1.0, "{width} is not {one_label}");
        assert!(rows.iter().all(|row| (row.top() - rows[0].top()).abs() < 1.0), "one row: {rows:?}");
    }

    /// The tops of the rows of `text`, which breaks into many at this width.
    fn row_tops(ctx: &egui::Context, line_height: f32) -> Vec<f32> {
        let mut tops = Vec::new();
        for _ in 0..2 {
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                ui.set_max_width(60.0);
                Markdown::new("one two three four five six").line_height(line_height).show(ui);
            });
            output.textures_delta.clear();
            tops = output
                .shapes
                .iter()
                .filter_map(|clipped| match &clipped.shape {
                    egui::Shape::Text(text) => Some(text.pos.y),
                    _ => None,
                })
                .collect();
        }
        tops.dedup_by(|a, b| (*a - *b).abs() < 1.0);
        tops
    }

    #[test]
    fn rows_are_as_far_apart_as_the_line_height_says() {
        let ctx = egui::Context::default();
        let font_size = ctx.style_of(egui::Theme::Dark).text_styles[&egui::TextStyle::Body].size;
        for line_height in [1.5, 2.0] {
            let expected = font_size * line_height;
            let tops = row_tops(&ctx, line_height);
            assert!(tops.len() > 2, "{tops:?}");
            for pair in tops.windows(2) {
                let distance = pair[1] - pair[0];
                assert!((distance - expected).abs() < 1.0, "{line_height}: {tops:?}");
            }
        }
    }

    #[test]
    fn a_line_breaks_between_words_when_it_is_narrow() {
        let frames = text_rows("one two three four five six seven eight nine ten", 80.0);
        let labels = frames.last().unwrap();
        let tops: Vec<f32> = labels.iter().flatten().map(|row| row.top()).collect();
        assert!(tops.windows(2).any(|pair| pair[1] > pair[0] + 1.0), "more than one row: {tops:?}");
    }
}
