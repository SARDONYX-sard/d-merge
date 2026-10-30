//! Takes keyboard input away from the `TextEdit` and turns it into [`VimKey`]s.
//!
//! In Normal/Visual mode **every** key event is removed from egui's input queue before the
//! `TextEdit` runs, so typing `dd` never inserts text. Shortcuts with Ctrl/Cmd (e.g., `Ctrl+S`)
//! are left alone for the app, except Vim's own `Ctrl+R` (redo).
//!
//! In Insert mode only `Esc` / `Ctrl+[` and the second char of an escape sequence
//! (e.g., the `k` of `jk`, see [`InsertEscape`]) are taken; everything else is normal typing.

use super::VimOptions;

/// One key press as Vim sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum VimKey {
    /// A printable char, already shifted (e.g., `$`, `G`).
    Char(char),
    Esc,
    /// The 2nd char of an Insert-mode escape sequence was typed (e.g., `k` of `jk`).
    /// The 1st char (here `j`) is already in the text and must be removed before leaving.
    SequenceEsc(char),
    Enter,
    Backspace,
    Left,
    Right,
    Up,
    Down,
    /// `Ctrl+R` (redo).
    Redo,
}

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
                let escape =
                    *key == egui::Key::Escape || (modifiers.ctrl && *key == egui::Key::OpenBracket);
                if insert_mode {
                    if escape && *pressed {
                        keys.push(VimKey::Esc);
                    }
                    // Moving/deleting between the two chars breaks a sequence: `j` <Left> `k`.
                    // (Letter keys also arrive as `Key` events, so only these reset it.)
                    if *pressed && breaks_sequence(*key) {
                        escape_seq.reset();
                    }
                    return !escape; // keep everything else for the TextEdit
                }

                if modifiers.command && !(modifiers.ctrl && *key == egui::Key::R) && !escape {
                    return true; // app shortcut such as Ctrl+S
                }
                if *pressed && let Some(vim_key) = map_key(*key, *modifiers, escape) {
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
            // No pasting/cutting into Normal mode (Vim uses `p` / `d`).
            egui::Event::Paste(_) | egui::Event::Cut if !insert_mode => false,
            _ => true,
        });
        keys
    })
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
const fn map_key(key: egui::Key, modifiers: egui::Modifiers, escape: bool) -> Option<VimKey> {
    use egui::Key;
    Some(match key {
        _ if escape => VimKey::Esc,
        Key::R if modifiers.ctrl => VimKey::Redo,
        Key::Enter => VimKey::Enter,
        Key::Backspace => VimKey::Backspace,
        Key::ArrowLeft => VimKey::Left,
        Key::ArrowRight => VimKey::Right,
        Key::ArrowUp => VimKey::Up,
        Key::ArrowDown => VimKey::Down,
        _ => return None,
    })
}
