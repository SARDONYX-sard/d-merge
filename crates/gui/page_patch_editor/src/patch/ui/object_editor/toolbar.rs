//! The row above the editor, as VS Code's: back / forward and the tabs at the left; at the right the
//! inline diff (`⇆`, `▲ ▼ 2/5`) and the split preview (`▥`).
//!
//! ```text
//! ← →  [ Root ✖ ][ GetUpModList ● ][ #0106 (#2) ↔ (working) 🔒 ✖ ]        ⇆ ▲ ▼ 2/5  ▥
//! ```

use egui_widgets::shadcn::{Button, Icon};
use gui_shared::i18n::{I18nKey, I18nMap};

use crate::patch::{FocusAt, PatchTool, TabEvent, tree::Graph, workspace::PANDORA_DOC};

/// What the row asked for.
#[derive(Debug, Default)]
pub(super) struct ToolbarOutput {
    /// `←` (`Some(true)`) / `→` (`Some(false)`).
    pub(super) walk_history: Option<bool>,
    pub(super) tab_event: Option<TabEvent>,
}

impl PatchTool {
    /// Draws the row of object `id`. `editor_focused`: the shown tab's line takes the accent color.
    pub(super) fn toolbar(
        &mut self,
        ui: &mut egui::Ui,
        i18n: &I18nMap,
        id: &str,
        (graph, editor_focused): (&Graph, bool),
    ) -> ToolbarOutput {
        let mut output = ToolbarOutput::default();
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                self.split_and_diff_buttons(ui, i18n, id);

                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    output = self.history_and_tabs(ui, i18n, id, (graph, editor_focused));
                });
            });
        });
        output
    }

    /// `← →` and the tabs: the objects', then the diffs' / blames'.
    fn history_and_tabs(
        &self,
        ui: &mut egui::Ui,
        i18n: &I18nMap,
        id: &str,
        (graph, focused): (&Graph, bool),
    ) -> ToolbarOutput {
        let labels = (i18n.t(I18nKey::NemesisGoBack), i18n.t(I18nKey::NemesisGoForward));
        let walk_history = self.pane.history.buttons(ui, labels);

        let marks = super::super::tab_bar::TabMarks { focused, problems: self.problems.list.as_deref() };
        let views = self.views.tabs_of(self.active);
        let tab_event = super::super::tab_bar::tab_bar(ui, i18n, &self.tabs, (self.active, id), graph, marks, &views);

        ToolbarOutput { walk_history, tab_event }
    }

    /// Right to left: `▥` (not for a new object: it has no preview), `⇆`, and `▲ ▼ 2/5` while the
    /// inline diff is shown.
    fn split_and_diff_buttons(&mut self, ui: &mut egui::Ui, i18n: &I18nMap, id: &str) {
        let behavior = &self.workspace.behaviors[self.active];
        let can_split = !behavior.added.contains_key(id) && !behavior.is_ref(id) && id != PANDORA_DOC;
        if can_split {
            let hover = i18n.t(preview_hover(self.pane.show_preview));
            toggle(ui, &mut self.pane.show_preview, Icon::Columns2, hover);
        }

        toggle(ui, &mut self.pane.show_inline_diff, Icon::Diff, i18n.t(I18nKey::NemesisInlineDiffHover));
        if self.pane.show_inline_diff {
            let hover = i18n.t(I18nKey::NemesisJumpDiffHover);
            if let Some(line) = self.pane.inline_diff.nav(ui, self.pane.cursor_line, hover) {
                self.pane.focus = Some(FocusAt::Line(line));
            }
        }
    }
}

/// A small icon button that turns `on` on and off.
fn toggle(ui: &mut egui::Ui, on: &mut bool, icon: Icon, hover: &str) {
    if ui.add(Button::selectable(*on, "").icon(icon).small()).on_hover_text(hover).clicked() {
        *on = !*on;
    }
}

/// `▥`'s hover: what a click does.
const fn preview_hover(shown: bool) -> I18nKey {
    if shown { I18nKey::EditorHidePreview } else { I18nKey::EditorShowPreview }
}
