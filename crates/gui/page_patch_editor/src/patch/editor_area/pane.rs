//! The editor pane's own work: what it computes from the open object, seeing only the behavior and
//! the settings it is given. See [`EditorPane`].
//!
//! ```text
//! refresh_preview      the patch, its check, the underlines, the inline diff   (after a pause)
//! expire_preview       the pause after typing is over: the preview is due
//! apply_focus_request  the keyboard to the editor, the cursor to a line
//! field_values         the enum / flag values of the behavior's fields, once per behavior
//! refresh_trigger_names  the trigger names of clip data, for the completion
//! step_change          `]c` / `[c`: the next / previous change
//! reverted_block       the text with a block of the preview put back
//! ```

use std::{borrow::Cow, path::Path, rc::Rc, time::Instant};

use egui_code_editor::{CodeEditorState, Diagnostic, InlineDiff};
use gui_shared::settings::{
    pages::{patch::behavior::ParserMode, patch_editor::PatchEditorSettings},
    ui::editor::{EditorMode, LspOptions},
};

use crate::patch::{
    FocusAt, REFRESH_DELAY, anim, editor,
    workspace::{Behavior, ExportFormat, PANDORA_DOC},
};

/// What the preview is computed with: the mod code, the parser's mode, the export's format.
#[derive(Debug, Clone, Copy)]
pub(super) struct PreviewSettings<'a> {
    pub(super) code: &'a str,
    pub(super) parser_mode: ParserMode,
    pub(super) export_format: ExportFormat,
}

/// The editor of the open object, and what is shown beside it.
///
/// ```text
/// [tabs]                                             (EditorArea::tabs)
/// ┌ editor (text, state) ──────────┬ preview / merge ┐
/// │ underlines: problems            │ preview_view    │
/// │ inline diff: inline_diff        │ merge_values    │
/// └─────────────────────────────────┴─────────────────┘
/// [status bar: mode, lsp]
/// ```
#[derive(Debug, Default)]
pub(super) struct EditorPane {
    /// The text in the editor.
    pub(super) text: String,
    /// What `text` belongs to: `(behavior, id)`.
    pub(super) opened: Option<(usize, String)>,
    pub(super) state: CodeEditorState,
    /// The editor had the keyboard last frame. See [`Self::note_focus`].
    pub(super) had_focus: bool,
    /// Where the keyboard goes next frame.
    pub(super) focus: Option<FocusAt>,
    /// The line of the cursor, last frame.
    pub(super) cursor_line: usize,
    /// The patch of the open object and the parser's word on it. `None`: to compute again, see
    /// [`EditorArea::refresh_preview`].
    pub(super) preview: Option<(Option<String>, Result<(), String>)>,
    /// When the text was last typed: the preview waits for a pause.
    pub(super) typed_at: Option<Instant>,
    /// The problems of the open object, underlined in the editor.
    pub(super) problems: Vec<Diagnostic>,
    pub(super) inline_diff: InlineDiff,
    pub(super) show_inline_diff: bool,
    pub(super) show_preview: bool,
    /// The preview scrolls with the cursor.
    pub(super) follow_preview: bool,
    pub(super) preview_view: Option<editor::preview::Preview>,
    /// What the merge panel shows, and the lines accepted last. See [`editor::merge`].
    pub(super) merge_values: Option<editor::resolve::MergeValues>,
    pub(super) merge_accepted: Vec<String>,
    /// The window of the references of an object.
    pub(super) references: Option<egui_code_editor::ReferenceList>,
    /// Back and forward, as `Alt+←` / `Alt+→`.
    pub(super) history: egui_code_editor::NavHistory<String>,
    pub(super) mode: EditorMode,
    pub(super) lsp: LspOptions,
    pub(super) show_lsp_options: bool,
    /// `(behavior, values)`: enum / flag values per field, for the completion.
    pub(super) field_values: Option<(usize, Rc<editor::language::FieldValues>)>,
    /// What each line of an animation data file is. See [`anim::lines`].
    pub(super) anim_roles: Vec<anim::lines::LineRole>,
    /// The trigger names of the clip data template, read once; and those known now, for the completion.
    template_triggers: Option<std::collections::BTreeSet<String>>,
    pub(super) trigger_names: Vec<String>,
}

impl EditorPane {
    /// Notes whether the editor has the keyboard this frame. Returns whether it just got it: the
    /// side bar shows its object again then, as VS Code's explorer follows the file being edited.
    pub(super) const fn note_focus(&mut self, has_focus: bool) -> bool {
        let got = has_focus && !self.had_focus;
        self.had_focus = has_focus;
        got
    }

    /// Gives the editor the keyboard and moves its cursor, if that was asked for.
    pub(super) fn apply_focus_request(&mut self, ctx: &egui::Context, editor: egui::Id) {
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
    pub(super) fn expire_preview(&mut self, ctx: &egui::Context) {
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
    pub(super) fn refresh_preview(&mut self, behavior: &Behavior, id: &str, settings: PreviewSettings<'_>) -> bool {
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
    pub(super) fn field_values(&mut self, index: usize, behavior: &Behavior) -> Rc<editor::language::FieldValues> {
        let cached = self.field_values.take().filter(|(cached, _)| *cached == index);
        let (_, values) = cached.unwrap_or_else(|| (index, Rc::new(editor::language::field_values(behavior))));
        self.field_values = Some((index, Rc::clone(&values)));
        values
    }

    /// In a clip data file, trigger lines complete with the known names (the template's read once).
    pub(super) fn refresh_trigger_names(&mut self, behavior: &Behavior, template_dir: &Path) {
        if !behavior.adsf {
            return;
        }
        let template = self.template_triggers.get_or_insert_with(|| anim::data::template_trigger_names(template_dir));
        self.trigger_names = anim::data::trigger_names(template, behavior);
    }

    /// `]c` / `[c`, `▲ ▼`: the next / previous change, of the inline diff if shown, else of the
    /// preview.
    pub(super) fn step_change(&mut self, forward: bool) {
        if self.show_inline_diff {
            if let Some(line) = self.inline_diff.step(self.cursor_line, forward) {
                self.focus = Some(FocusAt::Line(line));
            }
        } else if self.show_preview
            && let Some(view) = &mut self.preview_view
        {
            view.step(forward);
        }
    }

    /// The text with block `block` of the preview put back as in `template`: an operation of the
    /// Pandora file, or a hunk of the Nemesis patch. `None`: no such block.
    pub(super) fn reverted_block(&self, template: &str, block: usize) -> Option<String> {
        let is_pandora = self.preview_view.as_ref().is_some_and(editor::preview::Preview::is_pandora);
        if is_pandora {
            let opened = nemesis_diff::open_kept_arrays(&self.text);
            pandora_patch::doc::without_op(template, &opened, block)
        } else {
            Some(nemesis_diff::revert_hunk(template, &self.text, block))
        }
    }

    /// The editor's settings, shared with the hkanno editor.
    pub(super) fn read_options(&mut self, editor: &LspOptions) {
        self.mode = editor.editor_mode;
        self.lsp.clone_from(editor);
    }

    /// Back to the settings: changed by the keymap box and the options window.
    pub(super) fn write_options(&mut self, editor: &mut LspOptions) {
        // Both hold the keymap: the box only changes `editor_mode`.
        self.lsp.editor_mode = self.mode;
        if *editor != self.lsp {
            editor.clone_from(&self.lsp);
        }
    }

    /// What is shown beside the text, kept in the settings.
    pub(super) const fn read_view(&mut self, settings: &PatchEditorSettings) {
        self.show_inline_diff = settings.inline_diff;
        self.follow_preview = settings.follow_preview;
        self.show_preview = settings.show_preview;
    }

    pub(super) const fn write_view(&self, settings: &mut PatchEditorSettings) {
        settings.inline_diff = self.show_inline_diff;
        settings.show_preview = self.show_preview;
        settings.follow_preview = self.follow_preview;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Only getting the keyboard asks: keeping it does not, or the side bar could not be scrolled.
    #[test]
    fn getting_into_the_editor_asks_the_side_bar_to_follow() {
        let mut pane = EditorPane::default();
        assert!(pane.note_focus(true));
        assert!(!pane.note_focus(true));
        assert!(!pane.note_focus(false));
        assert!(pane.note_focus(true));
    }

    /// Vim `]c`: in the text while the inline diff shows, else in the preview.
    #[test]
    fn the_change_keys_go_to_what_shows_the_changes() {
        let mut pane = EditorPane {
            inline_diff: egui_code_editor::InlineDiff::between("a\nb\nc\nd\n", "a\nx\nc\ny\n"),
            preview_view: Some(crate::patch::editor::preview::Preview::new(
                "<!-- MOD_CODE ~m~ OPEN -->\nb\n<!-- CLOSE -->\n<!-- MOD_CODE ~m~ OPEN -->\nd\n<!-- CLOSE -->\n",
            )),
            show_inline_diff: true,
            show_preview: true,
            ..Default::default()
        };
        let current = |pane: &EditorPane| pane.preview_view.as_ref().map(|view| view.current);

        pane.step_change(true);
        assert!(matches!(pane.focus, Some(FocusAt::Line(1))));
        pane.cursor_line = 1;
        pane.step_change(true);
        assert!(matches!(pane.focus, Some(FocusAt::Line(3))));
        pane.cursor_line = 3;
        pane.step_change(false);
        assert!(matches!(pane.focus, Some(FocusAt::Line(1))));
        assert_eq!(current(&pane), Some(0), "the preview stays");

        pane.focus = None;
        pane.show_inline_diff = false;
        pane.step_change(true);
        assert_eq!(current(&pane), Some(1));
        assert!(pane.focus.is_none(), "the cursor stays");

        pane.show_preview = false;
        pane.step_change(true);
        assert_eq!(current(&pane), Some(1), "neither shows");
    }

    /// `LspOptions` holds the keymap too: it must not put back the one it was given.
    #[test]
    fn the_keymap_box_changes_the_keymap() {
        let mut pane = EditorPane::default();
        let mut editor = LspOptions { editor_mode: EditorMode::Normal, ..LspOptions::default() };
        pane.read_options(&editor);
        pane.mode = EditorMode::Vim;
        pane.write_options(&mut editor);
        assert_eq!(editor.editor_mode, EditorMode::Vim);

        pane.read_options(&editor);
        pane.lsp.hover = !pane.lsp.hover;
        pane.write_options(&mut editor);
        assert_eq!(editor.hover, pane.lsp.hover);
        assert_eq!(editor.editor_mode, EditorMode::Vim);
    }
}
