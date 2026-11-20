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
    /// The active behavior's template (`meshes_path`), and what changed since the last commit: the
    /// tabs are marked `A` / `M` by them.
    pub(super) changes: (&'a str, &'a [crate::patch::workspace::history::ObjectChange]),
    /// The active behavior, and the object shown (`id`).
    pub(super) active: usize,
    pub(super) id: &'a str,
    pub(super) graph: &'a Graph,
    /// The editor has the keyboard: the shown tab's line takes the accent color.
    pub(super) focused: bool,
    /// `▥` is shown: the object has a preview (not a new one, nor one of another mod).
    pub(super) can_split: bool,
    /// The node graph's button is shown: see [`super::graph::can_show`].
    pub(super) can_graph: bool,
    /// The side tab, and what the tabs tell the dock: see `crate::patch::editor_area::groups`.
    pub(super) strip: &'a mut crate::patch::editor_area::groups::TabStrip,
}

/// Draws the row of the object shown.
pub(super) fn show(ui: &mut egui::Ui, i18n: &I18nMap, input: ToolbarInput<'_>) -> ToolbarOutput {
    let ToolbarInput { pane, tabs, views, problems, changes, active, id, graph, focused, can_split, can_graph, strip } =
        input;
    let mut output = ToolbarOutput::default();
    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            split_and_diff_buttons(ui, i18n, pane, can_split);
            if can_graph && toggle_button(ui, pane.show_graph, Icon::Workflow, i18n.t(I18nKey::GraphHover)) {
                pane.toggle_graph();
            }

            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                let labels = (i18n.t(I18nKey::GoBack), i18n.t(I18nKey::GoForward));
                output.walk_history = pane.history.buttons(ui, labels);

                let marks = super::super::tab_bar::TabMarks { focused, problems, changes };
                let view_tabs = views.tabs_of(active);
                output.tab_event =
                    super::super::tab_bar::tab_bar(ui, i18n, tabs, (active, id), graph, (marks, strip), &view_tabs);
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

    toggle(ui, &mut pane.show_inline_diff, Icon::Diff, i18n.t(I18nKey::InlineDiffHover));
    if pane.show_inline_diff {
        let hover = i18n.t(I18nKey::JumpDiffHover);
        if let Some(line) = pane.inline_diff.nav(ui, pane.cursor_line, hover) {
            pane.focus = Some(FocusAt::Line(line));
        }
    }
}

/// A small icon button that turns `on` on and off.
fn toggle(ui: &mut egui::Ui, on: &mut bool, icon: Icon, hover: &str) {
    if toggle_button(ui, *on, icon, hover) {
        *on = !*on;
    }
}

/// A small icon button, drawn pressed while `on`. Returns whether it was clicked. `hover` can be
/// Markdown.
fn toggle_button(ui: &mut egui::Ui, on: bool, icon: Icon, hover: &str) -> bool {
    let button = ui.add(Button::selectable(on, "").icon(icon).small());
    egui_widgets::HoverMarkdown::on_hover_markdown(button, hover).clicked()
}

/// `▥`'s hover: what a click does.
const fn preview_hover(shown: bool) -> I18nKey {
    if shown { I18nKey::PreviewHide } else { I18nKey::PreviewShow }
}
