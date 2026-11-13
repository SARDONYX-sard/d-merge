//! Keys as the user file writes them: `ctrl+shift+l`, `space e`, `shift+g`.
//!
//! | part      | written                                                                   |
//! | --------- | ------------------------------------------------------------------------- |
//! | modifiers | `ctrl` (Cmd on macOS, egui's `COMMAND`; `cmd` is the same), `shift`, `alt` |
//! | key       | `a`-`z`, `0`-`9`, `f1`-`f35`, `enter`, `escape`, `space`, `tab`, `up`, `.` ... |
//! | sequence  | chords split by spaces: `space e`, `ctrl+k ctrl+s`                        |
//!
//! Case and the order of the modifiers do not matter: `Shift+Ctrl+L` is `ctrl+shift+l`.

use std::fmt;

use egui::{Key, Modifiers};

/// One key with its modifiers. e.g. `ctrl+shift+l`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Chord {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub key: Key,
}

impl Chord {
    /// As egui reports a key press. `ctrl` is egui's `command`: Ctrl, or Cmd on macOS.
    #[inline]
    pub const fn from_event(key: Key, modifiers: Modifiers) -> Self {
        Self { ctrl: modifiers.command, shift: modifiers.shift, alt: modifiers.alt, key }
    }

    /// Does the key type a char into a text field? Then its `Text` event goes with it.
    #[inline]
    pub(crate) const fn types_text(self) -> bool {
        !self.ctrl && !self.alt
    }

    /// The text this key types, to give a key back to the widgets as it came. e.g. `shift+g` -> `G`
    pub(crate) fn text(self) -> Option<String> {
        if !self.types_text() {
            return None;
        }
        let symbol = match self.key {
            Key::Space => " ",
            key => key.symbol_or_name(),
        };
        let mut chars = symbol.chars();
        let c = chars.next().filter(|_| chars.next().is_none())?;
        Some(if self.shift { c.to_uppercase().collect() } else { c.to_lowercase().collect() })
    }
}

impl fmt::Display for Chord {
    /// Normalized: `ctrl+shift+alt+<key>`, lowercase.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (on, name) in [(self.ctrl, "ctrl+"), (self.shift, "shift+"), (self.alt, "alt+")] {
            if on {
                f.write_str(name)?;
            }
        }
        f.write_str(key_name(self.key))
    }
}

/// The keys of a binding, one chord or more. e.g. `space e` -> `[space, e]`
pub type Keys = Vec<Chord>;

/// Reads `text` (e.g. `ctrl+shift+l`, `space e`). `Err`: what of it is not a key.
///
/// # Errors
/// An unknown key or modifier, a chord without a key, or nothing at all.
pub fn parse_keys(text: &str) -> Result<Keys, String> {
    let keys: Result<Keys, String> = text.split_whitespace().map(parse_chord).collect();
    match keys {
        Ok(keys) if keys.is_empty() => Err("no key".to_owned()),
        keys => keys,
    }
}

/// `ctrl+shift+l` -> the chord. `+` itself is written as the key of a chord: `ctrl++`.
fn parse_chord(text: &str) -> Result<Chord, String> {
    let lower = text.to_ascii_lowercase();
    let (mods, key) = match lower.strip_suffix("++") {
        Some(mods) => (mods, "+"),
        None => lower.rsplit_once('+').map_or(("", lower.as_str()), |(mods, key)| (mods, key)),
    };
    let (mut ctrl, mut shift, mut alt) = (false, false, false);
    for modifier in mods.split('+').filter(|m| !m.is_empty()) {
        match modifier {
            "ctrl" | "cmd" | "control" => ctrl = true,
            "shift" => shift = true,
            "alt" | "option" => alt = true,
            other => return Err(format!("`{other}` is not a modifier (ctrl, shift, alt)")),
        }
    }
    let key = key_of(key).ok_or_else(|| format!("`{key}` is not a key"))?;
    Ok(Chord { ctrl, shift, alt, key })
}

/// The names that differ from egui's (`Key::from_name`), as VS Code writes them.
const ALIASES: [(&str, Key); 22] = [
    ("up", Key::ArrowUp),
    ("down", Key::ArrowDown),
    ("left", Key::ArrowLeft),
    ("right", Key::ArrowRight),
    ("esc", Key::Escape),
    ("escape", Key::Escape),
    ("enter", Key::Enter),
    ("return", Key::Enter),
    ("pageup", Key::PageUp),
    ("pagedown", Key::PageDown),
    ("del", Key::Delete),
    ("ins", Key::Insert),
    (".", Key::Period),
    (",", Key::Comma),
    ("/", Key::Slash),
    ("\\", Key::Backslash),
    (";", Key::Semicolon),
    ("[", Key::OpenBracket),
    ("]", Key::CloseBracket),
    ("-", Key::Minus),
    ("=", Key::Equals),
    ("`", Key::Backtick),
];

/// A key by its name in a binding. e.g. `l` -> `Key::L`, `pagedown` -> `Key::PageDown`
fn key_of(name: &str) -> Option<Key> {
    if let Some((_, key)) = ALIASES.iter().find(|(alias, _)| *alias == name) {
        return Some(*key);
    }
    Key::ALL.iter().copied().find(|key| key.name().eq_ignore_ascii_case(name) || key.symbol_or_name() == name)
}

/// The name of `key` in a binding: the first alias, else egui's name, lowercase.
fn key_name(key: Key) -> &'static str {
    if let Some((alias, _)) = ALIASES.iter().find(|(_, known)| *known == key) {
        return alias;
    }
    // egui's names are ASCII; letters and digits are one char. Kept static: no allocation per frame.
    KEY_NAMES.iter().find(|(known, _)| *known == key).map_or_else(|| key.name(), |(_, name)| name)
}

/// Lowercase names of the keys whose egui names are capitalized.
const KEY_NAMES: [(Key, &str); 40] = [
    (Key::A, "a"),
    (Key::B, "b"),
    (Key::C, "c"),
    (Key::D, "d"),
    (Key::E, "e"),
    (Key::F, "f"),
    (Key::G, "g"),
    (Key::H, "h"),
    (Key::I, "i"),
    (Key::J, "j"),
    (Key::K, "k"),
    (Key::L, "l"),
    (Key::M, "m"),
    (Key::N, "n"),
    (Key::O, "o"),
    (Key::P, "p"),
    (Key::Q, "q"),
    (Key::R, "r"),
    (Key::S, "s"),
    (Key::T, "t"),
    (Key::U, "u"),
    (Key::V, "v"),
    (Key::W, "w"),
    (Key::X, "x"),
    (Key::Y, "y"),
    (Key::Z, "z"),
    (Key::Space, "space"),
    (Key::Tab, "tab"),
    (Key::Backspace, "backspace"),
    (Key::Home, "home"),
    (Key::End, "end"),
    (Key::F1, "f1"),
    (Key::F2, "f2"),
    (Key::F3, "f3"),
    (Key::F4, "f4"),
    (Key::F5, "f5"),
    (Key::F6, "f6"),
    (Key::F7, "f7"),
    (Key::F8, "f8"),
    (Key::F12, "f12"),
];

/// The modifiers as the file writes them, for completion.
pub const MODIFIERS: [&str; 3] = ["ctrl", "shift", "alt"];

/// The names of the keys a binding can have, as the file writes them, for completion. e.g. `a`,
/// `pagedown`, `f12`, `.`
pub fn key_names() -> impl Iterator<Item = &'static str> {
    const MORE: [&str; 30] = [
        "0",
        "1",
        "2",
        "3",
        "4",
        "5",
        "6",
        "7",
        "8",
        "9",
        "f9",
        "f10",
        "f11",
        "insert",
        "delete",
        "plus",
        "f13",
        "f14",
        "f15",
        "f16",
        "f17",
        "f18",
        "f19",
        "f20",
        "quote",
        "colon",
        "pipe",
        "questionmark",
        "exclamationmark",
        "copy",
    ];
    KEY_NAMES.iter().map(|(_, name)| *name).chain(ALIASES.iter().map(|(name, _)| *name)).chain(MORE)
}

/// `ctrl+k ctrl+s`: the keys as the file writes them, normalized.
pub fn keys_text(keys: &[Chord]) -> String {
    let parts: Vec<String> = keys.iter().map(ToString::to_string).collect();
    parts.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_read_whatever_their_case_and_order() {
        let chord = parse_keys("Shift+Ctrl+L").unwrap();
        assert_eq!(chord, [Chord { ctrl: true, shift: true, alt: false, key: Key::L }]);
        assert_eq!(keys_text(&chord), "ctrl+shift+l");
        assert_eq!(keys_text(&parse_keys("space  e").unwrap()), "space e");
        assert_eq!(keys_text(&parse_keys("alt+left").unwrap()), "alt+left");
        assert_eq!(keys_text(&parse_keys("ctrl+pagedown").unwrap()), "ctrl+pagedown");
        assert_eq!(keys_text(&parse_keys("ctrl+.").unwrap()), "ctrl+.");
        assert_eq!(keys_text(&parse_keys("f12").unwrap()), "f12");
        assert_eq!(parse_keys("ctrl++").unwrap()[0].key, Key::Plus);
    }

    #[test]
    fn what_is_not_a_key_is_told() {
        assert!(parse_keys("").is_err());
        assert!(parse_keys("ctrl+").is_err());
        assert!(parse_keys("hyper+a").unwrap_err().contains("hyper"));
        assert!(parse_keys("ctrl+nokey").unwrap_err().contains("nokey"));
    }

    #[test]
    fn every_key_name_offered_is_read() {
        for name in key_names() {
            assert!(parse_keys(name).is_ok(), "{name}");
        }
    }

    #[test]
    fn a_typing_key_has_its_text() {
        let text = |keys: &str| parse_keys(keys).unwrap()[0].text();
        assert_eq!(text("g").as_deref(), Some("g"));
        assert_eq!(text("shift+g").as_deref(), Some("G"));
        assert_eq!(text("space").as_deref(), Some(" "));
        assert_eq!(text("ctrl+g"), None);
        assert_eq!(text("enter"), None);
    }
}
