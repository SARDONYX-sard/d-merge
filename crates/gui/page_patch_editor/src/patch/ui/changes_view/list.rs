//! The lists of the Changes view, below the commit box:
//!
//! ```text
//! ▼ CHANGES (2)
//!   M 0_master #0106 hkbModifierList GetUpModList   a click opens it; its menu compares / discards
//! ▼ OBJECT HISTORY  #0106                           the revisions that changed the selected object
//!   [Blame]
//!   M #3 fix the speed  2026-11-12 18:00
//! ▼ HISTORY
//!   ▼ #3 fix the speed  2026-11-12 18:00  (2)       its menu restores the whole work
//!       M 0_master #0106 ...                        a click shows what #3 changed of it
//! ```
//! They see only the work, the Changes view's state and the selection; what a click asks for is
//! returned ([`ListOutput`]).

use egui_widgets::shadcn::Button;
use gui_shared::i18n::{I18nKey, I18nMap};

use super::ChangeAction;
use crate::patch::{
    changes::{Scm, ScmRow},
    workspace::{
        Workspace,
        history::{ChangeKind, ObjectChange, Revision},
    },
};

/// What the lists are drawn with.
pub(super) struct ListInput<'a> {
    pub(super) workspace: &'a Workspace,
    pub(super) scm: &'a Scm,
    /// The selected object, of behavior `.0`: its history is shown.
    pub(super) selected: (usize, Option<&'a str>),
    /// The keyboard's row, and whether it just moved there.
    pub(super) cursor: (Option<ScmRow>, bool),
}

/// What the lists asked for.
#[derive(Debug, Default)]
pub(super) struct ListOutput {
    pub(super) picked: Option<ChangeAction>,
    /// A revision's header was clicked: it opens / closes.
    pub(super) toggled: Option<u32>,
}

/// Draws the three lists, up and down only (the rows are cut with `…`, as VS Code's).
pub(super) fn show(ui: &mut egui::Ui, i18n: &I18nMap, input: &ListInput<'_>) -> ListOutput {
    let mut output = ListOutput::default();
    egui::ScrollArea::vertical().id_salt("side_changes").auto_shrink(false).show(ui, |ui| {
        if let Some(picked) = pending(ui, i18n, input) {
            output.picked = Some(picked);
        }
        if let Some(picked) = object_history(ui, i18n, input) {
            output.picked = Some(picked);
        }

        let (picked, toggled) = history(ui, i18n, input);
        if picked.is_some() {
            output.picked = picked;
        }
        output.toggled = toggled;
    });
    output
}

/// `▼ CHANGES (2)`: the changes since the last revision.
fn pending(ui: &mut egui::Ui, i18n: &I18nMap, input: &ListInput<'_>) -> Option<ChangeAction> {
    let changes = &input.scm.pending.changes;
    let title = format!("{} ({})", i18n.t(I18nKey::SideChanges), changes.len());
    let mut picked = None;

    egui::CollapsingHeader::new(title).id_salt("side_pending").default_open(true).show(ui, |ui| {
        if changes.is_empty() {
            ui.weak(i18n.t(I18nKey::SideNoChanges));
        }
        for (at, change) in changes.iter().enumerate() {
            let keyboard = (input.cursor.0 == Some(ScmRow::Change(at)), input.cursor.1);
            if let Some(action) = change_row(ui, i18n, (input.workspace, change), None, keyboard) {
                picked = Some(action);
            }
        }
    });
    picked
}

/// `OBJECT HISTORY  #0106`: the revisions that changed the selected object, newest first, as
/// GitLens' File History. Each one's menu puts the object as it was then.
fn object_history(ui: &mut egui::Ui, i18n: &I18nMap, input: &ListInput<'_>) -> Option<ChangeAction> {
    let (active, id) = input.selected;
    let id = id?;
    let workspace = input.workspace;
    let key = (workspace.behaviors.get(active)?.meshes_path.clone(), id.to_owned());
    let mut picked = None;

    let title = format!("{}  {id}", i18n.t(I18nKey::SideObjectHistory));
    egui::CollapsingHeader::new(title).id_salt("side_object_history").default_open(true).show(ui, |ui| {
        if ui.small_button(i18n.t(I18nKey::SideBlame)).on_hover_text(i18n.t(I18nKey::SideBlameHover)).clicked() {
            picked = Some(ChangeAction::Blame(active, id.to_owned()));
        }

        let mut any = false;
        for (revision, change) in workspace.history.of_object(&key) {
            any = true;
            if let Some(action) = object_revision_row(ui, i18n, workspace, (revision, change), (active, id)) {
                picked = Some(action);
            }
        }
        if !any {
            ui.weak(i18n.t(I18nKey::SideNoObjectHistory));
        }
    });
    picked
}

/// `M #3 fix the speed  2026-11-12 18:00`: a click compares with the revision before; its menu
/// compares with the work now, or puts the object back as it was.
fn object_revision_row(
    ui: &mut egui::Ui,
    i18n: &I18nMap,
    workspace: &Workspace,
    (revision, change): (&Revision, &ObjectChange),
    (active, id): (usize, &str),
) -> Option<ChangeAction> {
    let number = revision.number;
    let parent = workspace.history.parent_number(number);
    let text = format!("{} #{number} {}  {}", change.kind.mark(), revision.message, local_time(revision.time));

    // Cut with `…` to the width left, the whole on hover when cut, as VS Code's.
    let response = egui_widgets::fitted_button(ui, false, &text);
    let mut picked = response.clicked().then(|| ChangeAction::Compare(active, id.to_owned(), parent, Some(number)));

    response.context_menu(|ui| {
        let items = [
            (I18nKey::SideComparePrevious, ChangeAction::Compare(active, id.to_owned(), parent, Some(number))),
            (I18nKey::SideCompareWorking, ChangeAction::Compare(active, id.to_owned(), number, None)),
        ];
        for (label, action) in items {
            if ui.button(i18n.t(label)).clicked() {
                picked = Some(action);
            }
        }
        ui.separator();
        if ui.add(Button::new(i18n.t(I18nKey::SideRestoreObject)).destructive()).clicked() {
            picked = Some(ChangeAction::RestoreObject(active, id.to_owned(), number));
        }
        if picked.is_some() {
            ui.close();
        }
    });
    picked
}

/// `▼ HISTORY`: each revision, open ones with their changes. Returns what was picked, and the
/// revision whose header was clicked.
fn history(ui: &mut egui::Ui, i18n: &I18nMap, input: &ListInput<'_>) -> (Option<ChangeAction>, Option<u32>) {
    let (mut picked, mut toggled) = (None, None);

    egui::CollapsingHeader::new(i18n.t(I18nKey::SideHistory)).id_salt("side_history").default_open(true).show(
        ui,
        |ui| {
            for revision in input.workspace.history.log() {
                let (asked, clicked) = revision_section(ui, i18n, input, revision);
                if asked.is_some() {
                    picked = asked;
                }
                if clicked {
                    toggled = Some(revision.number);
                }
            }
        },
    );
    (picked, toggled)
}

/// `▼ #3 fix the speed  2026-11-12 18:00  (2)` and, if open, its changes. Returns what was picked,
/// and whether the header was clicked.
fn revision_section(
    ui: &mut egui::Ui,
    i18n: &I18nMap,
    input: &ListInput<'_>,
    revision: &Revision,
) -> (Option<ChangeAction>, bool) {
    let number = revision.number;
    let (cursor, moved) = input.cursor;
    let mut picked = None;

    let title = format!("#{number} {}  {}  ({})", revision.message, local_time(revision.time), revision.changes.len());
    let shown = egui::CollapsingHeader::new(title)
        .id_salt(("side_revision", number))
        .open(Some(input.scm.open_revisions.contains(&number)))
        .show(ui, |ui| {
            for (at, change) in revision.changes.iter().enumerate() {
                let keyboard = (cursor == Some(ScmRow::RevisionChange(number, at)), moved);
                if let Some(action) = change_row(ui, i18n, (input.workspace, change), Some(number), keyboard) {
                    picked = Some(action);
                }
            }
        });

    let header = &shown.header_response;
    if cursor == Some(ScmRow::Revision(number)) {
        egui_tree::list::cursor_frame(ui, header.rect);
        if moved {
            header.scroll_to_me(None);
        }
    }

    // As GitLens' "Restore": the whole work as it was then.
    header.context_menu(|ui| {
        let restore = Button::new(i18n.t(I18nKey::SideRestoreRevision)).destructive();
        if ui.add(restore).on_hover_text(i18n.t(I18nKey::SideRestoreRevisionHover)).clicked() {
            picked = Some(ChangeAction::RestoreRevision(number));
            ui.close();
        }
    });
    (picked, header.clicked())
}

/// `M  0_master  #0106  hkbModifierList  GetUpModList`. `revision`: `None` for a change since the last
/// revision, which can be discarded; else the revision of the history it is of, which the object can
/// be put back to. A removed one can not be opened.
///
/// `(under_cursor, moved)`: the keyboard is on it (a frame), and just went there (scrolled to).
fn change_row(
    ui: &mut egui::Ui,
    i18n: &I18nMap,
    (workspace, change): (&Workspace, &ObjectChange),
    revision: Option<u32>,
    (under_cursor, moved): (bool, bool),
) -> Option<ChangeAction> {
    let behavior = workspace.behaviors.iter().position(|behavior| behavior.meshes_path == change.behavior);
    let object = behavior.and_then(|at| workspace.behaviors[at].object_text(&change.id));
    let mut picked = None;

    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(change.kind.mark()).monospace().strong().color(kind_color(ui, change.kind)));

        let text = {
            let name = behavior.map_or_else(|| file_stem(&change.behavior), |at| workspace.behaviors[at].name.as_str());
            let (class, label) = object.map_or(("", ""), |object| (object.class, object.name));
            format!("{name}  {}  {class}  {label}", change.id)
        };
        // Cut with `…` to the width left, the whole on hover when cut, as VS Code's.
        let response = egui_widgets::fitted_button(ui, false, &text);
        if under_cursor {
            egui_tree::list::cursor_frame(ui, response.rect);
            if moved {
                response.scroll_to_me(None);
            }
        }

        // As VS Code's Source Control: a change opens its object; a file of a revision, what that
        // revision changed of it.
        picked = match (response.clicked(), behavior, revision) {
            (true, Some(at), Some(number)) => {
                let parent = workspace.history.parent_number(number);
                Some(ChangeAction::Compare(at, change.id.clone(), parent, Some(number)))
            }
            (true, Some(at), None) if object.is_some() => Some(ChangeAction::Open(at, change.id.clone())),
            _ => None,
        };

        if let Some(at) = behavior {
            response.context_menu(|ui| {
                if let Some(action) = change_menu(ui, i18n, workspace, (at, &change.id), revision) {
                    picked = Some(action);
                    ui.close();
                }
            });
        }
    });
    picked
}

/// A change's menu: compare (with the revision before / the work now), then discard or restore.
fn change_menu(
    ui: &mut egui::Ui,
    i18n: &I18nMap,
    workspace: &Workspace,
    (at, id): (usize, &str),
    revision: Option<u32>,
) -> Option<ChangeAction> {
    let mut picked = None;

    // Of the history: what it changed, and against the work now. A change: against the last revision.
    let compares = match revision {
        Some(number) => vec![
            (I18nKey::SideComparePrevious, workspace.history.parent_number(number), Some(number)),
            (I18nKey::SideCompareWorking, number, None),
        ],
        None => vec![(I18nKey::SideCompareWorking, workspace.history.head_number(), None)],
    };
    for (label, from, to) in compares {
        if ui.button(i18n.t(label)).clicked() {
            picked = Some(ChangeAction::Compare(at, id.to_owned(), from, to));
        }
    }
    if revision.is_some() {
        ui.separator();
    }

    let (label, action) = match revision {
        None => (I18nKey::SideDiscard, ChangeAction::Discard(at, id.to_owned())),
        Some(number) => (I18nKey::SideRestoreObject, ChangeAction::RestoreObject(at, id.to_owned(), number)),
    };
    if ui.add(Button::new(i18n.t(label)).destructive()).clicked() {
        picked = Some(action);
    }
    picked
}

/// Source Control's colors: added green, modified yellow, removed red.
fn kind_color(ui: &egui::Ui, kind: ChangeKind) -> egui::Color32 {
    match kind {
        ChangeKind::Added => egui::Color32::from_rgb(0x73, 0xc9, 0x91),
        ChangeKind::Modified => egui::Color32::from_rgb(0xe2, 0xc0, 0x8d),
        ChangeKind::Removed => ui.visuals().error_fg_color,
    }
}

/// `meshes/actors/character/behaviors/0_master.bin` -> `0_master`: a behavior the work no longer has.
fn file_stem(meshes_path: &str) -> &str {
    let file = meshes_path.rsplit('/').next().unwrap_or(meshes_path);
    file.rsplit_once('.').map_or(file, |(stem, _)| stem)
}

/// Seconds since the Unix epoch as the local time, e.g. `2026-11-12 18:00`.
fn local_time(seconds: u64) -> String {
    let seconds = i64::try_from(seconds).unwrap_or_default();
    chrono::DateTime::from_timestamp(seconds, 0)
        .map(|time| time.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_default()
}
