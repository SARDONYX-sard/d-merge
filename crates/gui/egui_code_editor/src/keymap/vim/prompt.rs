//! The bottom-line prompts (`:` Ex commands, `/` `?` search), `n` / `N`, `*` / `#` and `gx`.
//!
//! ```text
//! /foo<Enter>   search forward for `foo` (smartcase, plain text), highlight all matches
//! ?foo<Enter>   search backward
//! n / N         next / previous match (wraps around the file); the status shows `[2/5]`
//! * / #         search the word under the cursor (whole word); g* / g# also inside words
//! :noh          hide the highlight until the next search
//! gx            open the URL under the cursor
//! ```
//! Ex commands themselves are in [`super::ex`].

use super::{VimState, buffer::Buffer, exec::Exec, keys::VimKey};
use crate::search::Matcher;

/// What the prompt line is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PromptKind {
    /// `:`
    Ex,
    /// `/`
    SearchForward,
    /// `?`
    SearchBackward,
}

/// The line being typed at the bottom, e.g. `:w` or `/foo`.
#[derive(Debug, Clone)]
pub(super) struct Prompt {
    pub(super) kind: PromptKind,
    pub(super) text: String,
    /// While searching: how many matches the typed pattern has.
    matches: Option<usize>,
}

impl Prompt {
    pub(super) const fn new(kind: PromptKind) -> Self {
        Self { kind, text: String::new(), matches: None }
    }

    /// A prompt with text already typed, e.g. `:'<,'>` from Visual mode.
    pub(super) fn with_text(kind: PromptKind, text: &str) -> Self {
        Self { kind, text: text.to_string(), matches: None }
    }

    const fn is_search(&self) -> bool {
        matches!(self.kind, PromptKind::SearchForward | PromptKind::SearchBackward)
    }

    /// e.g., `:w`, `/foo  [5 matches]`
    pub(super) fn display(&self) -> String {
        let prefix = match self.kind {
            PromptKind::Ex => ':',
            PromptKind::SearchForward => '/',
            PromptKind::SearchBackward => '?',
        };
        match self.matches {
            Some(n) => format!(
                "{prefix}{}    [{n} {}]",
                self.text,
                if n == 1 { "match" } else { "matches" }
            ),
            None => format!("{prefix}{}", self.text),
        }
    }
}

/// The last `/`, `?`, `*` or `#` search, repeated by `n` / `N`.
#[derive(Debug, Clone)]
pub(super) struct LastSearch {
    pub(super) pattern: String,
    /// `true` for `/` and `*`.
    pub(super) forward: bool,
    /// `*` / `#`: whole words only.
    pub(super) whole_word: bool,
}

impl LastSearch {
    fn matcher(&self) -> Option<Matcher> {
        Matcher::with_options(&self.pattern, self.whole_word)
    }
}

impl VimState {
    /// Typing in the prompt: chars, `Backspace`, `Ctrl+U` / `Ctrl+W` (clear / delete a word),
    /// `Enter` (run), `Esc` (cancel).
    pub(super) fn feed_prompt(&mut self, key: VimKey, exec: &mut Exec) {
        let Some(prompt) = &mut self.prompt else { return };
        match key {
            VimKey::Char(c) => prompt.text.push(c),
            VimKey::Backspace => {
                // Like Vim, backspace on an empty prompt leaves it.
                if prompt.text.pop().is_none() {
                    self.prompt = None;
                    return;
                }
            }
            VimKey::Ctrl('u') => prompt.text.clear(),
            VimKey::Ctrl('w') => {
                let kept = prompt.text.trim_end().rfind(' ').map_or(0, |i| i + 1);
                prompt.text.truncate(kept);
            }
            VimKey::Esc => {
                self.prompt = None;
                return;
            }
            VimKey::Enter => {
                let Some(Prompt { kind, text, .. }) = self.prompt.take() else { return };
                match kind {
                    PromptKind::Ex => self.run_ex(text.trim(), exec),
                    PromptKind::SearchForward | PromptKind::SearchBackward => {
                        // An empty pattern repeats the last one, like Vim.
                        let (pattern, whole_word) = match (&self.search, text.is_empty()) {
                            (Some(last), true) => (last.pattern.clone(), last.whole_word),
                            _ => (text, false),
                        };
                        let forward = kind == PromptKind::SearchForward;
                        self.search = Some(LastSearch { pattern, forward, whole_word });
                        self.jump_to_search(true, 1, &exec.buf);
                    }
                }
                return;
            }
            _ => return,
        }
        // Live match count while typing a search (like Vim's `shortmess-S` count).
        if let Some(prompt) = &mut self.prompt
            && prompt.is_search()
        {
            prompt.matches = Matcher::new(&prompt.text).map(|m| m.count(&exec.buf.chars));
        }
    }

    /// `*` / `#` (`whole`), `g*` / `g#`: search the word under the cursor.
    pub(super) fn search_word(&mut self, forward: bool, whole: bool, buf: &Buffer) {
        let Some(word) = buf.keyword_at(self.cursor) else {
            self.message = Some("No string under cursor".to_string());
            return;
        };
        self.search = Some(LastSearch { pattern: word, forward, whole_word: whole });
        self.jump_to_search(true, 1, buf);
    }

    /// Moves to the next match (records a jump), like `n` as a plain command.
    fn jump_to_search(&mut self, same_direction: bool, count: usize, buf: &Buffer) {
        if let Some(at) = self.search_target(same_direction, count, buf) {
            self.marks.push_jump(self.cursor);
            self.cursor = at;
            self.desired_col = None;
        }
    }

    /// `n` (`same_direction: true`) / `N`: the match `count` steps away from the cursor,
    /// wrapping around. Shows `[i/n]` (or "not found") in the status line.
    pub(super) fn search_target(
        &mut self,
        same_direction: bool,
        count: usize,
        buf: &Buffer,
    ) -> Option<usize> {
        let Some(last) = &self.search else {
            self.message = Some("No previous search".to_string());
            return None;
        };
        let matcher = last.matcher()?;
        let forward = last.forward == same_direction;

        let mut at = self.cursor;
        for _ in 0..count {
            let Some(next) = find_from(&matcher, &buf.chars, at, forward) else {
                self.message = Some(format!("Pattern not found: {}", last.pattern));
                return None;
            };
            at = next;
        }
        self.hlsearch = true;
        let total = matcher.count(&buf.chars);
        let index = matcher.index_of(&buf.chars, at).unwrap_or(0);
        let prefix = if last.forward { '/' } else { '?' };
        self.message = Some(format!("{prefix}{}    [{index}/{total}]", last.pattern));
        Some(at)
    }

    /// What to highlight: the pattern being typed after `/`, else the last search
    /// (unless hidden by `:noh`).
    pub(crate) fn search_highlight(&self) -> Option<Matcher> {
        match &self.prompt {
            Some(prompt) if prompt.is_search() => Matcher::new(&prompt.text),
            _ if self.hlsearch => self.search.as_ref().and_then(LastSearch::matcher),
            _ => None,
        }
    }

    /// `gx`: open the URL under the cursor in the browser.
    pub(super) fn open_url(&mut self, exec: &Exec) {
        match url_at(&exec.buf.chars, self.cursor) {
            Some(url) => exec.ctx.open_url(egui::OpenUrl::new_tab(url)),
            None => self.message = Some("No URL under the cursor".to_string()),
        }
    }
}
/// The next match after (`forward`) or before `from`, wrapping around the text.
fn find_from(matcher: &Matcher, chars: &[char], from: usize, forward: bool) -> Option<usize> {
    let len = chars.len();
    if len == 0 {
        return None;
    }
    // Start next to the cursor so that `n` on a match moves to the next one.
    (1..=len)
        .map(|step| if forward { (from + step) % len } else { (from + len - step % len) % len })
        .find(|&at| matcher.matches_at(chars, at))
}

/// The URL around `at`: the non-blank word, without surrounding brackets/quotes and trailing
/// punctuation. e.g., `see <https://example.com/a>.` -> `https://example.com/a`
fn url_at(chars: &[char], at: usize) -> Option<String> {
    let is_delimiter = |c: char| {
        c.is_whitespace() || matches!(c, '"' | '\'' | '<' | '>' | '(' | ')' | '[' | ']' | '`')
    };
    if chars.get(at).is_none_or(|&c| is_delimiter(c)) {
        return None;
    }
    let start = chars[..at].iter().rposition(|&c| is_delimiter(c)).map_or(0, |i| i + 1);
    let end = chars[at..].iter().position(|&c| is_delimiter(c)).map_or(chars.len(), |i| at + i);

    let word: String = chars[start..end].iter().collect();
    let word = word.trim_end_matches(['.', ',', ';', ':', '!', '?']);
    let is_url = ["https://", "http://", "file://"].iter().any(|scheme| word.starts_with(scheme));
    is_url.then(|| word.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chars(s: &str) -> Vec<char> {
        s.chars().collect()
    }

    #[test]
    fn finds_url_under_cursor() {
        let text = chars("see <https://example.com/a>. ok");
        assert_eq!(url_at(&text, 10).as_deref(), Some("https://example.com/a"));
        assert_eq!(url_at(&text, 1), None); // `see`
        assert_eq!(url_at(&text, 3), None); // space
    }

    #[test]
    fn search_wraps_around() {
        let text = chars("ab x ab");
        let m = Matcher::new("ab").unwrap();
        assert_eq!(find_from(&m, &text, 0, true), Some(5));
        assert_eq!(find_from(&m, &text, 5, true), Some(0)); // wraps
        assert_eq!(find_from(&m, &text, 5, false), Some(0));
        assert_eq!(find_from(&m, &text, 0, false), Some(5)); // wraps backward
    }
}
