//! VS Code's "Preferences: Open Keyboard Shortcuts (JSON)": `keybindings.json` in the code editor.
//!
//! It has what its schema gives ([`crate::json`]). Saved (`Ctrl+S`, Vim `:w`), the bindings are
//! read again at once.
//!
//! ```text
//! ┌ keybindings.json ● ───────────────────────────────── x ┐
//! │ { "$schema": ..., "bindings": [                         │
//! │     { "key": "f9", "command": "tabs.cl|" },   <- completion of the commands
//! │ ] }                                                     │
//! │ NORMAL  ✖ 0  ⚠ 1  Ln 3, Col 35                          │
//! └─────────────────────────────────────────────────────────┘
//! ```

use egui_code_editor::{CodeEditor, CodeEditorState, Diagnostic, EditorCommand, EditorLabels, Keymap};

use crate::{
    Keybindings,
    file::{self, entry_text},
    json::{KeybindingsJson, diagnose},
};

/// The salt of the editor's id: one such window in the app.
const EDITOR_SALT: &str = "keybindings_json";

/// The window's state. Keep one per host (see [`crate::KeyUi`]).
#[derive(Debug, Default)]
pub struct JsonEditor {
    open: bool,
    text: String,
    state: CodeEditorState,
    problems: Vec<Diagnostic>,
    /// Edited since it was read or saved.
    dirty: bool,
    /// Where the cursor goes when it is shown next: a char index.
    cursor: Option<usize>,
}

impl JsonEditor {
    /// Opens `path`; with `command`, at its entry, as VS Code's "Configure Keybinding" of a command.
    /// A command with none gets one at the end, its key to be typed (the file is then edited).
    pub fn open(&mut self, path: &std::path::Path, command: Option<&str>) {
        let read = std::fs::read_to_string(path).unwrap_or_default();
        self.text = if read.trim().is_empty() { file::template() } else { read };
        self.dirty = false;
        self.cursor = command.map(|command| self.entry_of(command));
        self.problems = diagnose(&self.text);
        self.open = true;
    }

    /// The char index of `command`'s entry: its `"key"` value. An uncommented entry first, then a
    /// default in the comments; with neither, a new entry is added.
    fn entry_of(&mut self, command: &str) -> usize {
        let quoted = format!("\"command\": \"{command}\"");
        let quoted = quoted.as_str();
        let found = line_of(&self.text, quoted, false).or_else(|| line_of(&self.text, quoted, true));
        if let Some((start, line)) = found {
            let key = line.find("\"key\": \"").map_or(0, |at| line[..at + 8].chars().count());
            return start + key;
        }
        self.text = file::add_entries(&self.text, &[entry_text("", command, None)]);
        self.dirty = true;
        self.problems = diagnose(&self.text);
        let (start, line) = line_of(&self.text, quoted, false).unwrap_or((0, ""));
        start + line.find("\"key\": \"").map_or(0, |at| line[..at + 8].chars().count())
    }

    #[inline]
    #[must_use]
    pub const fn is_open(&self) -> bool {
        self.open
    }

    /// Its editor has the keyboard: the page's bindings are not for the keys then.
    #[must_use]
    pub fn has_keyboard(&self, ctx: &egui::Context) -> bool {
        self.open && ctx.memory(|m| m.has_focus(egui_code_editor::editor_id(EDITOR_SALT)))
    }

    /// Draws the window. Returns how a save went, if it was saved this frame.
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        bindings: &mut Keybindings,
        keymap: Keymap,
        labels: EditorLabels<'_>,
    ) -> Option<Result<(), String>> {
        if !self.open {
            return None;
        }
        let id = egui_code_editor::editor_id(EDITOR_SALT);
        if let Some(cursor) = self.cursor.take() {
            egui_code_editor::set_cursor(ctx, id, cursor);
            ctx.memory_mut(|m| m.request_focus(id));
        }
        // `Ctrl+S` as in any editor; Vim's `:w` comes as its command.
        let mut save =
            ctx.memory(|m| m.has_focus(id)) && ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::S));
        let title = if self.dirty { "keybindings.json ●" } else { "keybindings.json" };
        let mut open = true;
        let size = ctx.content_rect().size() * egui::vec2(0.7, 0.75);
        egui::Window::new(title)
            .id(egui::Id::NULL.with(EDITOR_SALT))
            .open(&mut open)
            .default_size(size)
            .resizable(true)
            .collapsible(false)
            .show(ctx, |ui| {
                egui::Panel::bottom("keybindings_json_status").show(ui, |ui| {
                    egui_code_editor::status_line(ui, &self.state, keymap, &self.text);
                });
                let language = KeybindingsJson { bindings };
                let output = egui::ScrollArea::both()
                    .id_salt("keybindings_json_scroll")
                    .auto_shrink(false)
                    .show(ui, |ui| {
                        CodeEditor::new(EDITOR_SALT, &mut self.text)
                            .language(&language)
                            .diagnostics(&self.problems)
                            .keymap(keymap)
                            .labels(labels)
                            .show(ui, &mut self.state)
                    })
                    .inner;
                let (changed, command) = (output.changed, output.command);
                if changed {
                    self.dirty = true;
                    self.problems = diagnose(&self.text);
                }
                save |= command == Some(EditorCommand::Save);
            });
        self.open = open;
        if !save {
            return None;
        }
        let path = bindings.path().map(std::path::Path::to_path_buf);
        let Some(path) = path else { return Some(Err("no keybindings.json".to_owned())) };
        if let Err(err) = std::fs::write(&path, &self.text) {
            return Some(Err(format!("{}: {err}", path.display())));
        }
        self.dirty = false;
        bindings.reload_now();
        Some(Ok(()))
    }
}

/// The first line with `quoted` (commented or not), and the char index it starts at.
fn line_of<'t>(text: &'t str, quoted: &str, commented: bool) -> Option<(usize, &'t str)> {
    let mut start = 0;
    for line in text.split('\n') {
        if line.contains(quoted) && line.trim_start().starts_with("//") == commented {
            return Some((start, line));
        }
        start += line.chars().count() + 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_command_is_opened_at_its_entry() {
        let mut editor = JsonEditor { text: file::template(), ..Default::default() };
        // A default in the comments: there, not edited.
        let at = editor.entry_of(crate::commands::TABS_CLOSE);
        let line = editor.text.chars().skip(at).take_while(|c| *c != '\n').collect::<String>();
        assert!(line.starts_with("ctrl+w\""), "{line}");
        assert!(!editor.dirty);

        // None anywhere (a command of no defaults would be): a new entry, its key to type.
        editor.text = r#"{ "bindings": [ ] }"#.to_owned();
        let at = editor.entry_of(crate::commands::TABS_CLOSE);
        assert!(editor.dirty);
        assert_eq!(editor.text.chars().nth(at), Some('"'), "between the quotes of the key: {}", editor.text);
    }
}
