//! Char-level view of the text used by Vim commands.
//!
//! Positions are **char indices** into the whole text (egui's `CCursor` unit).
//! e.g., in `"ab\ncd"`, `c` is index 3 and `\n` is index 2.

/// The text as chars, so motions can index it directly.
pub(super) struct Buffer {
    pub(super) chars: Vec<char>,
}

/// Word class: Vim treats `foo_1` and `.,|` as different words.
#[derive(PartialEq, Eq, Clone, Copy)]
pub(super) enum Class {
    Space,
    Word,
    Punct,
}

/// The class of `c`. `big`: a WORD (`W`, `B`, `E`, `iW`) is any run of non-blanks.
pub(super) fn class(c: char, big: bool) -> Class {
    if c.is_whitespace() {
        Class::Space
    } else if big || c.is_alphanumeric() || c == '_' {
        Class::Word
    } else {
        Class::Punct
    }
}
impl Buffer {
    pub(super) fn new(text: &str) -> Self {
        Self { chars: text.chars().collect() }
    }

    pub(super) fn text(&self) -> String {
        self.chars.iter().collect()
    }

    pub(super) const fn len(&self) -> usize {
        self.chars.len()
    }

    // ── Lines ────────────────────────────────────────────────────────────────

    /// Index of the first char of the line containing `i`.
    pub(super) fn line_start(&self, i: usize) -> usize {
        let i = i.min(self.len());
        self.chars[..i].iter().rposition(|&c| c == '\n').map_or(0, |nl| nl + 1)
    }

    /// Index of the `\n` ending the line containing `i` (or the text length on the last line).
    pub(super) fn line_end(&self, i: usize) -> usize {
        let i = i.min(self.len());
        self.chars[i..].iter().position(|&c| c == '\n').map_or(self.len(), |n| i + n)
    }

    /// Last char of the line (where Normal mode may put the cursor), or the line start if empty.
    pub(super) fn line_last_char(&self, i: usize) -> usize {
        let (start, end) = (self.line_start(i), self.line_end(i));
        if end > start { end - 1 } else { start }
    }

    /// 0-based line number of `i`.
    pub(super) fn line_of(&self, i: usize) -> usize {
        self.chars[..i.min(self.len())].iter().filter(|&&c| c == '\n').count()
    }

    pub(super) fn line_count(&self) -> usize {
        self.chars.iter().filter(|&&c| c == '\n').count() + 1
    }

    /// Start index of line `line` (clamped to the last line).
    pub(super) fn nth_line_start(&self, line: usize) -> usize {
        let mut current = 0;
        for (i, &c) in self.chars.iter().enumerate() {
            if current == line {
                return i;
            }
            if c == '\n' {
                current += 1;
            }
        }
        if current == line { self.len() } else { self.line_start(self.len()) }
    }

    /// First non-blank char of the line containing `i` (`^`).
    pub(super) fn first_non_blank(&self, i: usize) -> usize {
        let (start, end) = (self.line_start(i), self.line_end(i));
        (start..end).find(|&j| !self.chars[j].is_whitespace()).unwrap_or(start)
    }

    /// Keeps the Normal-mode cursor on a char: never on `\n` or past the end (unless the line is empty).
    pub(super) fn clamp_normal(&self, i: usize) -> usize {
        self.line_last_char(i).min(i).max(self.line_start(i))
    }

    // ── Words ────────────────────────────────────────────────────────────────

    /// `w` / `W`: start of the next word.
    pub(super) fn next_word_start(&self, i: usize, big: bool) -> usize {
        let len = self.len();
        if i >= len {
            return len;
        }
        let mut j = i;
        let start_class = class(self.chars[j], big);
        // Skip the rest of the current word...
        if start_class != Class::Space {
            while j < len && class(self.chars[j], big) == start_class {
                j += 1;
            }
        }
        // ...and the following spaces.
        while j < len && class(self.chars[j], big) == Class::Space {
            j += 1;
        }
        j
    }

    /// `b` / `B`: start of the previous word.
    pub(super) fn prev_word_start(&self, i: usize, big: bool) -> usize {
        let mut j = i;
        while j > 0 && class(self.chars[j - 1], big) == Class::Space {
            j -= 1;
        }
        if j == 0 {
            return 0;
        }
        let word_class = class(self.chars[j - 1], big);
        while j > 0 && class(self.chars[j - 1], big) == word_class {
            j -= 1;
        }
        j
    }

    /// `e` / `E`: end (last char) of the current/next word.
    pub(super) fn word_end(&self, i: usize, big: bool) -> usize {
        let len = self.len();
        let mut j = i + 1;
        while j < len && class(self.chars[j], big) == Class::Space {
            j += 1;
        }
        if j >= len {
            return len.saturating_sub(1);
        }
        let word_class = class(self.chars[j], big);
        while j + 1 < len && class(self.chars[j + 1], big) == word_class {
            j += 1;
        }
        j
    }

    /// `ge` / `gE`: end (last char) of the previous word.
    pub(super) fn prev_word_end(&self, i: usize, big: bool) -> usize {
        let Some(&c) = self.chars.get(i) else { return i.saturating_sub(1) };
        let mut j = i;
        // Leave the current word...
        let start_class = class(c, big);
        if start_class != Class::Space {
            while j > 0 && class(self.chars[j], big) == start_class {
                j -= 1;
            }
        }
        // ...and the spaces before it.
        while j > 0 && class(self.chars[j], big) == Class::Space {
            j -= 1;
        }
        j
    }

    /// The word under the cursor for `*` / `#` (or the next one on the line), e.g. `animmotion`.
    pub(super) fn keyword_at(&self, i: usize) -> Option<String> {
        let end_of_line = self.line_end(i);
        let start = (i..end_of_line).find(|&j| class(self.chars[j], false) == Class::Word)?;
        let range = self.word_object(start, false, false);
        Some(self.slice(range))
    }

    // ── Editing ──────────────────────────────────────────────────────────────

    pub(super) fn slice(&self, range: std::ops::Range<usize>) -> String {
        self.chars[range].iter().collect()
    }

    /// Replaces `range` with `with`.
    pub(super) fn replace(&mut self, range: std::ops::Range<usize>, with: &str) {
        self.chars.splice(range, with.chars());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words() {
        let b = Buffer::new("0.5 animmotion x");
        assert_eq!(b.next_word_start(4, false), 15);
        assert_eq!(b.prev_word_start(15, false), 4);
        assert_eq!(b.word_end(4, false), 13);
        assert_eq!(b.word_object(6, false, false), 4..14);
        assert_eq!(b.prev_word_end(15, false), 13); // `ge`
        assert_eq!(b.next_word_start(0, false), 1); // `0` then `.`
        assert_eq!(b.next_word_start(0, true), 4); // WORD `0.5`
        assert_eq!(b.keyword_at(0).as_deref(), Some("0"));
    }

    #[test]
    fn lines() {
        let b = Buffer::new("ab\n\ncd");
        assert_eq!(b.line_end(0), 2);
        assert_eq!(b.line_start(5), 4);
        assert_eq!(b.nth_line_start(1), 3);
        assert_eq!(b.clamp_normal(2), 1); // never on `\n`
        assert_eq!(b.clamp_normal(3), 3); // empty line
    }
}
