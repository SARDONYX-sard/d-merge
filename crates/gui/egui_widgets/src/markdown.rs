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
pub const fn class_color(visuals: &egui::Visuals) -> egui::Color32 {
    if visuals.dark_mode { egui::Color32::from_rgb(86, 182, 194) } else { egui::Color32::from_rgb(0, 110, 130) }
}

/// The color of an object's name in lists and tooltips.
pub const fn name_color(visuals: &egui::Visuals) -> egui::Color32 {
    if visuals.dark_mode { egui::Color32::from_rgb(229, 192, 123) } else { egui::Color32::from_rgb(150, 90, 0) }
}

/// Renders `markdown` into `ui`.
///
/// Returns the responses of the links, in order, e.g. to focus the first one for keyboard use
/// (a focused link opens with `Enter`; `Tab` moves to the next one).
pub fn markdown(ui: &mut egui::Ui, markdown: &str) -> Vec<egui::Response> {
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
            ui.horizontal_wrapped(|ui| {
                ui.label("•");
                inline(ui, item, &mut links);
            });
        } else if trimmed.is_empty() {
            ui.add_space(4.0);
        } else {
            ui.horizontal_wrapped(|ui| inline(ui, trimmed, &mut links));
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
fn inline(ui: &mut egui::Ui, text: &str, links: &mut Vec<egui::Response>) {
    ui.spacing_mut().item_spacing.x = 0.0;

    let mut rest = text;
    while !rest.is_empty() {
        // Find the next special span.
        let next = [rest.find('`'), rest.find('['), rest.find("**"), rest.find('⟦')].into_iter().flatten().min();
        let Some(next) = next else {
            ui.label(rest);
            return;
        };

        if next > 0 {
            ui.label(&rest[..next]);
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
