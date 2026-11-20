//! `F2`: renames the symbol at the cursor, as VS Code's rename box.
//!
//! ```text
//!   <hkparam name="eventId">$eventID[┌──────────────┐]$</hkparam>
//!                                    │ [moveStart▌] │  <- over the name, all of it selected
//!                                    │ an error     │  <- why it can't, if the language says so
//!                                    └──────────────┘
//! ```
//!
//! The language says what can be renamed ([`Language::prepare_rename`]) and what changes, in every
//! document ([`Language::rename`]). The edits go to the host (see
//! [`crate::CodeEditorOutput::rename`]): the editor has one text only. `Enter` renames, `Esc` or a
//! click elsewhere closes.

use std::ops::Range;

use egui::{
    Key,
    text::{CCursor, CCursorRange},
};

use crate::{DocumentEdit, Language, text_pos::LinePos};

/// The box while it is open.
#[derive(Debug)]
struct Open {
    /// Where the cursor was: what is renamed.
    at: LinePos,
    /// The name's char columns in its line: where the box is.
    columns: Range<usize>,
    name: String,
    /// Why the last `Enter` could not rename.
    error: Option<String>,
    /// Just opened: the field takes the keyboard, its text selected.
    first_frame: bool,
}

/// See the module docs. One per editor.
#[derive(Debug, Default)]
pub(crate) struct RenameBox {
    open: Option<Open>,
}

impl RenameBox {
    /// Opens on the symbol at `at` of `text`, if the language can rename it. Returns whether it did.
    pub(crate) fn open(&mut self, language: &dyn Language, text: &str, at: LinePos) -> bool {
        let Some(columns) = language.prepare_rename(text, at.line, at.column) else { return false };
        let line = text.split('\n').nth(at.line).unwrap_or_default();
        let name = line.chars().skip(columns.start).take(columns.len()).collect();
        self.open = Some(Open { at, columns, name, error: None, first_frame: true });
        true
    }

    /// Draws the box over the name. Returns the edits once `Enter` renamed.
    pub(crate) fn ui<'a>(
        &mut self,
        ui: &egui::Ui,
        id: egui::Id,
        (language, text): (&'a dyn Language, &str),
        (galley, galley_pos): (&egui::Galley, egui::Pos2),
    ) -> Option<Vec<DocumentEdit<'a>>> {
        let open = self.open.as_mut()?;
        let start = LinePos { line: open.at.line, column: open.columns.start }.to_char_index(text);
        let pos = galley_pos + galley.pos_from_cursor(CCursor::new(start)).left_top().to_vec2();

        let field_id = id.with("rename_field");
        let area = egui::Area::new(id.with("rename_box")).order(egui::Order::Foreground).fixed_pos(pos);
        let field = area
            .show(ui.ctx(), |ui| {
                crate::widget_frame(ui.style()).show(ui, |ui| {
                    let field = ui.add(egui::TextEdit::singleline(&mut open.name).id(field_id).desired_width(200.0));
                    if let Some(error) = &open.error {
                        ui.colored_label(ui.visuals().error_fg_color, error);
                    }
                    field
                })
            })
            .inner
            .inner;
        if std::mem::take(&mut open.first_frame) {
            select_all(ui.ctx(), field_id, open.name.chars().count());
            field.request_focus();
            return None;
        }

        let (enter, escape) = ui.input(|input| (input.key_pressed(Key::Enter), input.key_pressed(Key::Escape)));
        if field.lost_focus() && enter {
            match language.rename(text, open.at.line, open.at.column, open.name.trim()) {
                Ok(edits) => {
                    self.close(ui, id);
                    return Some(edits);
                }
                Err(error) => {
                    open.error = Some(error.into_owned());
                    field.request_focus(); // a single-line `TextEdit` gives it up on `Enter`
                }
            }
        } else if field.lost_focus() || escape {
            self.close(ui, id);
        }
        None
    }

    /// Closes, the keyboard back in the editor.
    fn close(&mut self, ui: &egui::Ui, id: egui::Id) {
        self.open = None;
        ui.memory_mut(|memory| memory.request_focus(id));
    }
}

/// Selects all `len` chars of the field `id`: typing replaces the name, as VS Code's.
fn select_all(ctx: &egui::Context, id: egui::Id, len: usize) {
    let mut state = egui::TextEdit::load_state(ctx, id).unwrap_or_default();
    state.cursor.set_char_range(Some(CCursorRange::two(CCursor::new(0), CCursor::new(len))));
    state.store(ctx, id);
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::*;

    /// Renames what is between `[` and `]`.
    struct Brackets;

    impl Language for Brackets {
        fn prepare_rename(&self, text: &str, line: usize, _column: usize) -> Option<Range<usize>> {
            let line = text.split('\n').nth(line)?;
            Some(line.find('[')? + 1..line.find(']')?)
        }

        fn rename(
            &self,
            text: &str,
            line: usize,
            column: usize,
            new_name: &str,
        ) -> Result<Vec<DocumentEdit<'_>>, Cow<'_, str>> {
            let columns = self.prepare_rename(text, line, column).ok_or("nothing")?;
            Ok(vec![DocumentEdit { document: "doc".into(), line, columns, new_text: new_name.to_owned() }])
        }
    }

    /// It opens with the name at the cursor; nothing to rename: it stays closed.
    #[test]
    fn it_opens_on_a_name() {
        let mut rename = RenameBox::default();
        assert!(!rename.open(&Brackets, "a\nno name", LinePos { line: 1, column: 0 }));
        assert!(rename.open.is_none());

        assert!(rename.open(&Brackets, "a\nx [name] y", LinePos { line: 1, column: 5 }));
        let open = rename.open.as_ref().unwrap();
        assert_eq!((open.name.as_str(), open.columns.clone()), ("name", 3..7));
    }

    /// The edits, applied by the host to each document.
    #[test]
    fn edits_apply_to_a_document() {
        let edit = |line, columns, new_text: &str| DocumentEdit {
            document: "doc".into(),
            line,
            columns,
            new_text: new_text.to_owned(),
        };
        let edits = [edit(0, 2..3, "x"), edit(1, 0..1, "y"), edit(1, 5..9, "off the line")];
        assert_eq!(DocumentEdit::apply("a b\nc", &edits), "a x\ny");
        assert_eq!(DocumentEdit::apply("é b", &[edit(0, 2..3, "x")]), "é x", "chars, not bytes");
    }
}
