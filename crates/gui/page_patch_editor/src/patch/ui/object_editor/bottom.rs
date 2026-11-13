//! Below the editor, as VS Code's: the Problems panel (when open) and the status bar, whose `⚙`
//! opens the language options.
//!
//! ```text
//! Problems  ✖ 1  ⚠ 0
//! ▼ #0106  0_master  (1)
//!     ✖ Unknown field names: "typoField".   Ln 3, Col 3
//! NORMAL  Ln 2, Col 1                       ✖ 1 ⚠ 0   Vim ▾  ⚙
//! ```

use egui_code_editor::{EditorFeatures, Keymap, ProblemsKey};
use egui_widgets::shadcn::Checkbox;
use gui_shared::{
    i18n::{I18nKey, I18nMap},
    settings::ui::editor::{EditorMode, LspOptions},
};

use crate::patch::PatchTool;

/// What the Problems panel asked for.
#[derive(Debug, Default)]
pub(super) struct ProblemsOutput {
    /// A problem was opened: `(behavior, object, line)`.
    pub(super) go_to: Option<(usize, String, usize)>,
    /// `Esc` in the panel: the keyboard goes back to the editor.
    pub(super) left: bool,
}

impl PatchTool {
    /// The status bar. Returns whether its problem counts were clicked (to show / hide the panel).
    pub(super) fn status_bar(&mut self, ui: &mut egui::Ui, i18n: &I18nMap, keymap: Keymap) -> bool {
        if self.pane.show_lsp_options {
            lsp_options_window(ui.ctx(), i18n, &mut self.pane.lsp, &mut self.pane.show_lsp_options);
        }

        let status_id = if self.is_user_patch { "user_patch_status_bar" } else { "nemesis_status_bar" };
        egui::Panel::bottom(status_id)
            .show(ui, |ui| {
                // `Vim ▾` and `⚙` at its right end, as VS Code's status bar items.
                let mut chosen = keymap;
                let status = egui_code_editor::StatusBar::new(&self.pane.state, keymap, &self.pane.text)
                    .keymap_select(&mut chosen)
                    .options(&mut self.pane.show_lsp_options, i18n.t(I18nKey::LspOptionsTitle))
                    .show(ui);
                if status.keymap_changed {
                    self.pane.mode = EditorMode::from_vim(chosen.is_vim());
                }
                status.problems.clicked()
            })
            .inner
    }

    /// The Problems panel, if open. `keys`: this frame's keys of it (`j` / `k` / `Enter` / `Esc`).
    pub(super) fn problems_ui(&mut self, ui: &mut egui::Ui, i18n: &I18nMap, keys: &[ProblemsKey]) -> ProblemsOutput {
        let Some(problems) = self.problems.list.as_ref().filter(|_| self.problems.open) else {
            return ProblemsOutput::default();
        };

        let (behaviors, cursor) = (&self.workspace.behaviors, &mut self.problems.cursor);
        let was_focused = cursor.focused;
        let go_to = super::super::problems::problems_panel(ui, i18n, problems, behaviors, cursor, keys);
        // `Esc`: back to the editor, where it was. (`Enter` goes to the problem's line.)
        let left = was_focused && !cursor.focused && go_to.is_none();

        ProblemsOutput { go_to, left }
    }
}

/// The language features the settings turn on. The lines of the animation data have no form to
/// format them to.
pub(super) const fn editor_features(lsp: &LspOptions, anim_data: bool) -> EditorFeatures {
    EditorFeatures {
        completion: lsp.completion,
        hover: lsp.hover,
        diagnostics: lsp.diagnostics,
        format: lsp.formatter && !anim_data,
        inlay_hints: lsp.inlay_hints,
        signature_help: lsp.signature_help,
    }
}

/// The `⚙` window: the language features that apply to the object editor.
fn lsp_options_window(ctx: &egui::Context, i18n: &I18nMap, lsp: &mut LspOptions, open: &mut bool) {
    let window = egui_widgets::window_near_pointer(ctx, i18n.t(I18nKey::LspOptionsTitle));
    window.open(open).resizable(false).collapsible(false).show(ctx, |ui| {
        let options = [
            (&mut lsp.completion, I18nKey::LspCompletion, I18nKey::LspCompletionHover),
            (&mut lsp.code_action, I18nKey::LspCodeAction, I18nKey::LspCodeActionHover),
            (&mut lsp.diagnostics, I18nKey::LspDiagnostics, I18nKey::LspDiagnosticsHover),
            (&mut lsp.formatter, I18nKey::LspFormatter, I18nKey::LspFormatterHover),
            (&mut lsp.hover, I18nKey::LspHover, I18nKey::LspHoverHover),
            (&mut lsp.inlay_hints, I18nKey::LspInlayHints, I18nKey::LspInlayHintsHover),
            (&mut lsp.signature_help, I18nKey::LspSignatureHelp, I18nKey::LspSignatureHelpHover),
        ];
        for (value, label, hover) in options {
            ui.add(Checkbox::new(value, i18n.t(label))).on_hover_text(i18n.t(hover));
        }
    });
}
