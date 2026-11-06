//! The open object: its tabs, what it shows beside the text, and what the editor asked for.

use std::{borrow::Cow, path::Path, rc::Rc, time::Instant};

use egui_code_editor::{EditorCommand, InlineDiff};

use crate::patch::{EditorResult, FocusAt, PatchTool, REFRESH_DELAY, SAVE_DELAY, TabEvent, anim, editor};

impl PatchTool {
    /// The next or previous tab, with the keyboard staying in the editor.
    pub(in crate::patch) fn step_tab(&mut self, current: &str, forward: bool) {
        if let Some(next) = self.tabs.neighbor(self.active, current, forward) {
            self.selected = Some(next.into());
            self.focus_editor = Some(FocusAt::Keep);
        }
    }

    /// Vim `Ctrl+O` / `Ctrl+I`: the previous / next visited object (skips deleted ones).
    pub(in crate::patch) fn walk_history(&mut self, older: bool, current: &str) {
        let mut current = current.to_string();
        while let Some(target) = self.history.walk(older, &current) {
            if self.workspace.behaviors[self.active].text(&target).is_some() {
                self.selected = Some(target.into());
                self.focus_editor = Some(FocusAt::Keep);
                return;
            }
            current = target;
        }
    }

    /// Opens `document` at `line` (go to definition, a click in the references window).
    pub(in crate::patch) fn open_location(&mut self, current: &str, document: &str, line: usize) {
        if document != current {
            self.history.visit(current);
            self.selected = Some(document.into());
        }
        self.focus_editor = Some(FocusAt::Line(line));
    }

    /// Double click or menu in the tree and the list: shows `id` in a tab that stays.
    pub(in crate::patch) fn keep_open(&mut self, id: &str) {
        let current = self.selected.as_deref().filter(|selected| *selected != id);
        self.tabs.open(self.active, id, current);
        self.tabs.pin(self.active, id);
        self.selected = Some(id.into());
    }

    /// Loads `id` into the editor if it is not the open object already.
    pub(in crate::patch) fn open_in_editor(&mut self, id: &str) {
        if self.editor_for.as_ref().is_some_and(|(active, open)| *active == self.active && open == id) {
            return;
        }
        // The tab being left keeps its undo history and Vim state.
        let left = self.editor_for.take();
        if let Some((behavior, left)) = &left {
            self.tabs.store_state(*behavior, left, std::mem::take(&mut self.editor));
        }
        let current = left.as_ref().filter(|(behavior, _)| *behavior == self.active).map(|(_, id)| id.as_str());
        self.tabs.open(self.active, id, current);
        self.editor = self.tabs.take_state(self.active, id);
        self.refresh_trigger_names();

        let text = self.workspace.behaviors[self.active].text(id).unwrap_or_default();
        text.clone_into(&mut self.editor_text);
        self.preview = None;
        self.editor_for = Some((self.active, id.to_owned()));
    }

    /// In a clip data file, trigger lines complete with the known names.
    pub(in crate::patch) fn refresh_trigger_names(&mut self) {
        let behavior = &self.workspace.behaviors[self.active];
        if !behavior.adsf {
            return;
        }
        let template_dir = Path::new(&self.template_dir);
        let template = self.template_triggers.get_or_insert_with(|| anim::data::template_trigger_names(template_dir));
        self.trigger_names = anim::data::trigger_names(template, behavior);
    }

    /// Gives the editor the keyboard and moves its cursor, if that was asked for.
    pub(in crate::patch) fn apply_focus_request(&mut self, ctx: &egui::Context, editor: egui::Id) {
        let Some(at) = self.focus_editor.take() else { return };
        let index = match at {
            FocusAt::Line(line) => {
                egui_code_editor::text_pos::LinePos { line, column: 0 }.to_char_index(&self.editor_text)
            }
            FocusAt::Keep => egui::TextEdit::load_state(ctx, editor)
                .and_then(|state| state.cursor.char_range())
                .map_or(0, |range| range.primary.index.0),
        };
        egui_code_editor::set_cursor(ctx, editor, index);
    }

    /// The pause after typing is over: `preview` goes, for [`Self::refresh_preview`] to compute it.
    /// Until then the last results stay on screen, as in VS Code.
    pub(in crate::patch) fn expire_preview(&mut self, ctx: &egui::Context) {
        let Some(typed_at) = self.typed_at else { return };
        let left = REFRESH_DELAY.saturating_sub(typed_at.elapsed());
        if left.is_zero() {
            self.typed_at = None;
            self.preview = None;
        } else {
            ctx.request_repaint_after(left);
        }
    }

    /// Recomputes the patch, its check and the underlines after the text changed.
    pub(in crate::patch) fn refresh_preview(&mut self, id: &str) {
        if self.preview.is_some() {
            return;
        }
        let behavior = &self.workspace.behaviors[self.active];
        let patch = behavior.patch(self.mod_code.trim(), id);
        let check = patch.as_deref().map_or(Ok(()), |patch| behavior.validate(id, patch, self.parser_mode));
        self.field_problems = if behavior.is_anim_data() {
            self.anim_roles = anim::lines::roles(behavior.adsf, id, &self.editor_text);
            Vec::new() // Only the parser's, below: see `anim_lines`.
        } else if behavior.is_ref(id) {
            Vec::new() // not this mod's to fix
        } else {
            let is_new = behavior.added.contains_key(id);
            editor::diagnostics::field_diagnostics(&self.editor_text, behavior, is_new, self.mod_code.trim())
        };
        // Keep the ▲ / ▼ position across edits.
        let current = self.preview_view.as_ref().map_or(0, |view| view.current);
        self.preview_view = patch.as_deref().map(editor::preview::Preview::new);
        if let Some(view) = &mut self.preview_view {
            view.keep_current(current);
        }
        // The parser reads the patch, whose lines are the preview's: back to the editor's.
        if let Err(error) = &check {
            let view = self.preview_view.as_ref();
            let to_editor_line = |line| view.and_then(|view| view.editor_line_of(line, &self.editor_text));
            editor::diagnostics::add_parser_error(&mut self.field_problems, &self.editor_text, error, to_editor_line);
        }
        self.preview = Some((patch.map(Cow::into_owned), check));
        self.inline_diff = behavior.template_object(id).map_or_else(InlineDiff::default, |template| {
            editor::inline_diff::changes(&template.text, &self.editor_text)
        });
        self.refresh_work_check(id);
    }

    /// Enum / flag values only depend on the template: computed once per behavior.
    pub(in crate::patch) fn field_values(&mut self) -> Rc<editor::language::FieldValues> {
        let active = self.active;
        let cached = self.field_values.take().filter(|(cached, _)| *cached == active);
        let (_, values) = cached
            .unwrap_or_else(|| (active, Rc::new(editor::language::field_values(&self.workspace.behaviors[active]))));
        self.field_values = Some((active, Rc::clone(&values)));
        values
    }

    /// Does what the tab bar asked for. `current`: the object shown now.
    pub(in crate::patch) fn apply_tab_event(&mut self, event: Option<TabEvent>, current: &str) {
        let active = self.active;
        match event {
            Some(TabEvent::Show(id)) => self.selected = Some(id.into()),
            Some(TabEvent::KeepOpen(id)) => self.tabs.pin(active, &id),
            Some(TabEvent::CloseToTheRight(id)) => {
                self.tabs.close_to_the_right(active, &id);
                if !self.tabs.of(active).any(|tab| tab.id == current) {
                    self.selected = Some(id.into());
                }
            }
            Some(TabEvent::CloseAll) => {
                self.tabs.close_all(active);
                self.selected = None;
                self.editor_for = None;
            }
            Some(TabEvent::Close(id)) => {
                let neighbor = self.tabs.close(active, &id);
                if id == current {
                    self.selected = neighbor.map(Rc::from);
                    self.editor_for = None; // its editor state went with the tab
                }
            }
            Some(TabEvent::CloseOthers(id)) => {
                self.tabs.close_others(active, &id);
                self.selected = Some(id.into());
            }
            None => {}
        }
    }

    /// Does what the editor asked for. Returns whether the keyboard goes to the object tree.
    pub(in crate::patch) fn apply_editor_result(&mut self, result: EditorResult, id: &str) -> bool {
        if result.changed {
            self.workspace.behaviors[self.active].set_text(id, &self.editor_text);
            self.typed_at = Some(Instant::now());
            self.tabs.pin(self.active, id); // edited: it stays open
            self.object_changed(id);
        }
        if let Some(line) = result.jump_to_line {
            self.focus_editor = Some(FocusAt::Line(line));
        }
        self.apply_tab_event(result.tab_event, id);
        if let Some(index) = result.inline_revert {
            self.revert_inline(id, index);
        }
        match result.command {
            // Vim `:w`: save the work now.
            Some(EditorCommand::Save) => self.dirty_since = Instant::now().checked_sub(SAVE_DELAY),
            Some(EditorCommand::JumpBack) => self.walk_history(true, id),
            Some(EditorCommand::JumpForward) => self.walk_history(false, id),
            Some(EditorCommand::NextTab) => self.step_tab(id, true),
            Some(EditorCommand::PreviousTab) => self.step_tab(id, false),
            Some(EditorCommand::CloseTab) => {
                self.apply_tab_event(Some(TabEvent::Close(id.to_owned())), id);
                self.focus_editor = Some(FocusAt::Keep);
            }
            Some(EditorCommand::TogglePreview) => self.show_preview = !self.show_preview,
            Some(EditorCommand::ToggleInlineDiff) => self.show_inline_diff = !self.show_inline_diff,
            Some(EditorCommand::ToggleExplorer | EditorCommand::GoToDefinition | EditorCommand::FindReferences)
            | None => {}
        }
        if let Some(older) = result.walk_history {
            self.walk_history(older, id);
        }
        if let Some((document, line)) = result.definition {
            self.open_location(id, &document, line);
        }
        result.focus_tree || result.command == Some(EditorCommand::ToggleExplorer)
    }

    /// `↶` in the inline diff: puts a changed place back to the template.
    pub(in crate::patch) fn revert_inline(&mut self, id: &str, index: usize) {
        let behavior = &mut self.workspace.behaviors[self.active];
        let Some(template) = behavior.template_object(id) else { return };
        self.editor_text = editor::inline_diff::revert(&template.text, &self.editor_text, index);
        behavior.set_text(id, &self.editor_text);
        self.preview = None;
        self.object_changed(id);
    }

    /// `↶` in the preview: puts changed block `block` back to the template.
    pub(in crate::patch) fn revert_block(&mut self, id: &str, block: usize) {
        let behavior = &mut self.workspace.behaviors[self.active];
        let Some(template) = behavior.template_object(id) else { return };
        self.editor_text = nemesis_diff::revert_hunk(&template.text, &self.editor_text, block);
        behavior.set_text(id, &self.editor_text);
        self.preview = None;
        self.object_changed(id);
    }

    /// The text of object `id` changed, or it was added or deleted: `names` takes that one in.
    pub(in crate::patch) fn object_changed(&mut self, id: &str) {
        if !self.changed_objects.iter().any(|changed| changed == id) {
            self.changed_objects.push(id.to_owned());
        }
        self.mark_dirty();
    }

    /// The behaviors themselves changed (loaded, closed): `names` starts over.
    pub(in crate::patch) fn reset_names(&mut self) {
        self.names = None;
        self.changed_objects.clear();
    }
}
