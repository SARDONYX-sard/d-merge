//! The status bar under an editor, like VS Code's:
//!
//! ```text
//! [ NORMAL ]  recording @a  2d  ✖ 1 ⚠ 2        Ln 12, Col 5 (3 selected)   Tab: 4   UTF-8   LF
//! ```
//! The left part is the keymap's (e.g., Vim's mode badge and `:w`, `/foo [2/5]`); keymaps without
//! modes have none. The right part says where the cursor is and what the text is.

use crate::{CodeEditorState, Keymap, keymap::StatusLine, text_pos::LinePos};

/// How wide a tab counts for `Col`, like the tabs drawn in the editor.
const TAB_WIDTH: usize = 4;

/// Draws the status bar of an editor showing `text`.
///
/// The badge color follows the mode (see [`crate::StatusMode::color`]). Returns the response of the
/// problem counts: a click on them shows or hides the host's Problems panel (see [`crate::problems`]).
pub fn status_line(ui: &mut egui::Ui, state: &CodeEditorState, keymap: Keymap, text: &str) -> egui::Response {
    ui.horizontal(|ui| {
        if let Some(StatusLine { mode, text }) = state.keymap.status(keymap) {
            // Same width for every mode, so the details do not jump around when it changes.
            let badge = format!(" {:^7} ", mode.label());
            let dark_text = egui::Color32::from_rgb(40, 44, 52);
            ui.label(egui::RichText::new(badge).monospace().strong().color(dark_text).background_color(mode.color()));
            ui.label(egui::RichText::new(text).monospace());
        }
        let counts = state.host_problems.unwrap_or(state.status_problems);
        let problem_response = crate::problems::status_counts(ui, counts);

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 16.0;
            ui.weak(line_ending(text));
            ui.weak("UTF-8");
            ui.weak(state.indent.to_string());
            if let Some((cursor, selected)) = state.status_cursor {
                ui.weak(cursor_text(text, cursor, selected));
            }
        });

        problem_response
    })
    .inner
}

/// What the lines of `text` end with: `CRLF` if any line does, else `LF`.
fn line_ending(text: &str) -> &'static str {
    if text.contains("\r\n") { "CRLF" } else { "LF" }
}

/// `Ln 12, Col 5`, with `(3 selected)` if chars are selected. Both are 1-based.
fn cursor_text(text: &str, cursor: LinePos, selected: usize) -> String {
    let line = text.split('\n').nth(cursor.line).unwrap_or_default();
    let column = visual_column(line, cursor.column) + 1;
    let position = format!("Ln {}, Col {column}", cursor.line + 1);
    if selected == 0 { position } else { format!("{position} ({selected} selected)") }
}

/// The on-screen column of char `column`: a tab reaches the next tab stop, as VS Code counts it.
///
/// e.g., `\t\tab`, char 3 -> 9
pub(crate) fn visual_column(line: &str, column: usize) -> usize {
    line.chars()
        .take(column)
        .fold(0, |visual, c| if c == '\t' { (visual / TAB_WIDTH + 1) * TAB_WIDTH } else { visual + 1 })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_columns_like_the_screen() {
        assert_eq!(visual_column("abc", 2), 2);
        assert_eq!(visual_column("\t\tab", 3), 9);
        assert_eq!(visual_column("a\tb", 2), 4);
        assert_eq!(visual_column("ab", 9), 2); // past the end
    }

    #[test]
    fn tells_the_cursor_and_selection() {
        let text = "first\n\tsecond";
        assert_eq!(cursor_text(text, LinePos { line: 1, column: 2 }, 0), "Ln 2, Col 6");
        assert_eq!(cursor_text(text, LinePos { line: 0, column: 0 }, 3), "Ln 1, Col 1 (3 selected)");
    }

    #[test]
    fn tells_the_line_ending() {
        assert_eq!(line_ending("a\nb"), "LF");
        assert_eq!(line_ending("a\r\nb"), "CRLF");
        assert_eq!(line_ending("a"), "LF");
    }
}
