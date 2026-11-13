//! Find bar and "go to line" for [`Keymap::Normal`](crate::Keymap::Normal) (VS Code keys).
//! Vim has its own `/`, `n`, `N`, `*` and `:<line>` (see `keymap::vim::prompt`).
//!
//! ```text
//!                                   ┌──────────────────────────────────┐
//!                                   │ [animmotion        ] 2/5  ↑ ↓ ✖ │  <- top right of the editor
//!                                   └──────────────────────────────────┘
//! ```
//!
//! | Key                       | Action                                            |
//! | ------------------------- | ------------------------------------------------- |
//! | `Ctrl+F`                  | Find (pre-filled with the selection)              |
//! | `Enter` / `Shift+Enter`   | Next / previous match (in the find field)         |
//! | `F3` / `Shift+F3`         | Next / previous match (anywhere)                  |
//! | `Ctrl+F3`                 | Find the word under the cursor                    |
//! | `Ctrl+G`                  | Go to line                                        |
//! | `Esc`                     | Close                                             |
//!
//! Matching: text, or a regex with `.*` on; smartcase (see [`crate::search`]).

use std::ops::Range;

use egui::{Key, Modifiers};

use crate::{
    Keymap,
    search::{Matcher, is_word, next_match},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    Find,
    GotoLine,
}

/// State of the bar (one per document).
#[derive(Debug, Default)]
pub(crate) struct FindBar {
    /// `Some` while the bar is shown.
    pub(crate) open: Option<Kind>,
    pub(crate) query: String,
    pub(crate) line: String,
    /// Focus the input field on the next draw.
    pub(crate) focus_field: bool,
    /// Char index the incremental search starts from (the cursor when the bar opened).
    pub(crate) origin: usize,
    /// The match selected last. Remembered here because egui collapses the selection of an
    /// unfocused `TextEdit` to its end, so the editor's selection cannot tell where it starts.
    pub(crate) current: Option<Range<usize>>,
    /// `.*`: the query is a regex (as VS Code's toggle). Off: text.
    pub(crate) regex: bool,
}

impl FindBar {
    /// The query as [`Self::regex`] says to read it.
    pub(crate) fn matcher(&self) -> Option<Matcher> {
        Matcher::with_options(&self.query, false, self.regex)
    }

    /// The matches to highlight while the find bar is open.
    pub(crate) fn highlight(&self) -> Option<Matcher> {
        (self.open == Some(Kind::Find)).then(|| self.matcher()).flatten()
    }

    /// Opens the find bar, pre-filled with a one-line selection like VS Code.
    pub(crate) fn open_find(&mut self, ctx: &egui::Context, id: egui::Id, text: &str) {
        let selection = selection(ctx, id);
        if let Some(range) = selection.clone().filter(|r| !r.is_empty()) {
            let selected: String = text.chars().skip(range.start).take(range.len()).collect();
            if !selected.contains('\n') {
                self.query = selected;
            }
        }
        self.open_as(Kind::Find, selection.as_ref());
    }

    pub(crate) fn open_go_to_line(&mut self, ctx: &egui::Context, id: egui::Id) {
        self.line.clear();
        self.open_as(Kind::GotoLine, selection(ctx, id).as_ref());
    }

    /// Global keys. Call before the `TextEdit`. `editor_focus`: the editor has the keyboard.
    ///
    /// Only for [`Keymap::Normal`]: Vim uses these keys itself (e.g., `Ctrl+F` = page down).
    ///
    /// With `requests` (see [`crate::CodeEditor::requests`]) `Ctrl+F` / `Ctrl+G` are the host's: what it
    /// asked for. `F3` and its variants stay the find bar's.
    pub(crate) fn handle_keys(
        &mut self,
        keymap: Keymap,
        ui: &egui::Ui,
        id: egui::Id,
        text: &str,
        (editor_focus, requests): (bool, Option<&[crate::EditorRequest]>),
    ) {
        if keymap != Keymap::Normal {
            self.open = None; // e.g., switched to Vim while the bar was open
            return;
        }
        if !editor_focus && self.open.is_none() {
            return;
        }
        // NOTE: `consume_key` ignores extra Shift/Alt: the Shift variant must be tried first.
        let own = requests.is_none();
        let (find, goto, word, prev, next, escape) = ui.input_mut(|i| {
            (
                if own {
                    i.consume_key(Modifiers::COMMAND, Key::F)
                } else {
                    crate::request::asked(requests, crate::EditorRequest::Find)
                },
                if own {
                    i.consume_key(Modifiers::COMMAND, Key::G)
                } else {
                    crate::request::asked(requests, crate::EditorRequest::GoToLine)
                },
                i.consume_key(Modifiers::COMMAND, Key::F3),
                i.consume_key(Modifiers::SHIFT, Key::F3),
                i.consume_key(Modifiers::NONE, Key::F3),
                // egui drops the editor focus on `Esc` itself; just close the bar.
                editor_focus && i.key_pressed(Key::Escape),
            )
        });
        let selection = selection(ui.ctx(), id);

        if find {
            self.open_find(ui.ctx(), id, text);
        }
        if goto {
            self.open_go_to_line(ui.ctx(), id);
        }
        if word && let Some(at) = selection.as_ref().map(|r| r.start) {
            let chars: Vec<char> = text.chars().collect();
            if let Some(word) = word_at(&chars, at) {
                self.query = word;
                self.open_as(Kind::Find, selection.as_ref());
                self.focus_field = false; // keep typing in the editor
                self.jump(ui.ctx(), id, text, true);
            }
        }
        if next || prev {
            self.jump(ui.ctx(), id, text, next);
        }
        if escape {
            self.open = None;
        }
    }

    pub(crate) fn open_as(&mut self, kind: Kind, selection: Option<&Range<usize>>) {
        self.open = Some(kind);
        self.focus_field = true;
        self.origin = selection.map_or(0, |r| r.start);
    }

    /// While typing: select the first match at or after where the search started.
    pub(crate) fn select_from_origin(&mut self, ctx: &egui::Context, id: egui::Id, text: &str) {
        let Some(matcher) = self.matcher() else { return };
        let matches = matcher.find_all(text);
        let at = matches.iter().find(|m| m.start >= self.origin).or_else(|| matches.first());
        if let Some(range) = at.cloned() {
            self.select(ctx, id, range);
        }
    }

    /// Selects the next (`forward`) or previous match, wrapping around. It starts from the
    /// current match, or from the cursor once the user moved it.
    pub(crate) fn jump(&mut self, ctx: &egui::Context, id: egui::Id, text: &str, forward: bool) {
        let Some(matcher) = self.matcher() else { return };
        let from = self.current_match(ctx, id, text).or_else(|| selection(ctx, id)).map_or(0, |r| r.start);
        if let Some(range) = next_match(&matcher.find_all(text), from, forward) {
            self.select(ctx, id, range);
        }
    }

    pub(crate) fn select(&mut self, ctx: &egui::Context, id: egui::Id, range: Range<usize>) {
        select(ctx, id, range.clone());
        self.current = Some(range);
    }

    /// [`Self::current`] while it is still valid: the cursor is at its end (not moved by the
    /// user) and the text there still matches.
    pub(crate) fn current_match(&self, ctx: &egui::Context, id: egui::Id, text: &str) -> Option<Range<usize>> {
        let current = self.current.clone()?;
        let cursor = selection(ctx, id)?;
        (cursor.end == current.end && self.matcher()?.find_all(text).contains(&current)).then_some(current)
    }
}

/// The selection of the editor `id` as a sorted char range.
pub(crate) fn selection(ctx: &egui::Context, id: egui::Id) -> Option<Range<usize>> {
    let range = egui::TextEdit::load_state(ctx, id)?.cursor.char_range()?;
    let (a, b) = (range.primary.index.0, range.secondary.index.0);
    Some(a.min(b)..a.max(b))
}

/// Selects `range` in the editor and scrolls it into view, **without** taking the focus
/// (the user keeps typing in the find field).
pub(crate) fn select(ctx: &egui::Context, id: egui::Id, range: Range<usize>) {
    use egui::text::{CCursor, CCursorRange};
    let mut state = egui::TextEdit::load_state(ctx, id).unwrap_or_default();
    state.cursor.set_char_range(Some(CCursorRange::two(CCursor::new(range.start), CCursor::new(range.end))));
    state.store(ctx, id);
    ctx.data_mut(|data| data.insert_temp(crate::scroll_request_id(id), true));
}

/// The word (`[A-Za-z0-9_]+`, Unicode letters too) at or right before `at`.
fn word_at(chars: &[char], at: usize) -> Option<String> {
    let at = if chars.get(at).is_some_and(|&c| is_word(c)) { at } else { at.checked_sub(1)? };
    if !is_word(*chars.get(at)?) {
        return None;
    }
    let start = chars[..at].iter().rposition(|&c| !is_word(c)).map_or(0, |i| i + 1);
    let end = chars[at..].iter().position(|&c| !is_word(c)).map_or(chars.len(), |i| at + i);
    Some(chars[start..end].iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jump_both_ways() {
        let ctx = egui::Context::default();
        let id = egui::Id::unique("editor");
        let text = "ab x ab x ab";
        let mut bar = FindBar { query: "ab".to_string(), ..Default::default() };
        // What an unfocused egui `TextEdit` does every frame: the selection shrinks to its end.
        let collapse = || {
            let end = selection(&ctx, id).unwrap().end;
            select(&ctx, id, end..end);
        };

        bar.select(&ctx, id, 5..7); // 2nd match
        collapse();
        bar.jump(&ctx, id, text, true);
        assert_eq!(bar.current, Some(10..12));
        collapse();
        bar.jump(&ctx, id, text, false);
        assert_eq!(bar.current, Some(5..7));
        collapse();
        bar.jump(&ctx, id, text, false);
        assert_eq!(bar.current, Some(0..2));
    }

    #[test]
    fn word_under_cursor() {
        let chars: Vec<char> = "0.5 animmotion 1".chars().collect();
        assert_eq!(word_at(&chars, 6).as_deref(), Some("animmotion"));
        assert_eq!(word_at(&chars, 14).as_deref(), Some("animmotion")); // right after the word
        assert_eq!(word_at(&chars, 3), Some("5".to_string()));
    }
}
