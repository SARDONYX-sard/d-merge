//! Tiny Markdown renderer for tooltips (hover docs, completion docs).
//!
//! Supports only what the hkanno docs use:
//! - `# heading`
//! - `- list item`
//! - fenced code blocks (```` ``` ````)
//! - inline `` `code` `` and `[text](url)` links
//!
//! Anything else is shown as plain text. A full CommonMark renderer would be overkill here.

/// Renders `markdown` into `ui`.
pub fn markdown(ui: &mut egui::Ui, markdown: &str) {
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
                inline(ui, item);
            });
        } else if trimmed.is_empty() {
            ui.add_space(4.0);
        } else {
            ui.horizontal_wrapped(|ui| inline(ui, trimmed));
        }
    }

    // Unclosed fence: still show what we have.
    if in_code_block && !code.is_empty() {
        code_block(ui, code.trim_end_matches('\n'));
    }
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

/// One line with `` `code` `` and `[text](url)` spans.
fn inline(ui: &mut egui::Ui, text: &str) {
    ui.spacing_mut().item_spacing.x = 0.0;

    let mut rest = text;
    while !rest.is_empty() {
        // Find the next special span.
        let next_code = rest.find('`');
        let next_link = rest.find('[');
        let next = match (next_code, next_link) {
            (Some(c), Some(l)) => c.min(l),
            (Some(i), None) | (None, Some(i)) => i,
            (None, None) => {
                ui.label(rest);
                return;
            }
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
        } else if let Some((label, url, after)) = parse_link(rest) {
            ui.hyperlink_to(label, url);
            rest = after;
        } else {
            // A lone `` ` `` or `[`: print it and move on.
            let (c, after) = rest.split_at(1);
            ui.label(c);
            rest = after;
        }
    }
}

/// `[label](url)rest` -> `(label, url, rest)`
fn parse_link(text: &str) -> Option<(&str, &str, &str)> {
    let after_open = text.strip_prefix('[')?;
    let (label, after_label) = after_open.split_once("](")?;
    let (url, rest) = after_label.split_once(')')?;
    Some((label, url, rest))
}
