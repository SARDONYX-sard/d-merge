//! Text objects (`iw`, `a(`, `i"`, `ip`, ...): the range an operator acts on, around the cursor.
//!
//! `inner` (`i`) excludes the delimiters / trailing space, `around` (`a`) includes them.
//! e.g., in `f(a, b)` with the cursor on `a`: `i(` = `a, b`, `a(` = `(a, b)`

use std::ops::Range;

use super::buffer::{Buffer, class};

impl Buffer {
    /// `iw` / `aw` (`big`: `iW` / `aW`): the word under `i`; `around` also takes the
    /// following blanks.
    pub(super) fn word_object(&self, i: usize, around: bool, big: bool) -> Range<usize> {
        let len = self.len();
        if i >= len {
            return len..len;
        }
        let word_class = class(self.chars[i], big);
        let same = |c: char| class(c, big) == word_class && c != '\n';
        let (mut start, mut end) = (i, i + 1);
        while start > 0 && same(self.chars[start - 1]) {
            start -= 1;
        }
        while end < len && same(self.chars[end]) {
            end += 1;
        }
        if around {
            while end < len && matches!(self.chars[end], ' ' | '\t') {
                end += 1;
            }
        }
        start..end
    }

    /// `ip` / `ap`: the block of lines around `i` that are all blank or all non-blank.
    /// `around` also takes the blank lines after it. Returns a linewise range
    /// (first line start .. last line start).
    pub(super) fn paragraph_object(&self, i: usize, around: bool) -> Range<usize> {
        let last_line = self.line_count() - 1;
        let blank = |line: usize| self.is_blank_line(self.nth_line_start(line));
        let line = self.line_of(i);
        let kind = blank(line);

        let mut first = line;
        while first > 0 && blank(first - 1) == kind {
            first -= 1;
        }
        let mut last = line;
        while last < last_line && blank(last + 1) == kind {
            last += 1;
        }
        if around {
            while last < last_line && blank(last + 1) != kind {
                last += 1;
            }
        }
        self.nth_line_start(first)..self.nth_line_start(last)
    }

    /// `i"` / `a"`: the quoted string on the line that contains `i` (or the next one).
    pub(super) fn quote_object(&self, i: usize, quote: char, around: bool) -> Option<Range<usize>> {
        let (start, end) = (self.line_start(i), self.line_end(i));
        // Quotes pair up left to right: 1st-2nd, 3rd-4th, ...
        let quotes: Vec<usize> = (start..end).filter(|&j| self.chars[j] == quote).collect();
        let &[open, close] = quotes.as_chunks::<2>().0.iter().find(|[_, close]| i <= *close)?;

        Some(if around {
            let mut close = close + 1;
            while close < end && matches!(self.chars[close], ' ' | '\t') {
                close += 1;
            }
            open..close
        } else {
            open + 1..close
        })
    }

    /// `i(` / `a(` (and `[`, `{`, `<`): the innermost pair around `i`, across lines.
    pub(super) fn pair_object(
        &self,
        i: usize,
        open: char,
        close: char,
        around: bool,
    ) -> Option<Range<usize>> {
        let open_at = self.enclosing_open(i, open, close)?;
        let close_at = self.matching_close(open_at, open, close)?;
        if around {
            return Some(open_at..close_at + 1);
        }
        // A block on its own lines: `{\n  body\n}` -> inner is just the body lines.
        let mut start = open_at + 1;
        let mut end = close_at;
        if self.chars.get(start) == Some(&'\n') {
            start += 1;
        }
        let close_line_start = self.line_start(close_at);
        if close_line_start > start
            && self.chars[close_line_start..close_at].iter().all(|c| c.is_whitespace())
        {
            end = close_line_start;
        }
        Some(start..end.max(start))
    }

    /// The unmatched `open` at or before `i`.
    fn enclosing_open(&self, i: usize, open: char, close: char) -> Option<usize> {
        if self.chars.get(i) == Some(&open) {
            return Some(i);
        }
        let mut depth = 0_usize;
        // On a `close`, start inside its pair.
        let from = if self.chars.get(i) == Some(&close) { i } else { i + 1 };
        for j in (0..from.min(self.len())).rev() {
            let c = self.chars[j];
            if c == close && j != i {
                depth += 1;
            } else if c == open {
                if depth == 0 {
                    return Some(j);
                }
                depth -= 1;
            }
        }
        None
    }

    /// The `close` matching the `open` at `open_at`.
    pub(super) fn matching_close(&self, open_at: usize, open: char, close: char) -> Option<usize> {
        let mut depth = 0_usize;
        for j in open_at + 1..self.len() {
            let c = self.chars[j];
            if c == open {
                depth += 1;
            } else if c == close {
                if depth == 0 {
                    return Some(j);
                }
                depth -= 1;
            }
        }
        None
    }

    /// The `open` matching the `close` at `close_at`.
    pub(super) fn matching_open(&self, close_at: usize, open: char, close: char) -> Option<usize> {
        let mut depth = 0_usize;
        for j in (0..close_at).rev() {
            let c = self.chars[j];
            if c == close {
                depth += 1;
            } else if c == open {
                if depth == 0 {
                    return Some(j);
                }
                depth -= 1;
            }
        }
        None
    }

    /// A line with only blanks (or nothing).
    pub(super) fn is_blank_line(&self, i: usize) -> bool {
        self.chars[self.line_start(i)..self.line_end(i)].iter().all(|c| c.is_whitespace())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn objects() {
        let b = Buffer::new("f(a, (b)) \"x y\" z");
        assert_eq!(b.pair_object(2, '(', ')', false), Some(2..8));
        assert_eq!(b.pair_object(6, '(', ')', true), Some(5..8));
        assert_eq!(b.quote_object(12, '"', false), Some(11..14));
        assert_eq!(b.quote_object(0, '"', true), Some(10..16)); // next quote on the line
        assert_eq!(b.word_object(2, true, false), 2..3);

        let block = Buffer::new("{\n  a\n}");
        assert_eq!(block.pair_object(3, '{', '}', false), Some(2..6));

        let paragraphs = Buffer::new("a\nb\n\nc");
        assert_eq!(paragraphs.paragraph_object(0, false), 0..2);
        assert_eq!(paragraphs.paragraph_object(0, true), 0..4);
    }
}
