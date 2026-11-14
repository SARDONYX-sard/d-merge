//! The row above the editor, as VS Code's: back / forward and the tabs at the left; at the right the
//! inline diff (`⇆`, `▲ ▼ 2/5`) and the split preview (`▥`).
//!
//! ```text
//! ← →  [ Root ✖ ][ GetUpModList ● ][ #0106 (#2) ↔ (working) 🔒 ✖ ]        ⇆ ▲ ▼ 2/5  ▥
//! ```

use egui_widgets::shadcn::{Button, Icon};
use gui_shared::i18n::{I18nKey, I18nMap};

use crate::patch::{
    FocusAt, TabEvent,
    editor::tabs::EditorTabs,
    editor_area::{EditorPane, revision_view::Views},
    problems::ObjectProblems,
    tree::Graph,
};

/// What the row asked for.
#[derive(Debug, Default)]
pub(super) struct ToolbarOutput {
    /// `←` (`Some(true)`) / `→` (`Some(false)`).
    pub(super) walk_history: Option<bool>,
    pub(super) tab_event: Option<TabEvent>,
}

/// What the row is drawn with.
pub(super) struct ToolbarInput<'a> {
    pub(super) pane: &'a mut EditorPane,
    pub(super) tabs: &'a EditorTabs,
    pub(super) views: &'a Views,
    /// The work's problems: the tabs are colored by them.
    pub(super) problems: Option<&'a [ObjectProblems]>,
    /// The active behavior, and the object shown (`id`).
    pub(super) active: usize,
    pub(super) id: &'a str,
    pub(super) graph: &'a Graph,
    /// The editor has the keyboard: the shown tab's line takes the accent color.
    pub(super) focused: bool,
    /// `▥` is shown: the object has a preview (not a new one, nor one of another mod).
    pub(super) can_split: bool,
}

/// Draws the row of the object shown.
pub(super) fn show(ui: &mut egui::Ui, i18n: &I18nMap, input: ToolbarInput<'_>) -> ToolbarOutput {
    let ToolbarInput { pane, tabs, views, problems, active, id, graph, focused, can_split } = input;
    let mut output = ToolbarOutput::default();
    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            split_and_diff_buttons(ui, i18n, pane, can_split);

            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                let labels = (i18n.t(I18nKey::NemesisGoBack), i18n.t(I18nKey::NemesisGoForward));
                output.walk_history = pane.history.buttons(ui, labels);

                let marks = super::super::tab_bar::TabMarks { focused, problems };
                let view_tabs = views.tabs_of(active);
                output.tab_event =
                    super::super::tab_bar::tab_bar(ui, i18n, tabs, (active, id), graph, marks, &view_tabs);
            });
        });
    });
    output
}

/// Right to left: `▥` (if `can_split`), `⇆`, and `▲ ▼ 2/5` while the inline diff is shown.
fn split_and_diff_buttons(ui: &mut egui::Ui, i18n: &I18nMap, pane: &mut EditorPane, can_split: bool) {
    if can_split {
        let hover = i18n.t(preview_hover(pane.show_preview));
        toggle(ui, &mut pane.show_preview, Icon::Columns2, hover);
    }

    toggle(ui, &mut pane.show_inline_diff, Icon::Diff, i18n.t(I18nKey::NemesisInlineDiffHover));
    if pane.show_inline_diff {
        let hover = i18n.t(I18nKey::NemesisJumpDiffHover);
        if let Some(line) = pane.inline_diff.nav(ui, pane.cursor_line, hover) {
            pane.focus = Some(FocusAt::Line(line));
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
