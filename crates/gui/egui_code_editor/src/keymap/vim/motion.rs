//! Where a [`Motion`] goes (used both to move and as an operator's range).
//!
//! # Range kinds
//! Like Vim, a motion used by an operator (`d`, `c`, `y`, ...) is one of:
//! - **exclusive**: the target char is not included (e.g., `dw` stops before the next word)
//! - **inclusive**: the target char is included (e.g., `de`, `d$`, `df,`)
//! - **linewise**: whole lines (e.g., `dj`, `dG`, `d'a`)

use super::{View, VimState, buffer::Buffer, command::Motion};
use crate::keymap::Scroll;

/// How a motion's range is interpreted by operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    Exclusive,
    Inclusive,
    Linewise,
}

/// The last `f` / `t` / `F` / `T`, for `;` and `,`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Find {
    pub(super) forward: bool,
    pub(super) till: bool,
    pub(super) ch: char,
}

impl VimState {
    /// Where `motion` (repeated `count` times) goes from the cursor, or `None` if it cannot
    /// (e.g., `fx` without an `x`), in which case nothing happens.
    ///
    /// `for_operator`: `l` and `w` may then reach the line end (e.g., `dl` on the last char).
    pub(super) fn motion_target(
        &mut self,
        motion: Motion,
        count: usize,
        explicit_count: bool,
        buf: &Buffer,
        for_operator: bool,
    ) -> Option<(usize, Kind)> {
        let cur = self.cursor;
        let line_end = buf.line_end(cur);
        let repeat = |mut i: usize, step: &dyn Fn(usize) -> usize| {
            for _ in 0..count {
                i = step(i);
            }
            i
        };
        // `count` lines down/up -> first non-blank (`+`, `-`, `_`, `j`/`k` keep the column instead).
        let line_first_non_blank = |line: usize| buf.first_non_blank(buf.nth_line_start(line));
        let last_line = buf.line_count() - 1;

        Some(match motion {
            Motion::Left => (cur.saturating_sub(count).max(buf.line_start(cur)), Kind::Exclusive),
            Motion::Right => {
                let limit = if for_operator { line_end } else { buf.line_last_char(cur) };
                ((cur + count).min(limit), Kind::Exclusive)
            }
            Motion::Down | Motion::Up => {
                let col = *self.desired_col.get_or_insert_with(|| cur - buf.line_start(cur));
                let line = buf.line_of(cur);
                let line =
                    if motion == Motion::Down { (line + count).min(last_line) } else { line.saturating_sub(count) };
                let start = buf.nth_line_start(line);
                ((start + col).min(buf.line_last_char(start)), Kind::Linewise)
            }
            Motion::WordForward { big } => {
                let mut i = repeat(cur, &|i| buf.next_word_start(i, big));
                // `dw` on the last word of a line must not eat the newline.
                if for_operator && i > line_end {
                    i = line_end;
                }
                (i, Kind::Exclusive)
            }
            Motion::WordBackward { big } => (repeat(cur, &|i| buf.prev_word_start(i, big)), Kind::Exclusive),
            Motion::WordEnd { big } => (repeat(cur, &|i| buf.word_end(i, big)), Kind::Inclusive),
            Motion::WordEndBackward { big } => (repeat(cur, &|i| buf.prev_word_end(i, big)), Kind::Inclusive),
            Motion::LineStart => (buf.line_start(cur), Kind::Exclusive),
            Motion::FirstNonBlank => (buf.first_non_blank(cur), Kind::Exclusive),
            Motion::LineEnd => {
                // `3$` = end of the line 2 below.
                let line = (buf.line_of(cur) + count - 1).min(last_line);
                (buf.line_last_char(buf.nth_line_start(line)), Kind::Inclusive)
            }
            Motion::Column => {
                let start = buf.line_start(cur);
                ((start + count - 1).min(buf.line_last_char(start)), Kind::Exclusive)
            }
            Motion::NextLineStart => (line_first_non_blank((buf.line_of(cur) + count).min(last_line)), Kind::Linewise),
            Motion::PrevLineStart => (line_first_non_blank(buf.line_of(cur).saturating_sub(count)), Kind::Linewise),
            Motion::CurrentLineStart => {
                (line_first_non_blank((buf.line_of(cur) + count - 1).min(last_line)), Kind::Linewise)
            }
            Motion::FileStart | Motion::FileEnd => {
                let line = if explicit_count {
                    count - 1
                } else if motion == Motion::FileStart {
                    0
                } else {
                    last_line
                };
                (line_first_non_blank(line.min(last_line)), Kind::Linewise)
            }
            Motion::ParagraphForward => (repeat(cur, &|i| paragraph_boundary(buf, i, true)), Kind::Exclusive),
            Motion::ParagraphBackward => (repeat(cur, &|i| paragraph_boundary(buf, i, false)), Kind::Exclusive),
            Motion::MatchPair => (match_pair(buf, cur, self.options.comment)?, Kind::Inclusive),
            Motion::Find { forward, till, ch } => {
                self.last_find = Some(Find { forward, till, ch });
                find_char(buf, cur, Find { forward, till, ch }, count, false)?
            }
            Motion::RepeatFind { reverse } => {
                let mut find = self.last_find?;
                find.forward ^= reverse;
                find_char(buf, cur, find, count, true)?
            }
            Motion::Mark { name, linewise } => {
                let at = self.marks.get(name)?.min(buf.len());
                if linewise {
                    (buf.first_non_blank(at), Kind::Linewise)
                } else {
                    (buf.clamp_normal(at), Kind::Exclusive)
                }
            }
            Motion::ScreenTop | Motion::ScreenMiddle | Motion::ScreenBottom => {
                let View { first_line, lines } = self.view;
                let last_visible = (first_line + lines.max(1) - 1).min(last_line);
                let line = match motion {
                    Motion::ScreenTop => (first_line + count - 1).min(last_visible),
                    Motion::ScreenBottom => last_visible.saturating_sub(count - 1).max(first_line),
                    _ => first_line + (last_visible - first_line) / 2,
                };
                (line_first_non_blank(line), Kind::Linewise)
            }
            Motion::HalfPage { down } | Motion::Page { down } => {
                let lines = self.view.lines.max(2);
                let step = if matches!(motion, Motion::HalfPage { .. }) {
                    if explicit_count { count } else { lines / 2 }
                } else {
                    count * (lines - 2).max(1)
                };
                let line = buf.line_of(cur);
                let line = if down { (line + step).min(last_line) } else { line.saturating_sub(step) };
                // The view scrolls with the cursor, like Vim.
                let step_i32 = i32::try_from(step).unwrap_or(i32::MAX);
                self.scroll = Some(Scroll::Lines(if down { step_i32 } else { -step_i32 }));
                (line_first_non_blank(line), Kind::Linewise)
            }
            Motion::SearchNext { reverse } => (self.search_target(!reverse, count, buf)?, Kind::Exclusive),
        })
    }
}

/// `}` (`forward`) / `{`: the next / previous blank line after (before) a non-blank one.
fn paragraph_boundary(buf: &Buffer, i: usize, forward: bool) -> usize {
    let last_line = buf.line_count() - 1;
    let blank = |line: usize| buf.is_blank_line(buf.nth_line_start(line));
    let mut line = buf.line_of(i);
    if forward {
        // Skip blank lines, then the paragraph.
        while line < last_line && blank(line) {
            line += 1;
        }
        while line < last_line && !blank(line) {
            line += 1;
        }
        if line == last_line && !blank(line) { buf.len() } else { buf.nth_line_start(line) }
    } else {
        while line > 0 && blank(line) {
            line -= 1;
        }
        while line > 0 && !blank(line) {
            line -= 1;
        }
        buf.nth_line_start(line)
    }
}

/// `%`: from the tag the cursor is in (or the first after it), or the first bracket at or after `i`
/// on the line, to its partner, as VS Code's: a bracket pairs with one of the same place, the code or
/// the same comment / string. e.g. in JSON, the `[` of the code skips the `]` of `// ]`, and the `[`
/// of `// []` pairs with its own `]`. `comment`: the language's.
fn match_pair(buf: &Buffer, i: usize, comment: Option<crate::Comment>) -> Option<usize> {
    // `<tag>` / `</tag>`, as Vim's `matchit`: see `tags`.
    if let Some(at) = super::tags::tag_near(buf, i) {
        return super::tags::matching_tag(buf, at);
    }
    const PAIRS: [(char, char); 3] = [('(', ')'), ('[', ']'), ('{', '}')];
    let places = places(&buf.chars, comment);
    let end = buf.line_end(i);
    let (at, c) = (i..end).map(|j| (j, buf.chars[j])).find(|(_, c)| "()[]{}".contains(*c))?;
    let counts = |j: usize| places[j] == places[at];
    PAIRS.iter().find_map(|&(open, close)| {
        if c == open {
            buf.matching_close_where(at, (open, close), &counts)
        } else if c == close {
            buf.matching_open_where(at, (open, close), &counts)
        } else {
            None
        }
    })
}

/// The open and close marks of a block comment, e.g. `/*` and `*/`.
type Block = (Vec<char>, Vec<char>);

/// The place of each char of `chars`: `0` the code, else the number of the string (`"..."`, to the
/// line's end at most) or the comment of `comment` it is in (a line comment `//` also has its block
/// comments `/* */`, as JSONC and Rust), each its own.
///
/// e.g. `[1, "]", // ]` -> `[1, ` is 0, `"]"` 1, `// ]` 2.
fn places(chars: &[char], comment: Option<crate::Comment>) -> Vec<u32> {
    // A line comment's mark (none: empty), and a block comment's open and close marks.
    let (line, block): (Vec<char>, Option<Block>) = match comment {
        Some(crate::Comment::Line(mark)) => {
            let block = (mark == "//").then(|| (vec!['/', '*'], vec!['*', '/']));
            (mark.chars().collect(), block)
        }
        Some(crate::Comment::Block(open, close)) => {
            (Vec::new(), Some((open.chars().collect(), close.chars().collect())))
        }
        None => (Vec::new(), None),
    };
    let at = |i: usize, mark: &[char]| chars.get(i..i + mark.len()) == Some(mark);

    let mut places = vec![0; chars.len()];
    let mut place = 0;
    let mut i = 0;
    while i < chars.len() {
        let skip_to = if chars[i] == '"' {
            // To the closing quote (an escaped one does not close it), or the line's end.
            let mut j = i + 1;
            while j < chars.len() && chars[j] != '"' && chars[j] != '\n' {
                j += if chars[j] == '\\' { 2 } else { 1 };
            }
            (j + 1).min(chars.len())
        } else if !line.is_empty() && at(i, &line) {
            (i..chars.len()).find(|&j| chars[j] == '\n').unwrap_or(chars.len())
        } else if let Some((open, close)) = block.as_ref().filter(|(open, _)| at(i, open)) {
            let end = (i + open.len()..chars.len()).find(|&j| at(j, close));
            end.map_or(chars.len(), |j| j + close.len())
        } else {
            i += 1;
            continue;
        };
        place += 1;
        places[i..skip_to].fill(place);
        i = skip_to;
    }
    places
}

/// `f`/`t`/`F`/`T` on the current line. `repeating` (`;`, `,`): `t` does not get stuck right
/// before the char it already stopped at.
fn find_char(buf: &Buffer, cur: usize, find: Find, count: usize, repeating: bool) -> Option<(usize, Kind)> {
    let (start, end) = (buf.line_start(cur), buf.line_end(cur));
    let Find { forward, till, ch } = find;
    let mut at = cur;
    for n in 0..count {
        // `t,` then `;`: skip the char right next to the cursor, or it would never move.
        let skip = usize::from(till && (repeating || n > 0));
        at = if forward {
            (at + 1 + skip..end).find(|&j| buf.chars[j] == ch)?
        } else {
            (start..at.saturating_sub(skip)).rev().find(|&j| buf.chars[j] == ch)?
        };
    }
    let at = match (till, forward) {
        (true, true) => at - 1,
        (true, false) => at + 1,
        (false, _) => at,
    };
    Some((at, if forward { Kind::Inclusive } else { Kind::Exclusive }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_and_pairs() {
        let b = Buffer::new("a(b, c) d, e");
        let f = |forward, till, ch| Find { forward, till, ch };
        assert_eq!(find_char(&b, 0, f(true, false, ','), 1, false), Some((3, Kind::Inclusive)));
        assert_eq!(find_char(&b, 0, f(true, false, ','), 2, false), Some((9, Kind::Inclusive)));
        assert_eq!(find_char(&b, 0, f(true, true, ','), 1, false), Some((2, Kind::Inclusive)));
        assert_eq!(find_char(&b, 2, f(true, true, ','), 1, true), Some((8, Kind::Inclusive)));
        assert_eq!(find_char(&b, 9, f(false, false, '('), 1, false), Some((1, Kind::Exclusive)));
        assert_eq!(find_char(&b, 0, f(true, false, 'z'), 1, false), None);

        assert_eq!(match_pair(&b, 0, None), Some(6));
        assert_eq!(match_pair(&b, 6, None), Some(1));

        let p = Buffer::new("a\nb\n\nc\n\nd");
        assert_eq!(paragraph_boundary(&p, 0, true), 4);
        assert_eq!(paragraph_boundary(&p, 4, true), 7);
        assert_eq!(paragraph_boundary(&p, 9, false), 7);
    }

    /// As VS Code's `%`: a bracket of the code skips those of comments and strings; one of a comment
    /// pairs within it.
    #[test]
    fn a_pair_is_of_the_same_place() {
        let line = Some(crate::Comment::Line("//"));
        let b = Buffer::new("[\n  \"a\",\n  // []\n  \"]\", /* ] */\n]");
        let last = b.chars.len() - 1;
        assert_eq!(match_pair(&b, 0, line), Some(last), "the code's `]`");
        assert_eq!(match_pair(&b, last, line), Some(0));
        let in_comment = b.chars.iter().collect::<String>().find("[]").unwrap();
        assert_eq!(match_pair(&b, in_comment, line), Some(in_comment + 1), "`// []` within itself");
    }
}
