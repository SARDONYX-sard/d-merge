//! Takes keyboard input away from the `TextEdit` and turns it into [`VimKey`]s.
//!
//! In Normal/Visual mode **every** key event is removed from egui's input queue before the
//! `TextEdit` runs, so typing `dd` never inserts text. Shortcuts with Ctrl/Cmd (e.g., `Ctrl+S`)
//! are left alone for the app, except Vim's own `Ctrl+R` (redo).
//!
//! In Insert mode only `Esc` / `Ctrl+[` are taken; everything else is normal typing.

/// One key press as Vim sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum VimKey {
    /// A printable char, already shifted (e.g., `$`, `G`).
    Char(char),
    Esc,
    Enter,
    Backspace,
    Left,
    Right,
    Up,
    Down,
    /// `Ctrl+R` (redo).
    Redo,
}

/// Removes the keys Vim handles from `ui`'s input and returns them in order.
pub(super) fn take(ui: &egui::Ui, insert_mode: bool) -> Vec<VimKey> {
    ui.input_mut(|input| {
        let mut keys = Vec::new();
        input.events.retain(|event| match event {
            egui::Event::Key { key, pressed, modifiers, .. } => {
                let escape =
                    *key == egui::Key::Escape || (modifiers.ctrl && *key == egui::Key::OpenBracket);
                if insert_mode {
                    if escape && *pressed {
                        keys.push(VimKey::Esc);
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
            // No pasting/cutting into Normal mode (Vim uses `p` / `d`).
            egui::Event::Paste(_) | egui::Event::Cut if !insert_mode => false,
            _ => true,
        });
        keys
    })
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
