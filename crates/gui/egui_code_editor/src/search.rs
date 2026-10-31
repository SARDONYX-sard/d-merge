//! Plain text search (no regex), shared by the Vim `/` search, the `Ctrl+F` find bar and the
//! match highlighting.
//!
//! Case: "smartcase" like Vim's `:set ignorecase smartcase`:
//! a pattern without uppercase letters ignores case (`abc` finds `ABC`), otherwise it does not.
//! Whole-word search (Vim `*` / `#`) only matches where the pattern is not part of a longer word.

use std::ops::Range;

/// A compiled search pattern.
#[derive(Debug, Clone)]
pub(crate) struct Matcher {
    pattern: Vec<char>,
    ignore_case: bool,
    /// Only whole words (e.g., `foo` does not match in `foobar`).
    whole_word: bool,
}

impl Matcher {
    /// `None` for an empty pattern (it would match everywhere).
    pub(crate) fn new(pattern: &str) -> Option<Self> {
        Self::with_options(pattern, false)
    }

    /// `whole_word`: see [`Self::whole_word`].
    pub(crate) fn with_options(pattern: &str, whole_word: bool) -> Option<Self> {
        if pattern.is_empty() {
            return None;
        }
        let ignore_case = !pattern.chars().any(char::is_uppercase);
        let pattern = pattern.chars().map(|c| fold(c, ignore_case)).collect();
        Some(Self { pattern, ignore_case, whole_word })
    }

    /// Does the pattern start at `chars[at]`?
    pub(crate) fn matches_at(&self, chars: &[char], at: usize) -> bool {
        let end = at + self.pattern.len();
        chars.len() >= end
            && self.pattern.iter().zip(&chars[at..]).all(|(&p, &c)| p == fold(c, self.ignore_case))
            && (!self.whole_word
                || (!at.checked_sub(1).is_some_and(|i| is_word(chars[i]))
                    && !chars.get(end).is_some_and(|&c| is_word(c))))
    }

    /// How many matches `chars` has (non-overlapping).
    pub(crate) fn count(&self, chars: &[char]) -> usize {
        let mut n = 0;
        let mut i = 0;
        while i < chars.len() {
            if self.matches_at(chars, i) {
                n += 1;
                i += self.pattern.len();
            } else {
                i += 1;
            }
        }
        n
    }

    /// 1-based number of the match starting at `at` (for `[2/5]`), or `None` if none does.
    pub(crate) fn index_of(&self, chars: &[char], at: usize) -> Option<usize> {
        if !self.matches_at(chars, at) {
            return None;
        }
        Some(self.count(&chars[..at]) + 1)
    }

    /// Char ranges of all (non-overlapping) matches in `line`.
    ///
    /// e.g., `"ab ab"` with `ab` -> `[0..2, 3..5]`
    pub(crate) fn find_all(&self, line: &str) -> Vec<Range<usize>> {
        let chars: Vec<char> = line.chars().collect();
        let mut found = Vec::new();
        let mut i = 0;
        while i < chars.len() {
            if self.matches_at(&chars, i) {
                found.push(i..i + self.pattern.len());
                i += self.pattern.len();
            } else {
                i += 1;
            }
        }
        found
    }
}

/// The next match after (`forward`) or before `from`, wrapping around the text.
pub(crate) fn find_from(
    matcher: &Matcher,
    chars: &[char],
    from: usize,
    forward: bool,
) -> Option<usize> {
    let len = chars.len();
    if len == 0 {
        return None;
    }
    // Start next to the cursor so that `n` (or `F3`) on a match moves to the next one.
    (1..=len)
        .map(|step| if forward { (from + step) % len } else { (from + len - step % len) % len })
        .find(|&at| matcher.matches_at(chars, at))
}
pub(crate) fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Lowercases `c` when ignoring case (only single-char lowercase forms, enough for search).
fn fold(c: char, ignore_case: bool) -> char {
    if ignore_case { c.to_lowercase().next().unwrap_or(c) } else { c }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smartcase() {
        let lower = Matcher::new("ab").unwrap();
        assert_eq!(lower.find_all("Ab ab AB"), vec![0..2, 3..5, 6..8]);

        let upper = Matcher::new("Ab").unwrap();
        assert_eq!(upper.find_all("Ab ab AB"), vec![0..2]);

        assert!(Matcher::new("").is_none());

        let whole = Matcher::with_options("ab", true).unwrap();
        assert_eq!(whole.find_all("ab abc xab ab"), vec![0..2, 11..13]);
        let chars: Vec<char> = "ab x ab".chars().collect();
        assert_eq!(lower.count(&chars), 2);
        assert_eq!(lower.index_of(&chars, 5), Some(2));
    }
}
