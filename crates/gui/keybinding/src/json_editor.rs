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

use egui_code_editor::{CodeEditor, CodeEditorState, Diagnostic, EditorCommand, Keymap};

use crate::{
    Keybindings, RecorderTexts,
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
    /// Its `x` was clicked with changes: "Save / Don't Save / Cancel" shows, as VS Code's.
    asking: bool,
    /// Just opened: its editor takes the keyboard when shown next.
    focus: bool,
    /// What had the keyboard when it was opened: it gets it back when it is closed, as in VS Code.
    return_to: Option<egui::Id>,
    /// The question on closing was cancelled: the editor takes the keyboard back.
    refocus: bool,
}

/// What the question on closing with changes was answered. See [`JsonEditor::asking`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Answer {
    Save,
    DontSave,
    Cancel,
}

impl JsonEditor {
    /// Opens `path`; with `command`, at its entry, as VS Code's "Configure Keybinding" of a command.
    /// A command with none gets one at the end, its key to be typed (the file is then edited).
    pub fn open(&mut self, path: &std::path::Path, command: Option<&str>) {
        let read = std::fs::read_to_string(path).unwrap_or_default();
        self.text = if read.trim().is_empty() { file::template() } else { read };
        (self.dirty, self.asking) = (false, false);
        self.cursor = command.map(|command| self.entry_of(command));
        self.problems = diagnose(&self.text);
        (self.open, self.focus) = (true, true);
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

    /// Its `x` (`closed`) and the answer to "Save / Don't Save / Cancel": with changes it asks first,
    /// without them it closes. Returns whether it is to be saved now.
    const fn on_close(&mut self, closed: bool, answer: Option<Answer>) -> bool {
        self.asking |= closed && self.dirty;
        self.open &= !closed || self.dirty;
        match answer {
            Some(Answer::Save) => {
                (self.asking, self.open) = (false, false);
                true
            }
            Some(Answer::DontSave) => {
                (self.dirty, self.asking, self.open) = (false, false, false);
                false
            }
            Some(Answer::Cancel) => {
                (self.asking, self.refocus) = (false, true);
                false
            }
            None => false,
        }
    }

    /// Draws the window. Returns how a save went, if it was saved this frame.
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        bindings: &mut Keybindings,
        keymap: Keymap,
        texts: &RecorderTexts<'_>,
    ) -> Option<Result<(), String>> {
        let labels = texts.labels;
        if !self.open {
            return None;
        }
        let id = egui_code_editor::editor_id(EDITOR_SALT);
        let focus = std::mem::take(&mut self.focus);
        if focus {
            self.return_to = ctx.memory(|m| m.focused()).filter(|focused| *focused != id);
            egui_code_editor::set_cursor(ctx, id, self.cursor.take().unwrap_or(0));
            ctx.memory_mut(|m| m.request_focus(id));
        }
        // `Ctrl+S` as in any editor; Vim's `:w` comes as its command.
        let mut save =
            ctx.memory(|m| m.has_focus(id)) && ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::S));
        let title = if self.dirty { "keybindings.json ●" } else { "keybindings.json" };
        // As a tab of the editor: Vim's `<Space>c` / `:q`, `Ctrl+W` with the Normal keymap.
        let mut close = keymap == Keymap::Normal
            && ctx.memory(|m| m.has_focus(id))
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::W));
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
                            .host_commands(&[EditorCommand::CloseTab])
                            .language(&language)
                            .diagnostics(&self.problems)
                            .keymap(keymap)
                            .labels(labels)
                            .show(ui, &mut self.state)
                    })
                    .inner;
                // Asked again of the editor drawn: a focus asked for by id before it was is lost.
                if focus || std::mem::take(&mut self.refocus) {
                    output.response.request_focus();
                }
                let (changed, command) = (output.changed, output.command);
                if changed {
                    self.dirty = true;
                    self.problems = diagnose(&self.text);
                }
                save |= command == Some(EditorCommand::Save);
                close |= command == Some(EditorCommand::CloseTab);
            });
        let closed = !open || close;
        self.asking |= closed && self.dirty;
        let answer = if self.asking { ask(ctx, texts) } else { None };
        save |= self.on_close(closed, answer);
        if !self.open
            && let Some(back) = self.return_to.take()
        {
            ctx.memory_mut(|m| m.request_focus(back));
        }
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

/// "Do you want to save the changes?" `Save` / `Don't Save` / `Cancel` (also `Esc`), as VS Code
/// asks. `None` while it waits.
fn ask(ctx: &egui::Context, texts: &RecorderTexts<'_>) -> Option<Answer> {
    let mut answer = None;
    let modal = egui::Modal::new(egui::Id::NULL.with("keybindings_json_unsaved")).show(ctx, |ui| {
        ui.set_max_width(420.0);
        ui.label(texts.unsaved);
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            // `Save` first, as VS Code: `Enter` takes it.
            let save = ui.button(texts.save);
            if ui.memory(|m| m.focused().is_none()) {
                save.request_focus();
            }
            if save.clicked() {
                answer = Some(Answer::Save);
            }
            if ui.button(texts.dont_save).clicked() {
                answer = Some(Answer::DontSave);
            }
            if ui.button(texts.cancel).clicked() {
                answer = Some(Answer::Cancel);
            }
        });
    });
    if modal.should_close() && answer.is_none() {
        answer = Some(Answer::Cancel); // `Esc`, or a click beside it
    }
    answer
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

    /// As VS Code: closed with changes, it asks; `Cancel` keeps it, `Save` saves, `Don't Save` drops.
    #[test]
    fn closing_with_changes_asks_first() {
        let mut editor = JsonEditor { open: true, ..Default::default() };
        assert!(!editor.on_close(true, None));
        assert!(!editor.open, "no changes: closed");

        let mut editor = JsonEditor { open: true, dirty: true, ..Default::default() };
        assert!(!editor.on_close(true, None));
        assert!(editor.open && editor.asking, "kept while it asks");
        assert!(!editor.on_close(false, Some(Answer::Cancel)));
        assert!(editor.open && !editor.asking && editor.dirty);
        assert!(editor.refocus, "the editor takes the keyboard back");
        editor.on_close(true, None);
        assert!(editor.on_close(false, Some(Answer::Save)), "saved");
        assert!(!editor.open);

        let mut editor = JsonEditor { open: true, dirty: true, asking: true, ..Default::default() };
        assert!(!editor.on_close(false, Some(Answer::DontSave)));
        assert!(!editor.open && !editor.dirty);
    }

    /// Opened, its editor has the keyboard; closed (`Ctrl+W`), what had it before has it again.
    #[test]
    fn the_keyboard_goes_in_and_back() {
        let dir = std::env::temp_dir().join(format!("d_merge_json_focus_{}", std::process::id()));
        let mut bindings = Keybindings::load(&dir.join("keybindings.json"));
        let ctx = egui::Context::default();
        let page = egui::Id::NULL.with("page_editor");
        let texts = RecorderTexts {
            hint: "",
            also_bound: "",
            edit_json: "",
            labels: egui_code_editor::EditorLabels::ENGLISH,
            unsaved: "",
            save: "",
            dont_save: "",
            cancel: "",
        };
        let mut editor = JsonEditor::default();
        let mut page_text = String::new();
        let mut frame = |editor: &mut JsonEditor, bindings: &mut Keybindings, events: Vec<egui::Event>| {
            let raw = egui::RawInput { events, ..Default::default() };
            let mut output = ctx.run_ui(raw, |ui| {
                ui.add(egui::TextEdit::multiline(&mut page_text).id(page));
                editor.show(ui.ctx(), bindings, Keymap::Normal, &texts);
            });
            output.textures_delta.clear();
        };
        ctx.memory_mut(|m| m.request_focus(page));
        frame(&mut editor, &mut bindings, Vec::new());
        assert_eq!(ctx.memory(|m| m.focused()), Some(page));

        editor.open(&dir.join("keybindings.json"), None);
        frame(&mut editor, &mut bindings, Vec::new());
        frame(&mut editor, &mut bindings, Vec::new());
        assert_eq!(ctx.memory(|m| m.focused()), Some(egui_code_editor::editor_id(EDITOR_SALT)), "in the JSON");

        let modifiers = egui::Modifiers { ctrl: true, command: true, ..Default::default() };
        let key = egui::Event::Key { key: egui::Key::W, physical_key: None, pressed: true, repeat: false, modifiers };
        frame(&mut editor, &mut bindings, vec![key]);
        frame(&mut editor, &mut bindings, Vec::new());
        assert!(!editor.is_open());
        assert_eq!(ctx.memory(|m| m.focused()), Some(page), "back where it was");
        std::fs::remove_dir_all(&dir).unwrap();
    }

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
