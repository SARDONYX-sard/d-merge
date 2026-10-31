//! Pointer navigation in the object editor.
//!
//! | Key                    | Action                                                  |
//! | ---------------------- | ------------------------------------------------------- |
//! | `F12` / `Ctrl+click`   | Go to the object of the `#id` under the cursor          |
//! | `Shift+F12`            | List the objects that point to it (click one to go)     |
//! | Right click a `#id`    | A menu with both                                        |
//! | Vim `gd` / `gD`        | Same as `F12` / `Shift+F12`                             |
//! | Vim `Ctrl+O` / `Ctrl+I`| Back / forward through the visited objects              |

use egui_code_editor::text_pos::LinePos;

use super::workspace::Behavior;

/// The "references" window: `(id, [(referrer id, its class)])`.
pub(super) type References = Option<(String, Vec<(String, String)>)>;

/// What the keys / click / menu asked for.
pub(super) enum Request {
    GoTo(String),
    References(String),
    Back,
    Forward,
}

/// Visited objects, for `Ctrl+O` / `Ctrl+I`.
#[derive(Debug, Default)]
pub(super) struct History {
    back: Vec<String>,
    forward: Vec<String>,
}

impl History {
    /// Leaving `from` for another object.
    pub(super) fn visit(&mut self, from: &str) {
        if self.back.last().map(String::as_str) != Some(from) {
            self.back.push(from.to_string());
        }
        self.forward.clear();
    }

    /// The object to go back (`older`) / forward to, from `current`.
    pub(super) fn walk(&mut self, older: bool, current: &str) -> Option<String> {
        let (from, to) = if older {
            (&mut self.back, &mut self.forward)
        } else {
            (&mut self.forward, &mut self.back)
        };
        let target = from.pop()?;
        to.push(current.to_string());
        Some(target)
    }
}

/// `F12` / `Shift+F12` for the `#id` at the editor's cursor. Call **before** showing the editor:
/// a keymap (Vim) takes the keys otherwise.
pub(super) fn key_request(ui: &egui::Ui, editor: egui::Id, text: &str) -> Option<Request> {
    use egui::{Key, Modifiers};
    if !ui.memory(|m| m.has_focus(editor)) {
        return None;
    }
    let (references, go_to) = ui.input_mut(|i| {
        (i.consume_key(Modifiers::SHIFT, Key::F12), i.consume_key(Modifiers::NONE, Key::F12))
    });
    if !(references || go_to) {
        return None;
    }
    let index = egui::TextEdit::load_state(ui.ctx(), editor)?.cursor.char_range()?.primary.index.0;
    let id = pointer_at(text, LinePos::from_char_index(text, index))?;
    Some(if references { Request::References(id) } else { Request::GoTo(id) })
}

/// Vim `gd` / `gD` / `Ctrl+O` / `Ctrl+I` (the keymap reports them as commands).
pub(super) fn command_request(
    output: &egui_code_editor::CodeEditorOutput,
    text: &str,
) -> Option<Request> {
    use egui_code_editor::EditorCommand;
    let id = || output.cursor.and_then(|pos| pointer_at(text, pos));
    match output.command? {
        EditorCommand::GoToDefinition => id().map(Request::GoTo),
        EditorCommand::FindReferences => id().map(Request::References),
        EditorCommand::JumpBack => Some(Request::Back),
        EditorCommand::JumpForward => Some(Request::Forward),
        EditorCommand::Save => None,
    }
}

/// `Ctrl+click` on a `#id`, and the right-click menu. Call after showing the editor.
///
/// `menu_target`: the `#id` the menu was opened on (the pointer moves while it is open).
pub(super) fn pointer_request(
    output: &egui_code_editor::CodeEditorOutput,
    text: &str,
    menu_target: &mut Option<String>,
    labels: (&str, &str),
) -> Option<Request> {
    let response = &output.response;
    if response.secondary_clicked() {
        *menu_target = output.pointer.and_then(|pos| pointer_at(text, pos));
    }
    let ctrl_click = response.clicked() && response.ctx.input(|i| i.modifiers.command);
    if ctrl_click && let Some(id) = output.pointer.and_then(|pos| pointer_at(text, pos)) {
        return Some(Request::GoTo(id));
    }

    let mut request = None;
    if let Some(target) = menu_target.clone() {
        response.context_menu(|ui| {
            ui.weak(&target);
            if ui.button(format!("{}  (F12)", labels.0)).clicked() {
                request = Some(Request::GoTo(target.clone()));
                ui.close();
            }
            if ui.button(format!("{}  (Shift+F12)", labels.1)).clicked() {
                request = Some(Request::References(target.clone()));
                ui.close();
            }
        });
    }
    request
}

/// The `#id` token at `pos` (on it or right after it).
fn pointer_at(text: &str, pos: LinePos) -> Option<String> {
    let line: Vec<char> = text.lines().nth(pos.line)?.chars().collect();
    let is_id = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '$' || c == '#';
    let at = pos.column.min(line.len());
    let start = line[..at].iter().rposition(|&c| !is_id(c)).map_or(0, |i| i + 1);
    let end = line[at..].iter().position(|&c| !is_id(c)).map_or(line.len(), |i| at + i);
    let word: String = line[start..end].iter().collect();
    // `#0106` (also inside `name="#0106"`); not a Nemesis `$variable$`.
    let word = word.trim_end_matches('$');
    word.starts_with('#').then(|| word.to_string())
}

/// Objects whose text points to `id` (whole token).
pub(super) fn references(behavior: &Behavior, id: &str) -> Vec<(String, String)> {
    let ids = behavior.added.keys().cloned().chain(behavior.objects.iter().map(|o| o.id.clone()));
    ids.filter(|other| other != id)
        .filter(|other| behavior.text(other).is_some_and(|text| contains_token(text, id)))
        .map(|other| {
            let class = behavior.class_of(&other).unwrap_or_default();
            (other, class)
        })
        .collect()
}

fn contains_token(text: &str, token: &str) -> bool {
    let is_id = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '$';
    text.match_indices(token).any(|(at, _)| !text[at + token.len()..].starts_with(is_id))
}

/// The references window. Returns the referrer that was clicked.
pub(super) fn references_window(
    ctx: &egui::Context,
    references: &mut References,
) -> Option<String> {
    let (id, list) = references.as_ref()?;
    let mut open = true;
    let mut picked = None;
    egui::Window::new(format!("References: {id} ({})", list.len()))
        .open(&mut open)
        .default_height(300.0)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                for (other, class) in list {
                    let text = egui::RichText::new(format!("{other}  {class}")).monospace();
                    if ui.add(egui::Button::selectable(false, text)).clicked() {
                        picked = Some(other.clone());
                    }
                }
            });
        });
    if !open {
        *references = None;
    }
    picked
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_pointer_under_cursor() {
        let text = "a\n\t\t\t\t#0105 #0109\n";
        assert_eq!(pointer_at(text, LinePos { line: 1, column: 6 }).as_deref(), Some("#0105"));
        assert_eq!(pointer_at(text, LinePos { line: 1, column: 11 }).as_deref(), Some("#0109"));
        assert!(!contains_token("#0105 #01090", "#0109"));
        assert!(contains_token("#0105 #0109\n", "#0109"));
    }
}
