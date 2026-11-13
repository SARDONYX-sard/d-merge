//! Key bindings as VS Code has them: a command, its keys and where they work (`when`), out of the box
//! ([`commands`]) and changed by the user ([`file`], `keybindings.json`).
//!
//! ```text
//! frame:  page fills a KeyContext (who has the keyboard, Vim's mode)
//!           -> Keybindings::dispatch: the keys of a binding are taken out of the frame's input
//!           -> the page carries out the commands (`tabs.close`, ...)
//!           -> the widgets see only the keys left
//! ```
//! - The last binding that matches wins: the user's come after the defaults. An entry whose
//!   command is `-tabs.close` takes the bindings of `tabs.close` before it away.
//! - A sequence (`space e`) holds its first keys until the rest come, for [`PENDING_SECONDS`] at most.
//!   Keys that turn out to start nothing are given back to the widgets, as they came.

pub mod commands;
pub mod file;
mod jsonc;
pub mod key;
pub mod when;

use std::{
    path::{Path, PathBuf},
    time::SystemTime,
};

pub use self::{
    commands::{COMMANDS, Command},
    key::{Chord, Keys, keys_text, parse_keys},
    when::{Context, Value, When},
};

/// Seconds the first keys of a sequence wait for the rest (Vim's `timeoutlen`).
pub const PENDING_SECONDS: f64 = 1.0;
/// Seconds between two looks at the user file for changes.
const RELOAD_SECONDS: f64 = 1.0;

/// Who has the keyboard, and what the page shows: the names a `when` can use.
///
/// | name              | field              | e.g.                                          |
/// | ----------------- | ------------------ | --------------------------------------------- |
/// | `page`            | `page`             | `'patchEditor'`, `'hkanno'`                   |
/// | `editorFocus`     | `editor_focus`     | the code editor has the keyboard              |
/// | `vim`             | `vim`              | the Vim keymap is on                          |
/// | `vim.mode`        | `vim_mode`         | `'normal'`, `'insert'`, `'visual'`, `'pending'` (`2d`), `'command'` (`:w`) |
/// | `explorerFocus`   | `explorer_focus`   | the object tree / list                        |
/// | `explorer.view`   | `explorer_view`    | `'tree'`, `'list'`                            |
/// | `problemsFocus`   | `problems_focus`   | the Problems panel                            |
/// | `problemsVisible` | `problems_visible` |                                               |
/// | `inputFocus`      | `input_focus`      | a text field that is not the editor           |
/// | `userPatch`       | `user_patch`       | the user patch window                         |
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct KeyContext {
    pub page: &'static str,
    pub editor_focus: bool,
    pub vim: bool,
    pub vim_mode: Option<&'static str>,
    pub explorer_focus: bool,
    pub explorer_view: Option<&'static str>,
    pub problems_focus: bool,
    pub problems_visible: bool,
    pub input_focus: bool,
    pub user_patch: bool,
}

impl Context for KeyContext {
    fn value(&self, name: &str) -> Option<Value<'_>> {
        let flag = |on: bool| Some(Value::Bool(on));
        match name {
            "page" => Some(Value::Text(self.page)),
            "editorFocus" => flag(self.editor_focus),
            "vim" => flag(self.vim),
            "vim.mode" => self.vim_mode.map(Value::Text),
            "explorerFocus" => flag(self.explorer_focus),
            "explorer.view" => self.explorer_view.map(Value::Text),
            "problemsFocus" => flag(self.problems_focus),
            "problemsVisible" => flag(self.problems_visible),
            "inputFocus" => flag(self.input_focus),
            "userPatch" => flag(self.user_patch),
            _ => None,
        }
    }
}

/// One binding of the table.
#[derive(Debug, Clone)]
struct Binding {
    keys: Keys,
    command: &'static str,
    when: Option<When>,
    /// As written, to match a `-command` entry by it.
    when_text: Option<String>,
}

impl Binding {
    fn works(&self, context: &KeyContext) -> bool {
        self.when.as_ref().is_none_or(|when| when.eval(context))
    }
}

/// What the keys pressed so far lead to.
#[derive(Debug, PartialEq, Eq)]
enum Lookup {
    Command(&'static str),
    /// The start of a longer binding: wait for the rest.
    Prefix,
    None,
}

/// The user file and when it was read.
#[derive(Debug)]
struct UserFile {
    path: PathBuf,
    modified: Option<SystemTime>,
    checked_at: f64,
}

/// The bindings in force, and the keys of a sequence being typed. Keep one in the app.
#[derive(Debug)]
pub struct Keybindings {
    bindings: Vec<Binding>,
    pending: Keys,
    pending_at: f64,
    file: Option<UserFile>,
    /// What of the user file could not be taken: for the app to tell. See [`Self::take_warnings`].
    warnings: Vec<String>,
}

impl Default for Keybindings {
    /// The defaults only.
    fn default() -> Self {
        Self::with_entries(&[])
    }
}

impl Keybindings {
    /// The defaults, then `entries` of the user file. What of them can't be taken is in
    /// [`Self::take_warnings`].
    #[must_use]
    pub fn with_entries(entries: &[file::Entry]) -> Self {
        let mut bindings = Vec::new();
        for command in COMMANDS {
            for &(keys, when) in command.defaults {
                // Both are checked by a test of `commands`.
                let (Ok(keys), Ok(parsed)) = (parse_keys(keys), when.map(When::parse).transpose()) else { continue };
                bindings.push(Binding { keys, command: command.id, when: parsed, when_text: when.map(str::to_owned) });
            }
        }
        let mut warnings = Vec::new();
        for (at, entry) in entries.iter().enumerate() {
            if let Err(why) = apply_entry(&mut bindings, entry) {
                warnings.push(format!("keybindings.json: entry {} (`{}`): {why}", at + 1, entry.key));
            }
        }
        Self { bindings, pending: Vec::new(), pending_at: 0.0, file: None, warnings }
    }

    /// The defaults and the user file at `path`. A missing file is made (nothing changed in it), and
    /// the schema is written next to it each time, as the commands of this exe are.
    #[must_use]
    pub fn load(path: &Path) -> Self {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
            let _ = std::fs::write(dir.join(file::SCHEMA_FILE), file::schema());
        }
        if !path.exists() {
            let _ = std::fs::write(path, file::template());
        }
        let mut keys = Self::read(path);
        keys.file = Some(UserFile { path: path.to_path_buf(), modified: modified(path), checked_at: 0.0 });
        keys
    }

    fn read(path: &Path) -> Self {
        let text = std::fs::read_to_string(path).unwrap_or_default();
        match file::parse(&text) {
            Ok(entries) => Self::with_entries(&entries),
            Err(err) => {
                let mut keys = Self::default();
                keys.warnings.push(format!("{}: {err} (the defaults are used)", path.display()));
                keys
            }
        }
    }

    /// Reads the user file again if it changed (looked at once a second). `now`: egui's time.
    /// Returns whether it was read.
    pub fn reload_if_changed(&mut self, now: f64) -> bool {
        let Some(file) = &mut self.file else { return false };
        if now - file.checked_at < RELOAD_SECONDS {
            return false;
        }
        file.checked_at = now;
        let modified = modified(&file.path);
        if modified == file.modified {
            return false;
        }
        let mut read = Self::read(&file.path);
        read.file = Some(UserFile { path: file.path.clone(), modified, checked_at: now });
        *self = read;
        true
    }

    /// What of the user file could not be taken since the last call, e.g.
    /// `keybindings.json: entry 2 (`ctrl+q`): `tabs.closee` is no command`.
    pub fn take_warnings(&mut self) -> Vec<String> {
        std::mem::take(&mut self.warnings)
    }

    /// The keys of the binding of `command` that works in `context`, as the file writes them: for
    /// hover texts. e.g. `ctrl+shift+l`
    #[must_use]
    pub fn keys_of(&self, command: &str, context: &KeyContext) -> Option<String> {
        let binding = self.bindings.iter().rev().find(|b| b.command == command && b.works(context))?;
        Some(keys_text(&binding.keys))
    }

    /// Takes the keys of this frame that make a command in `context` out of `ctx`'s input, and returns
    /// those commands. Call before the widgets that would see the keys.
    pub fn dispatch(&mut self, ctx: &egui::Context, context: &KeyContext) -> Vec<&'static str> {
        let mut commands = Vec::new();
        ctx.input_mut(|input| {
            if !self.pending.is_empty() && input.time - self.pending_at > PENDING_SECONDS {
                self.pending.clear();
            }
            let mut at = 0;
            while at < input.events.len() {
                let egui::Event::Key { key, pressed: true, modifiers, .. } = input.events[at] else {
                    at += 1;
                    continue;
                };
                let chord = Chord::from_event(key, modifiers);
                let mut typed = self.pending.clone();
                typed.push(chord);
                match self.lookup(&typed, context) {
                    Lookup::Command(command) => {
                        take_key(&mut input.events, at, chord);
                        self.pending.clear();
                        commands.push(command);
                    }
                    Lookup::Prefix => {
                        take_key(&mut input.events, at, chord);
                        self.pending = typed;
                        self.pending_at = input.time;
                    }
                    Lookup::None if self.pending.is_empty() => at += 1,
                    Lookup::None => {
                        // The held keys start nothing with this one: back to the widgets, and this key
                        // is looked at again alone.
                        let held = give_back(&self.pending);
                        let count = held.len();
                        input.events.splice(at..at, held);
                        at += count;
                        self.pending.clear();
                    }
                }
            }
        });
        commands
    }

    fn lookup(&self, typed: &[Chord], context: &KeyContext) -> Lookup {
        if let Some(binding) = self.bindings.iter().rev().find(|b| b.keys == typed && b.works(context)) {
            return Lookup::Command(binding.command);
        }
        let longer = |b: &&Binding| b.keys.len() > typed.len() && b.keys.starts_with(typed) && b.works(context);
        if self.bindings.iter().any(|b| longer(&b)) { Lookup::Prefix } else { Lookup::None }
    }
}

/// Puts `entry` into `bindings`: after them, or (`-command`) taking those it names away.
fn apply_entry(bindings: &mut Vec<Binding>, entry: &file::Entry) -> Result<(), String> {
    let (removes, id) = entry.command.strip_prefix('-').map_or((false, entry.command.as_str()), |id| (true, id));
    let command = commands::command(id).ok_or_else(|| format!("`{id}` is no command"))?;
    if removes {
        // An unreadable key takes away by command (and `when`) only, as VS Code does with no key.
        let keys = parse_keys(&entry.key).ok();
        let before = bindings.len();
        bindings.retain(|b| {
            let named = b.command == command.id
                && keys.as_ref().is_none_or(|keys| *keys == b.keys)
                && entry.when.as_ref().is_none_or(|when| Some(when) == b.when_text.as_ref());
            !named
        });
        return if bindings.len() < before { Ok(()) } else { Err("it takes away no binding".to_owned()) };
    }
    let keys = parse_keys(&entry.key)?;
    let when = entry.when.as_deref().map(When::parse).transpose().map_err(|err| format!("when: {err}"))?;
    bindings.push(Binding { keys, command: command.id, when, when_text: entry.when.clone() });
    Ok(())
}

/// Takes the key event at `at` out, and the char it typed (its `Text` event, before the next key).
fn take_key(events: &mut Vec<egui::Event>, at: usize, chord: Chord) {
    events.remove(at);
    if !chord.types_text() {
        return;
    }
    let typed = events[at..]
        .iter()
        .take_while(|event| !matches!(event, egui::Event::Key { pressed: true, .. }))
        .position(|event| matches!(event, egui::Event::Text(_)));
    if let Some(offset) = typed {
        events.remove(at + offset);
    }
}

/// The events of keys held for a sequence, as egui reported them.
fn give_back(keys: &[Chord]) -> Vec<egui::Event> {
    let mut events = Vec::with_capacity(keys.len() * 2);
    for chord in keys {
        let modifiers = egui::Modifiers {
            alt: chord.alt,
            ctrl: chord.ctrl,
            shift: chord.shift,
            mac_cmd: false,
            command: chord.ctrl,
        };
        events.push(egui::Event::Key { key: chord.key, physical_key: None, pressed: true, repeat: false, modifiers });
        events.extend(chord.text().map(egui::Event::Text));
    }
    events
}

fn modified(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|meta| meta.modified()).ok()
}

#[cfg(test)]
mod tests;
