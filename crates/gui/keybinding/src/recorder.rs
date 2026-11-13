//! VS Code's "Press desired key combination and then press ENTER": the keys of a command, recorded
//! from the keyboard, for [`crate::Keybindings::bind`].
//!
//! ```text
//! ┌ Close the tab ────────────────────────────────────┐
//! │              ctrl+k ctrl+w                        │   <- what was pressed, up to 2 chords
//! │ Press the desired key combination and then Enter. │
//! │ Other commands with these keys:                   │
//! │   problems.toggle   (editorFocus)                 │
//! └───────────────────────────────────────────────────┘
//! ```
//! While it is open it takes every key of the frame: nothing else gets them.

use crate::{Chord, Keybindings, Keys, commands::Command, keys_text};

/// The most chords a binding recorded here has, as VS Code's (`ctrl+k ctrl+s`).
const MAX_CHORDS: usize = 2;

/// The window's state. Keep one per host.
#[derive(Debug, Default)]
pub struct KeyRecorder {
    command: Option<&'static Command>,
    keys: Keys,
}

/// How the recording ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Recorded {
    /// `Enter`: these keys for this command.
    Keys(&'static str, Keys),
    /// `Esc`.
    Cancelled,
}

/// The texts of the recorder and of the JSON editor, translated by the host.
#[derive(Debug, Clone, Copy)]
pub struct RecorderTexts<'a> {
    /// e.g. `Press the desired key combination and then press Enter. Esc cancels.`
    pub hint: &'a str,
    /// e.g. `Other commands with these keys:`
    pub also_bound: &'a str,
    /// The recorder's way to the JSON. e.g. `Edit in keybindings.json`
    pub edit_json: &'a str,
    /// The menus of the JSON's editor.
    pub labels: egui_code_editor::EditorLabels<'a>,
    /// Closing the JSON with changes asks, as VS Code. e.g. `Do you want to save the changes ...?`
    pub unsaved: &'a str,
    /// Its buttons. e.g. `Save`, `Don't Save`, `Cancel`
    pub save: &'a str,
    pub dont_save: &'a str,
    pub cancel: &'a str,
}

impl KeyRecorder {
    /// Starts recording the keys of `command`. `false`: no command has that id.
    pub fn start(&mut self, command: &str) -> bool {
        self.command = crate::commands::command(command);
        self.keys.clear();
        self.command.is_some()
    }

    #[inline]
    #[must_use]
    pub const fn is_open(&self) -> bool {
        self.command.is_some()
    }

    /// Takes every key of this frame while open: call before the widgets. Returns how it ended.
    pub fn take_keys(&mut self, ctx: &egui::Context) -> Option<Recorded> {
        let command = self.command?;
        let mut ended = None;
        ctx.input_mut(|input| {
            input.events.retain(|event| match event {
                egui::Event::Key { key, pressed, modifiers, .. } => {
                    if *pressed && ended.is_none() {
                        ended = self.on_key(command, Chord::from_event(*key, *modifiers));
                    }
                    false
                }
                egui::Event::Text(_) | egui::Event::Copy | egui::Event::Cut | egui::Event::Paste(_) => false,
                _ => true,
            });
        });
        if ended.is_some() {
            self.command = None;
        }
        ended
    }

    /// One key pressed. `Enter` / `Esc` alone end it; a third chord starts over, as VS Code.
    fn on_key(&mut self, command: &'static Command, chord: Chord) -> Option<Recorded> {
        let alone = !chord.ctrl && !chord.shift && !chord.alt;
        match chord.key {
            egui::Key::Enter if alone && !self.keys.is_empty() => {
                return Some(Recorded::Keys(command.id, std::mem::take(&mut self.keys)));
            }
            egui::Key::Escape if alone => return Some(Recorded::Cancelled),
            _ => {}
        }
        if self.keys.len() >= MAX_CHORDS {
            self.keys.clear();
        }
        self.keys.push(chord);
        None
    }

    /// Draws the window, in the middle of the screen. Returns the command whose entry is to be edited
    /// in `keybindings.json` instead: the recorder is closed then.
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        bindings: &Keybindings,
        texts: RecorderTexts<'_>,
    ) -> Option<&'static str> {
        let command = self.command?;
        let mut to_json = false;
        egui::Window::new(command.description)
            .id(egui::Id::NULL.with("keybinding_recorder"))
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .collapsible(false)
            .resizable(false)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    let keys = if self.keys.is_empty() { "…".to_owned() } else { keys_text(&self.keys) };
                    ui.heading(egui::RichText::new(keys).monospace());
                });
                ui.weak(texts.hint);
                // The mouse only: a key would be recorded.
                to_json = ui.link(texts.edit_json).clicked();
                let others: Vec<_> = bindings.bound_to(&self.keys).filter(|(other, _)| *other != command.id).collect();
                if others.is_empty() || self.keys.is_empty() {
                    return;
                }
                ui.separator();
                ui.label(texts.also_bound);
                for (other, when) in others {
                    ui.monospace(format!("  {other}  {}", when.map(|when| format!("({when})")).unwrap_or_default()));
                }
            });
        ctx.request_repaint_after(std::time::Duration::from_millis(250));
        if !to_json {
            return None;
        }
        self.command = None;
        Some(command.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_keys;

    fn chord(keys: &str) -> Chord {
        parse_keys(keys).unwrap()[0]
    }

    #[test]
    fn keys_are_recorded_until_enter() {
        let mut recorder = KeyRecorder::default();
        assert!(recorder.start(crate::commands::TABS_CLOSE));
        let command = recorder.command.unwrap();
        assert_eq!(recorder.on_key(command, chord("enter")), None, "nothing to save yet: Enter is a key");
        recorder.keys.clear();
        recorder.on_key(command, chord("ctrl+k"));
        recorder.on_key(command, chord("ctrl+w"));
        let done = recorder.on_key(command, chord("enter"));
        assert_eq!(done, Some(Recorded::Keys("tabs.close", parse_keys("ctrl+k ctrl+w").unwrap())));

        recorder.on_key(command, chord("a"));
        recorder.on_key(command, chord("b"));
        recorder.on_key(command, chord("c"));
        assert_eq!(recorder.keys, parse_keys("c").unwrap(), "a third chord starts over");
        assert_eq!(recorder.on_key(command, chord("escape")), Some(Recorded::Cancelled));
        assert!(!recorder.start("no.such"));
    }
}
