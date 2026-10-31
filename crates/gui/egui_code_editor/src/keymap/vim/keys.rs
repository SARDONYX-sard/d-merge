//! Takes keyboard input away from the `TextEdit` and turns it into [`VimKey`]s.
//!
//! In Normal/Visual mode **every** key event is removed from egui's input queue before the
//! `TextEdit` runs, so typing `dd` never inserts text. `Ctrl` shortcuts that Vim does not use
//! (e.g., `Ctrl+S`) are left alone for the app; see [`NORMAL_CTRL`] for the ones Vim takes.
//!
//! In Insert mode only these are taken; everything else is normal typing:
//! - `Esc`, `Ctrl+[`, `Ctrl+C` (arrives as `Event::Copy`) and the 2nd char of an escape sequence (e.g., `k` of `jk`)
//! - `Ctrl+W` / `Ctrl+U` (delete word / line before the cursor), `Ctrl+T` / `Ctrl+D` (indent)

use super::VimOptions;

/// One key press as Vim sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum VimKey {
    /// A printable char, already shifted (e.g., `$`, `G`, ` `).
    Char(char),
    /// `Ctrl` + a letter, lowercase. e.g., `Ctrl('d')` for `Ctrl+D`
    Ctrl(char),
    Esc,
    /// The 2nd char of an Insert-mode escape sequence was typed (e.g., `k` of `jk`).
    /// The 1st char (here `j`) is already in the text and must be removed before leaving.
    SequenceEsc(char),
    Enter,
    Backspace,
    Delete,
    Tab,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
}

/// `Ctrl` + letter combinations Vim uses in Normal/Visual mode. Others go to the app.
///
/// e.g., `d`/`u` half page, `f`/`b` page, `e`/`y` scroll a line, `a`/`x` increment,
/// `o`/`i` jump list, `r` redo, `v` visual, `n`/`p` down/up, `w` (ignored, no windows)
const NORMAL_CTRL: &[char] =
    &['a', 'b', 'd', 'e', 'f', 'i', 'n', 'o', 'p', 'r', 'u', 'v', 'w', 'x', 'y'];

/// `Ctrl` + letter combinations Vim uses in Insert mode.
const INSERT_CTRL: &[char] = &['w', 'u', 't', 'd'];

/// Remembers the last char typed in Insert mode, to detect sequences such as `jk`
/// (Vim users map them with e.g. `inoremap jk <Esc>`).
#[derive(Debug, Default)]
pub(super) struct InsertEscape {
    /// e.g., `('j', 12.3)`: `j` was typed at time 12.3 s.
    last: Option<(char, f64)>,
}

impl InsertEscape {
    pub(super) const fn reset(&mut self) {
        self.last = None;
    }

    /// Called for each char typed in Insert mode.
    /// Returns `true` if it completes an escape sequence (the char must then not be typed).
    pub(super) fn on_char(&mut self, c: char, now: f64, options: &VimOptions) -> bool {
        let completes = self.last.take().is_some_and(|(first, at)| {
            now - at <= options.escape_timeout && options.insert_escapes.contains(&[first, c])
        });
        if !completes && options.insert_escapes.iter().any(|seq| seq[0] == c) {
            self.last = Some((c, now));
        }
        completes
    }
}

/// Removes the keys Vim handles from `ui`'s input and returns them in order.
pub(super) fn take(
    ui: &egui::Ui,
    insert_mode: bool,
    escape_seq: &mut InsertEscape,
    options: &VimOptions,
) -> Vec<VimKey> {
    ui.input_mut(|input| {
        let now = input.time;
        let mut keys = Vec::new();
        input.events.retain(|event| match event {
            egui::Event::Key { key, pressed, modifiers, .. } => {
                let ctrl_letter =
                    (modifiers.ctrl && !modifiers.alt).then(|| letter(*key)).flatten();
                let escape = *key == egui::Key::Escape
                    || (modifiers.ctrl && *key == egui::Key::OpenBracket)
                    || ctrl_letter == Some('c');

                if insert_mode {
                    if escape {
                        if *pressed {
                            keys.push(VimKey::Esc);
                        }
                        return false;
                    }
                    if let Some(c) = ctrl_letter.filter(|c| INSERT_CTRL.contains(c)) {
                        if *pressed {
                            keys.push(VimKey::Ctrl(c));
                        }
                        return false;
                    }
                    // Moving/deleting between the two chars breaks a sequence: `j` <Left> `k`.
                    // (Letter keys also arrive as `Key` events, so only these reset it.)
                    if *pressed && breaks_sequence(*key) {
                        escape_seq.reset();
                    }
                    return true; // keep everything else for the TextEdit
                }

                if escape {
                    if *pressed {
                        keys.push(VimKey::Esc);
                    }
                    return false;
                }
                if modifiers.command {
                    let Some(c) = ctrl_letter.filter(|c| NORMAL_CTRL.contains(c)) else {
                        return true; // app shortcut such as Ctrl+S
                    };
                    if *pressed {
                        keys.push(VimKey::Ctrl(c));
                    }
                    return false;
                }
                if *pressed && let Some(vim_key) = map_key(*key) {
                    keys.push(vim_key);
                }
                false // swallow: Normal mode must not edit text by itself
            }
            egui::Event::Text(text) if !insert_mode => {
                keys.extend(text.chars().map(VimKey::Char));
                false
            }
            egui::Event::Text(text) if insert_mode => {
                let mut chars = text.chars();
                match (chars.next(), chars.next()) {
                    // One typed char (IME commits / pastes of several chars never form a sequence).
                    (Some(c), None) if escape_seq.on_char(c, now, options) => {
                        keys.push(VimKey::SequenceEsc(c));
                        false // swallow the `k` of `jk`
                    }
                    (Some(_), None) => true,
                    _ => {
                        escape_seq.reset();
                        true
                    }
                }
            }
            // egui sends `Ctrl+C` / `Ctrl+X` / `Ctrl+V` as clipboard events, not as keys.
            // `Ctrl+C` leaves Insert/Visual mode like `Esc`.
            egui::Event::Copy => {
                keys.push(VimKey::Esc);
                false
            }
            // Normal mode: `Ctrl+X` decrements, `Ctrl+V` starts a Visual block (Vim uses `p` / `d`
            // for the clipboard). Insert mode keeps cutting / pasting.
            egui::Event::Cut if !insert_mode => {
                keys.push(VimKey::Ctrl('x'));
                false
            }
            egui::Event::Paste(_) if !insert_mode => {
                keys.push(VimKey::Ctrl('v'));
                false
            }
            _ => true,
        });
        keys
    })
}

/// `A`..`Z` -> `'a'`..`'z'`.
fn letter(key: egui::Key) -> Option<char> {
    let mut chars = key.name().chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) if c.is_ascii_alphabetic() => Some(c.to_ascii_lowercase()),
        _ => None,
    }
}

/// Keys that move the cursor or delete text in Insert mode.
const fn breaks_sequence(key: egui::Key) -> bool {
    use egui::Key;
    matches!(
        key,
        Key::ArrowLeft
            | Key::ArrowRight
            | Key::ArrowUp
            | Key::ArrowDown
            | Key::Backspace
            | Key::Delete
            | Key::Enter
            | Key::Tab
            | Key::Home
            | Key::End
    )
}

/// Non-printable keys. Printable ones arrive as `Event::Text`.
const fn map_key(key: egui::Key) -> Option<VimKey> {
    use egui::Key;
    Some(match key {
        Key::Enter => VimKey::Enter,
        Key::Backspace => VimKey::Backspace,
        Key::Delete => VimKey::Delete,
        Key::Tab => VimKey::Tab,
        Key::ArrowLeft => VimKey::Left,
        Key::ArrowRight => VimKey::Right,
        Key::ArrowUp => VimKey::Up,
        Key::ArrowDown => VimKey::Down,
        Key::Home => VimKey::Home,
        Key::End => VimKey::End,
        Key::PageUp => VimKey::PageUp,
        Key::PageDown => VimKey::PageDown,
        _ => return None,
    })
}
