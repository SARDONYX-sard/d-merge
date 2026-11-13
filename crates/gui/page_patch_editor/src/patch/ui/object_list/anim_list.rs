//! The animation data's list: folders (projects), their groups, the files.
//!
//! ```text
//! ▼ DefaultMale~1 (12) ●          a folder: its menu expands / collapses below, duplicates
//!   ▼ Clips (3) ●                 a group of one kind
//!       ● Idle~mymod$0.txt        a file: a click selects, a double click keeps it open
//! ```
//! It sees only the rows, the graph, the selection and the projects; what a click or a key asks for
//! is returned ([`AnimEvent`]).

use egui_tree::{Nav, TreeKey};
use gui_shared::i18n::{I18nKey, I18nMap};

use super::RowAction;
use crate::patch::{
    anim::data,
    tool::list::{ListScroll, fixed_row, offset_to_show, row_height},
    tree::{Graph, Node},
    ui::{
        anim_tree::{AnimRow, AnimTreeState, FileKind},
        object_list::row_menu,
    },
};

/// What a row or a key asked for.
#[derive(Debug)]
pub(super) enum AnimEvent {
    /// A folder / group opens or closes (its key).
    Toggle(String),
    /// A folder's menu: everything below it opens / closes.
    ExpandBelow(String),
    CollapseBelow(String),
    Select(Node),
    /// A file's menu, or a double click (`KeepOpen`).
    Menu(Node, RowAction),
    /// Duplicate `files` (of project `from`) to another project.
    Duplicate {
        pick: DuplicatePick,
        files: Files,
        from: String,
    },
}

/// Which files a duplicate takes.
#[derive(Debug)]
pub(super) enum Files {
    /// The changed ones of this folder.
    ChangedOf(String),
    /// This file, with its motion data.
    WithMotion(String),
}

/// Where to duplicate to.
#[derive(Debug)]
pub(super) enum DuplicatePick {
    To(String),
    /// Ask which project.
    Choose,
}

/// What the keys did.
pub(super) struct Keys {
    /// The cursor moved: the file it is on is selected.
    pub(super) moved: bool,
    /// `Enter` / `Esc`: back to the editor.
    pub(super) leave: bool,
}

/// Runs this frame's keys on `rows`, as in the object tree ([`egui_tree::navigate`]). `rebuild`: the
/// rows again, after a folder / group opened or closed.
pub(super) fn keys<'a>(
    state: &mut AnimTreeState,
    keys: &[TreeKey],
    rows: &mut Vec<AnimRow<'a>>,
    rebuild: impl Fn(&AnimTreeState) -> Vec<AnimRow<'a>>,
) -> Keys {
    let mut done = Keys { moved: false, leave: false };
    for &key in keys {
        let nav: Vec<_> = rows.iter().map(AnimRow::nav).collect();
        match egui_tree::navigate(key, &nav, state.cursor) {
            Nav::Cursor(at) => {
                state.cursor = at;
                done.moved = true;
            }
            Nav::SetOpen(at, _) => {
                crate::patch::ui::anim_tree::toggle(&mut state.open, rows[at].open_key());
                *rows = rebuild(state);
            }
            Nav::Leave => done.leave = true,
            Nav::None => {}
        }
    }
    done
}

/// What the list is drawn with.
pub(super) struct ListInput<'a> {
    pub(super) graph: &'a Graph,
    pub(super) selected: Option<&'a str>,
    /// The keyboard's row, while the list has the keyboard.
    pub(super) cursor: Option<usize>,
    /// Scroll to this row.
    pub(super) scroll_to: Option<usize>,
    pub(super) scroll: ListScroll,
    /// The projects files can be duplicated to.
    pub(super) projects: &'a [String],
}

/// Draws the rows seen (up and down only, a row cut with `…`). Returns what was asked for, and
/// where the list is scrolled now.
pub(super) fn show(
    ui: &mut egui::Ui,
    i18n: &I18nMap,
    rows: &[AnimRow<'_>],
    input: &ListInput<'_>,
) -> (Option<AnimEvent>, ListScroll) {
    let height = row_height(ui);
    let mut scroll = egui::ScrollArea::vertical().auto_shrink(false);
    let sizes = (height, ui.spacing().item_spacing.y);
    if let Some(offset) = input.scroll_to.and_then(|at| offset_to_show(at, sizes, rows.len(), input.scroll, 0)) {
        scroll = scroll.vertical_scroll_offset(offset);
    }

    let mut event = None;
    let output = scroll.show_rows(ui, height, rows.len(), |ui, range| {
        for index in range {
            // The keyboard's row has a frame, as in the object tree.
            if input.cursor == Some(index) {
                let rect = egui::Rect::from_min_size(ui.cursor().min, egui::vec2(ui.available_width(), height));
                egui_tree::list::cursor_frame(ui, rect);
            }
            if let Some(asked) = fixed_row(ui, height, |ui| row_ui(ui, i18n, &rows[index], input)) {
                event = Some(asked);
            }
        }
    });

    let mut now = input.scroll;
    now.note(&output);
    (event, now)
}

/// One row, as its kind is drawn.
fn row_ui(ui: &mut egui::Ui, i18n: &I18nMap, row: &AnimRow<'_>, input: &ListInput<'_>) -> Option<AnimEvent> {
    match row {
        AnimRow::Folder { name, files, changed, open } => folder_row(ui, i18n, name, (*files, *changed, *open), input),
        AnimRow::Group { kind, key, files, changed, open } => {
            let response = group_row(ui, i18n.t(kind_label(*kind)), (*files, *changed, *open), 1);
            response.clicked().then(|| AnimEvent::Toggle(key.clone()))
        }
        AnimRow::File { node, name } => file_row(ui, i18n, *node, name, input),
    }
}

/// `▼ DefaultMale~1 (12) ●`: a click opens / closes; its menu expands / collapses below, duplicates.
fn folder_row(
    ui: &mut egui::Ui,
    i18n: &I18nMap,
    name: &str,
    counts: (usize, bool, bool),
    input: &ListInput<'_>,
) -> Option<AnimEvent> {
    let response = group_row(ui, name, counts, 0);
    let mut event = response.clicked().then(|| AnimEvent::Toggle(name.to_owned()));

    response.context_menu(|ui| {
        if ui.button(i18n.t(I18nKey::NemesisExpandAllBelow)).clicked() {
            event = Some(AnimEvent::ExpandBelow(name.to_owned()));
            ui.close();
        }
        if ui.button(i18n.t(I18nKey::NemesisCollapseAllBelow)).clicked() {
            event = Some(AnimEvent::CollapseBelow(name.to_owned()));
            ui.close();
        }
        ui.separator();
        if let Some(pick) = duplicate_items(ui, i18n, input.projects, name) {
            let files = Files::ChangedOf(name.to_owned());
            event = Some(AnimEvent::Duplicate { pick, files, from: name.to_owned() });
        }
    });
    event
}

/// `    ● Idle~mymod$0.txt`
fn file_row(ui: &mut egui::Ui, i18n: &I18nMap, node: Node, name: &str, input: &ListInput<'_>) -> Option<AnimEvent> {
    let graph = input.graph;
    let id = graph.id(node);
    let is_selected = input.selected == Some(id);
    let marker = if graph.is_changed(node) { "● " } else { "" };

    let response = egui_widgets::fitted_button(ui, is_selected, &format!("    {marker}{name}"));
    // A double click keeps it open; a click selects it.
    let mut event = if response.double_clicked() {
        Some(AnimEvent::Menu(node, RowAction::KeepOpen))
    } else {
        response.clicked().then_some(AnimEvent::Select(node))
    };

    response.context_menu(|ui| {
        if let Some(picked) = row_menu(ui, i18n, graph.is_new(node), graph.is_changed(node)) {
            event = Some(AnimEvent::Menu(node, picked));
        }
        let folder = id.split_once('/').map_or(id, |(folder, _)| folder);
        if let Some(pick) = duplicate_items(ui, i18n, input.projects, folder) {
            let files = Files::WithMotion(id.to_owned());
            event = Some(AnimEvent::Duplicate { pick, files, from: folder.to_owned() });
        }
    });
    event
}

/// `Duplicate to DefaultFemale~1`, the project that usually has the same patches, if there is one, and
/// `Duplicate to...`, which asks. `current`: the project it is in.
fn duplicate_items(ui: &mut egui::Ui, i18n: &I18nMap, projects: &[String], current: &str) -> Option<DuplicatePick> {
    let hover = i18n.t(I18nKey::NemesisDuplicateToHover);
    let mut picked = None;

    let partner = data::partner_of(current);
    if partner != current && projects.contains(&partner) {
        let label = format!("{} {partner}", i18n.t(I18nKey::NemesisDuplicateTo));
        if ui.button(label).on_hover_text(hover).clicked() {
            picked = Some(DuplicatePick::To(partner));
        }
    }
    if ui.button(i18n.t(I18nKey::NemesisDuplicateToOther)).on_hover_text(hover).clicked() {
        picked = Some(DuplicatePick::Choose);
    }

    if picked.is_some() {
        ui.close();
    }
    picked
}

/// A folder or a group: `▼ Clips (3) ●`. `(files, changed, open)`.
fn group_row(
    ui: &mut egui::Ui,
    label: &str,
    (files, changed, open): (usize, bool, bool),
    depth: usize,
) -> egui::Response {
    let glyph = if open { "▼" } else { "▶" };
    let marker = if changed { " ●" } else { "" };
    let indent = "  ".repeat(depth);
    egui_widgets::fitted_button(ui, false, &format!("{indent}{glyph} {label} ({files}){marker}"))
}

const fn kind_label(kind: FileKind) -> I18nKey {
    match kind {
        FileKind::Header => I18nKey::NemesisAnimKindHeader,
        FileKind::Clip => I18nKey::NemesisAnimKindClip,
        FileKind::Motion => I18nKey::NemesisAnimKindMotion,
        FileKind::Set => I18nKey::NemesisAnimKindSet,
        FileKind::NewSet => I18nKey::NemesisAnimKindNewSet,
    }
}
