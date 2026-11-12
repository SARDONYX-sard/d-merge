//! Text search shared by the Vim `/` search, the `Ctrl+F` find bar and the match highlighting.
//!
//! | Where                  | Pattern                                             |
//! | ---------------------- | --------------------------------------------------- |
//! | Vim `/` `?` `:s`       | a regex (Rust `regex` syntax), as Vim's are          |
//! | Vim `*` `#`            | the word under the cursor, as text, whole words only |
//! | find bar (`Ctrl+F`)    | text, or a regex with `.*` on (as VS Code)           |
//!
//! - Case: "smartcase" like Vim's `:set ignorecase smartcase`: a pattern without uppercase letters
//!   ignores case (`abc` finds `ABC`), otherwise it does not. In a regex, the letter after `\` is not
//!   counted (`\W` is not uppercase).
//! - A regex that does not compile yet (e.g. `[ab` while typing) is searched as text.
//! - Empty matches (e.g. of `a*`) are not matches: there is nothing to go to or to show.
//!
//! Matches are char ranges (not byte ranges), as the editor's positions are.

use std::{
    hash::{Hash, Hasher},
    ops::Range,
};

use regex::{Regex, RegexBuilder};

/// A compiled search pattern.
#[derive(Debug, Clone)]
pub(crate) struct Matcher {
    regex: Regex,
    /// What it was made from, for [`Hash`] (a `Regex` has none).
    source: (Box<str>, bool, bool),
}

impl Hash for Matcher {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.source.hash(state);
    }
}

impl Matcher {
    /// `pattern` as a regex. See [`Self::with_options`].
    pub(crate) fn regex(pattern: &str) -> Option<Self> {
        Self::with_options(pattern, false, true)
    }

    /// `None` for an empty pattern (it would match everywhere).
    ///
    /// `whole_word`: only where the match is not part of a longer word (e.g. `foo` not in `foobar`).
    /// `is_regex`: `pattern` is a regex, else text.
    pub(crate) fn with_options(pattern: &str, whole_word: bool, is_regex: bool) -> Option<Self> {
        if pattern.is_empty() {
            return None;
        }
        let ignore_case = !has_uppercase(pattern, is_regex);
        let build = |body: &str| {
            let body = if whole_word { format!(r"\b(?:{body})\b") } else { body.to_owned() };
            RegexBuilder::new(&body).case_insensitive(ignore_case).build().ok()
        };
        let regex = is_regex.then(|| build(pattern)).flatten().or_else(|| build(&regex::escape(pattern)))?;
        Some(Self { regex, source: (pattern.into(), whole_word, is_regex) })
    }

    /// Char ranges of all (non-overlapping, non-empty) matches in `text`.
    ///
    /// e.g., `"ab ab"` with `ab` -> `[0..2, 3..5]`; `"é ab"` with `ab` -> `[2..4]` (chars, not bytes)
    pub(crate) fn find_all(&self, text: &str) -> Vec<Range<usize>> {
        let mut found = Vec::new();
        // Byte offsets of the matches go up: one pass over the chars turns them into char offsets.
        let mut chars = text.char_indices().map(|(byte, _)| byte).enumerate().peekable();
        let mut to_char = |byte: usize| {
            while chars.next_if(|&(_, at)| at < byte).is_some() {}
            chars.peek().map_or_else(|| text.chars().count(), |&(char_at, _)| char_at)
        };
        for found_match in self.regex.find_iter(text).filter(|m| !m.is_empty()) {
            let start = to_char(found_match.start());
            let end = to_char(found_match.end());
            found.push(start..end);
        }
        found
    }

    /// [`Self::find_all`] of a text held as chars (the Vim buffer).
    pub(crate) fn find_all_chars(&self, chars: &[char]) -> Vec<Range<usize>> {
        self.find_all(&chars.iter().collect::<String>())
    }
}

/// The next match after (`forward`) or before `from`, wrapping around. `matches`: see
/// [`Matcher::find_all`]. It starts next to `from`, so that `n` (or `F3`) on a match moves to the
/// next one; the match at `from` is found last.
///
/// e.g. matches at 0 and 5: from 0 forward -> 5, from 5 forward -> 0 (wraps).
pub(crate) fn next_match(matches: &[Range<usize>], from: usize, forward: bool) -> Option<Range<usize>> {
    let found = if forward {
        matches.iter().find(|m| m.start > from).or_else(|| matches.first())
    } else {
        matches.iter().rev().find(|m| m.start < from).or_else(|| matches.last())
    };
    found.cloned()
}

/// 1-based number of the match starting at `at` (for `[2/5]`), or `None` if none does.
pub(crate) fn index_of(matches: &[Range<usize>], at: usize) -> Option<usize> {
    matches.iter().position(|m| m.start == at).map(|i| i + 1)
}

pub(crate) fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Does `pattern` ask for case? In a regex, an escaped letter is a class (`\W`, `\S`), not a letter.
fn has_uppercase(pattern: &str, is_regex: bool) -> bool {
    let mut escaped = false;
    for c in pattern.chars() {
        if escaped {
            escaped = false;
        } else if is_regex && c == '\\' {
            escaped = true;
        } else if c.is_uppercase() {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    impl Matcher {
        /// `pattern` as text.
        fn new(pattern: &str) -> Option<Self> {
            Self::with_options(pattern, false, false)
        }
    }

    #[test]
    fn smartcase() {
        let lower = Matcher::new("ab").unwrap();
        assert_eq!(lower.find_all("Ab ab AB"), vec![0..2, 3..5, 6..8]);

        let upper = Matcher::new("Ab").unwrap();
        assert_eq!(upper.find_all("Ab ab AB"), vec![0..2]);

        assert!(Matcher::new("").is_none());

        let whole = Matcher::with_options("ab", true, false).unwrap();
        assert_eq!(whole.find_all("ab abc xab ab"), vec![0..2, 11..13]);
    }

    #[test]
    fn a_regex_finds_what_it_says() {
        let digits = Matcher::regex(r"#\d+").unwrap();
        assert_eq!(digits.find_all("#0106 #m$0 #12"), vec![0..5, 11..14]);
        // `\W` is a class, not an uppercase letter: case is still ignored.
        assert_eq!(Matcher::regex(r"a\Wb").unwrap().find_all("A-B"), vec![0..3]);
        // As text, the same pattern is only itself.
        assert!(Matcher::new(r"#\d+").unwrap().find_all("#0106").is_empty());
    }

    #[test]
    fn a_regex_not_finished_yet_is_text() {
        let open = Matcher::regex("[ab").unwrap();
        assert_eq!(open.find_all("x[ab"), vec![1..4]);
    }

    #[test]
    fn matches_are_chars_and_never_empty() {
        assert_eq!(Matcher::new("ab").unwrap().find_all("éé ab"), vec![3..5]);
        assert_eq!(Matcher::regex("x*").unwrap().find_all("axxb"), vec![1..3]);
    }

    #[test]
    fn next_and_index() {
        let matches = Matcher::new("ab").unwrap().find_all("ab x ab");
        assert_eq!(next_match(&matches, 0, true), Some(5..7));
        assert_eq!(next_match(&matches, 5, true), Some(0..2)); // wraps
        assert_eq!(next_match(&matches, 5, false), Some(0..2));
        assert_eq!(next_match(&matches, 0, false), Some(5..7)); // wraps backward
        assert_eq!(index_of(&matches, 5), Some(2));
        assert_eq!(index_of(&matches, 1), None);
    }
}
