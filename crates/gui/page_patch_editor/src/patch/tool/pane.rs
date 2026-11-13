//! The editor pane's own work: what it computes from the open object, seeing only the behavior and
//! the settings it is given. [`EditorPane`] is in [`super::editing`].
//!
//! ```text
//! refresh_preview      the patch, its check, the underlines, the inline diff   (after a pause)
//! expire_preview       the pause after typing is over: the preview is due
//! apply_focus_request  the keyboard to the editor, the cursor to a line
//! field_values         the enum / flag values of the behavior's fields, once per behavior
//! refresh_trigger_names  the trigger names of clip data, for the completion
//! ```

use std::{borrow::Cow, path::Path, rc::Rc};

use egui_code_editor::InlineDiff;
use gui_shared::settings::pages::patch::behavior::ParserMode;

use super::editing::EditorPane;
use crate::patch::{
    FocusAt, REFRESH_DELAY, anim, editor,
    workspace::{Behavior, ExportFormat, PANDORA_DOC},
};

/// What the preview is computed with: the mod code, the parser's mode, the export's format.
#[derive(Debug, Clone, Copy)]
pub(in crate::patch) struct PreviewSettings<'a> {
    pub(in crate::patch) code: &'a str,
    pub(in crate::patch) parser_mode: ParserMode,
    pub(in crate::patch) export_format: ExportFormat,
}

impl EditorPane {
    /// Gives the editor the keyboard and moves its cursor, if that was asked for.
    pub(in crate::patch) fn apply_focus_request(&mut self, ctx: &egui::Context, editor: egui::Id) {
        let Some(at) = self.focus.take() else { return };
        let index = match at {
            FocusAt::Line(line) => egui_code_editor::text_pos::LinePos { line, column: 0 }.to_char_index(&self.text),
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

    /// Recomputes the patch of object `id` of `behavior`, its check and the underlines after the text
    /// changed. Returns whether it did (the work's problems are then to look at again).
    pub(in crate::patch) fn refresh_preview(
        &mut self,
        behavior: &Behavior,
        id: &str,
        settings: PreviewSettings<'_>,
    ) -> bool {
        if self.preview.is_some() {
            return false;
        }

        let patch = behavior.patch(settings.code, id);
        let check = patch.as_deref().map_or(Ok(()), |patch| behavior.validate(id, patch, settings.parser_mode));
        self.problems = self.field_problems(behavior, id, settings.code);

        // Keep the ▲ / ▼ position across edits.
        let current = self.preview_view.as_ref().map_or(0, |view| view.current);
        let nemesis_view = patch.as_deref().map(editor::preview::Preview::new);
        // The parser reads the patch, whose lines are the Nemesis preview's: back to the editor's.
        if let Err(error) = &check {
            let view = nemesis_view.as_ref();
            let to_editor_line = |line| view.and_then(|view| view.editor_line_of(line, &self.text));
            editor::diagnostics::add_parser_error(&mut self.problems, &self.text, error, to_editor_line);
        }

        // What the export would write. Animation data stays lines: Pandora's export doesn't write it.
        let as_pandora = settings.export_format == ExportFormat::PandoraV5 && !behavior.is_anim_data();
        let (view, check) = if as_pandora {
            match editor::pandora::op_preview(behavior, id) {
                Ok(view) => (Some(view), Ok(())),
                Err(problem) => (None, Err(problem)),
            }
        } else {
            (nemesis_view, check)
        };
        self.preview_view = view;
        if let Some(view) = &mut self.preview_view {
            view.keep_current(current);
        }

        self.preview = Some((patch.map(Cow::into_owned), check));
        self.inline_diff = behavior
            .template_object(id)
            .map_or_else(InlineDiff::default, |template| InlineDiff::between(&template.text, &self.text));
        true
    }

    /// The underlines of the text itself (the parser's come after).
    fn field_problems(&mut self, behavior: &Behavior, id: &str, code: &str) -> Vec<egui_code_editor::Diagnostic> {
        if id == PANDORA_DOC {
            editor::pandora::diagnostics(&self.text, behavior)
        } else if behavior.is_anim_data() {
            self.anim_roles = anim::lines::roles(behavior.adsf, id, &self.text);
            Vec::new() // only the parser's: see `anim_lines`
        } else if behavior.is_ref(id) {
            Vec::new() // not this mod's to fix
        } else {
            let is_new = behavior.added.contains_key(id);
            editor::diagnostics::field_diagnostics(&self.text, behavior, is_new, code)
        }
    }

    /// Enum / flag values only depend on the template: computed once per behavior (`index`).
    pub(in crate::patch) fn field_values(
        &mut self,
        index: usize,
        behavior: &Behavior,
    ) -> Rc<editor::language::FieldValues> {
        let cached = self.field_values.take().filter(|(cached, _)| *cached == index);
        let (_, values) = cached.unwrap_or_else(|| (index, Rc::new(editor::language::field_values(behavior))));
        self.field_values = Some((index, Rc::clone(&values)));
        values
    }

    /// In a clip data file, trigger lines complete with the known names (the template's read once).
    pub(in crate::patch) fn refresh_trigger_names(&mut self, behavior: &Behavior, template_dir: &Path) {
        if !behavior.adsf {
            return;
        }
        let template = self.template_triggers.get_or_insert_with(|| anim::data::template_trigger_names(template_dir));
        self.trigger_names = anim::data::trigger_names(template, behavior);
    }
}
