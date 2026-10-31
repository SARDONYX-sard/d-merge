//! [`Keymap::Normal`](super::Keymap::Normal): plain egui editing plus VS Code's line commands
//! (the Vim `dd`, `ddp`, `yyp`, `o` / `O` of plain editing).
//!
//! | Key                           | Action                                        |
//! | ----------------------------- | --------------------------------------------- |
//! | `Ctrl+Shift+K`                | Delete the line(s)                            |
//! | `Alt+↑` / `Alt+↓`             | Move the line(s) up / down                    |
//! | `Shift+Alt+↑` / `Shift+Alt+↓` | Copy the line(s) up / down                    |
//! | `Ctrl+Enter`                  | Insert a line below                           |
//! | `Ctrl+Shift+Enter`            | Insert a line above                           |
//!
//! "The line(s)": every line the selection touches (a selection ending at column 0 does not
//! include that last line, like VS Code).

use egui::{
    Key, Modifiers,
    text::{CCursor, CCursorRange},
};

use super::{KeymapOutput, Scroll};
use crate::text_pos::LinePos;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LineOp {
    Delete,
    Move { up: bool },
    Copy { up: bool },
    Insert { above: bool },
}

/// Runs the line commands. Call before the `TextEdit` (only while the editor has the focus).
pub(crate) fn handle(ui: &egui::Ui, id: egui::Id, text: &mut String) -> KeymapOutput {
    let shift_alt = Modifiers::SHIFT | Modifiers::ALT;
    let ctrl_shift = Modifiers::COMMAND | Modifiers::SHIFT;
    let op = ui.input_mut(|i| {
        // Most specific modifiers first: `consume_key` ignores extra Shift.
        [
            (ctrl_shift, Key::K, LineOp::Delete),
            (shift_alt, Key::ArrowUp, LineOp::Copy { up: true }),
            (shift_alt, Key::ArrowDown, LineOp::Copy { up: false }),
            (Modifiers::ALT, Key::ArrowUp, LineOp::Move { up: true }),
            (Modifiers::ALT, Key::ArrowDown, LineOp::Move { up: false }),
            (ctrl_shift, Key::Enter, LineOp::Insert { above: true }),
            (Modifiers::COMMAND, Key::Enter, LineOp::Insert { above: false }),
        ]
        .into_iter()
        .find(|&(modifiers, key, _)| i.consume_key(modifiers, key))
        .map(|(_, _, op)| op)
    });
    let Some(op) = op else { return KeymapOutput::default() };

    let Some(range) = egui::TextEdit::load_state(ui.ctx(), id).and_then(|s| s.cursor.char_range())
    else {
        return KeymapOutput::default();
    };
    let primary = LinePos::from_char_index(text, range.primary.index.0);
    let secondary = LinePos::from_char_index(text, range.secondary.index.0);
    let Some((new_text, primary, secondary)) = apply(text, op, primary, secondary) else {
        return KeymapOutput::default();
    };
    *text = new_text;

    let mut state = egui::TextEdit::load_state(ui.ctx(), id).unwrap_or_default();
    state.cursor.set_char_range(Some(CCursorRange::two(
        CCursor::new(secondary.to_char_index(text)),
        CCursor::new(primary.to_char_index(text)),
    )));
    state.store(ui.ctx(), id);
    KeymapOutput { changed: true, scroll: Some(Scroll::IntoView), ..Default::default() }
}

/// The edited text and the new cursor ends, or `None` when nothing changes (e.g., moving the
/// first line up).
fn apply(
    text: &str,
    op: LineOp,
    primary: LinePos,
    secondary: LinePos,
) -> Option<(String, LinePos, LinePos)> {
    let mut lines: Vec<&str> = text.split('\n').collect();
    let (start, end) =
        if primary.line <= secondary.line { (primary, secondary) } else { (secondary, primary) };
    let first = start.line;
    // A selection that ends at column 0 of a later line does not touch that line.
    let last = if end.column == 0 && end.line > start.line { end.line - 1 } else { end.line };
    let count = last - first + 1;
    let shift = |pos: LinePos, by: isize| LinePos { line: pos.line.wrapping_add_signed(by), ..pos };

    match op {
        LineOp::Delete => {
            lines.drain(first..=last);
            if lines.is_empty() {
                lines.push("");
            }
            let cursor = LinePos { line: first.min(lines.len() - 1), column: primary.column };
            Some((lines.join("\n"), cursor, cursor))
        }
        LineOp::Move { up: true } => {
            if first == 0 {
                return None;
            }
            lines[first - 1..=last].rotate_left(1);
            Some((lines.join("\n"), shift(primary, -1), shift(secondary, -1)))
        }
        LineOp::Move { up: false } => {
            if last + 1 >= lines.len() {
                return None;
            }
            lines[first..=last + 1].rotate_right(1);
            Some((lines.join("\n"), shift(primary, 1), shift(secondary, 1)))
        }
        LineOp::Copy { up } => {
            let block: Vec<&str> = lines[first..=last].to_vec();
            lines.splice(last + 1..last + 1, block);
            // Up: the cursor stays on the upper copy; down: it moves to the lower one.
            #[expect(clippy::cast_possible_wrap, reason = "line counts are small")]
            let by = if up { 0 } else { count as isize };
            Some((lines.join("\n"), shift(primary, by), shift(secondary, by)))
        }
        LineOp::Insert { above } => {
            let at = if above { first } else { last + 1 };
            lines.insert(at, "");
            let cursor = LinePos { line: at, column: 0 };
            Some((lines.join("\n"), cursor, cursor))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn pos(line: usize, column: usize) -> LinePos {
        LinePos { line, column }
    }

    fn run(text: &str, op: LineOp, cursor: LinePos) -> Option<(String, LinePos)> {
        apply(text, op, cursor, cursor).map(|(text, primary, _)| (text, primary))
    }

    #[test]
    fn line_ops() {
        let text = "a\nb\nc";
        assert_eq!(run(text, LineOp::Delete, pos(1, 0)), Some(("a\nc".into(), pos(1, 0))));
        assert_eq!(
            run(text, LineOp::Move { up: true }, pos(1, 0)),
            Some(("b\na\nc".into(), pos(0, 0)))
        );
        assert_eq!(
            run(text, LineOp::Move { up: false }, pos(1, 0)),
            Some(("a\nc\nb".into(), pos(2, 0)))
        );
        assert_eq!(run(text, LineOp::Move { up: true }, pos(0, 0)), None);
        assert_eq!(
            run(text, LineOp::Copy { up: false }, pos(1, 1)),
            Some(("a\nb\nb\nc".into(), pos(2, 1)))
        );
        assert_eq!(
            run(text, LineOp::Copy { up: true }, pos(1, 1)),
            Some(("a\nb\nb\nc".into(), pos(1, 1)))
        );
        assert_eq!(
            run(text, LineOp::Insert { above: false }, pos(0, 1)),
            Some(("a\n\nb\nc".into(), pos(1, 0)))
        );
        assert_eq!(run("a", LineOp::Delete, pos(0, 0)), Some((String::new(), pos(0, 0))));
    }

    #[test]
    fn selection_ending_at_column_zero_skips_that_line() {
        // `a\n` selected (to the start of `b`): only `a` moves.
        let (text, primary, secondary) =
            apply("a\nb\nc", LineOp::Move { up: false }, pos(1, 0), pos(0, 0)).unwrap();
        assert_eq!(text, "b\na\nc");
        assert_eq!((primary, secondary), (pos(2, 0), pos(1, 0)));
    }
}
