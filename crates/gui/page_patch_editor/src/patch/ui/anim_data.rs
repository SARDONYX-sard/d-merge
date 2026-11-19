//! The "animation data" window: which clips of the mod get data, and where the mod's hkx are added.
//!
//! ```text
//! ┌ Animation data ──────────────────────────────────────────┐
//! │ D:/mods/Paraglider                                       │
//! │ ▼ Clips                                                  │
//! │   ☑ Paraglide_Equip  Animations\..\Equip.hkx   3.30 s    │
//! │   ☐ Broken           Animations\..\Gone.hkx    ✖         │  <- hover: why
//! │   [Generate their data]                                  │  <- animationdatasinglefile only
//! │ ▼ The mod's hkx                                          │
//! │   hkx  1 / 12  [All] [None]  [filter            ]        │
//! │   ☑ meshes\..\Equip.hkx                                  │
//! │   [Add to their characters' animationNames]              │  <- the characters' XML only
//! │   ▼ Anim sets: what the game caches                      │
//! │       Project: [DefaultMaleData~DefaultMale (1) ▼]       │
//! │       txt  1 / 374  [All] [None]  [filter       ]        │
//! │       ☑ 1HMSolo.txt                                      │
//! │       [Add to the checked txt]                           │  <- animationsetdatasinglefile only
//! └──────────────────────────────────────────────────────────┘
//! ```
//! Each button does its part alone, and the window stays: adding the hkx to the characters changes
//! no clip data. The hkx are chosen once for both of their buttons.
//! See [`crate::patch::anim::data`] for what is generated.

use std::path::{Path, PathBuf};

use egui_widgets::{
    HoverMarkdown as _,
    checklist::{Checklist, ChecklistLabels},
    notify::Notification,
    shadcn::{Button, Checkbox, ComboBox, NumberInput, TextEdit},
};
use gui_shared::i18n::{I18nKey, I18nMap};

use crate::patch::{
    anim::data::{self, AnimSetPush, ClipSource, Generate, HkxInfo},
    tool::{selection::Selection, work_state::Work},
    workspace::{ADSF, ASDSF},
};

/// The height of one row of the hkx and txt lists.
const LIST_HEIGHT: f32 = 220.0;

/// A clip in the list.
#[derive(Debug)]
struct ClipRow {
    clip: ClipSource,
    /// `Err`: why its hkx could not be read. Such a clip cannot be chosen.
    info: Result<HkxInfo, String>,
    checked: bool,
    /// Per annotation of the hkx: is it written as a trigger? Annotations are free text, so all are
    /// on until the user turns one off.
    triggers_on: Vec<bool>,
}

/// The anim set files of one project. The checks stay when another project is shown.
#[derive(Debug)]
struct SetProject {
    /// e.g. `DefaultMaleData~DefaultMale`
    folder: String,
    files: Checklist,
}

/// See the module docs.
#[derive(Debug)]
pub(super) struct AnimDataDialog {
    root: PathBuf,
    rows: Vec<ClipRow>,
    /// The mod's animations, as [`data::anim_hkx_files`] gives them.
    hkx: Checklist,
    projects: Vec<SetProject>,
    /// The project shown, in `projects`.
    project: usize,
}

/// What one button of the window does: only its part. See the module docs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AnimDataPart {
    /// The data of the checked clips, in `animationdatasinglefile`.
    Clips,
    /// The checked hkx to the `animationNames` of their actors' characters: see
    /// [`crate::patch::anim::character`].
    Names,
    /// The checked hkx to the checked txt of `animationsetdatasinglefile`.
    Sets,
}

/// What the window asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AnimDataAsk {
    Do(AnimDataPart),
    /// `✖`.
    Close,
}

/// What a form's window asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FormAsk {
    /// Its button: generate / create / duplicate.
    Done,
    /// `✖` or `Esc`.
    Close,
}

impl AnimDataDialog {
    /// Draws the window. Returns what was asked.
    pub(super) fn window(&mut self, ctx: &egui::Context, i18n: &I18nMap) -> Option<AnimDataAsk> {
        let mut open = true;
        let window = egui_widgets::window_near_pointer(ctx, i18n.t(I18nKey::AnimDataOpen)).open(&mut open);
        let part = window.default_width(560.0).show(ctx, |ui| dialog_ui(ui, i18n, self)).and_then(|shown| shown.inner);

        match part.flatten() {
            Some(part) => Some(AnimDataAsk::Do(part)),
            None => (!open).then_some(AnimDataAsk::Close),
        }
    }

    fn checked_clips(&self) -> usize {
        self.rows.iter().filter(|row| row.checked).count()
    }

    fn checked_sets(&self) -> usize {
        self.projects.iter().map(|project| project.files.checked_count()).sum()
    }

    /// Has `part` anything to do? Clips checked; hkx checked (and txt, for the anim sets).
    fn can_do(&self, part: AnimDataPart) -> bool {
        let hkx = self.hkx.checked_count() > 0;
        match part {
            AnimDataPart::Clips => self.checked_clips() > 0,
            AnimDataPart::Names => hkx,
            AnimDataPart::Sets => hkx && self.checked_sets() > 0,
        }
    }
}

impl AnimDataDialog {
    /// The window for the mod at `root`: reads the hkx of every clip of `work`, and lists the hkx of
    /// the mod and the anim set files of the template.
    pub(in crate::patch) fn open(work: &Work, root: &Path) -> Self {
        let clips = data::clip_sources(&work.workspace.behaviors);
        let rows = clips
            .into_iter()
            .map(|clip| {
                let info = data::read_hkx(root, &clip);
                let checked = info.is_ok();
                let triggers_on = info.as_ref().map_or_else(|_| Vec::new(), |info| vec![true; info.triggers.len()]);
                ClipRow { clip, info, checked, triggers_on }
            })
            .collect();
        let hkx = data::anim_hkx_files(root);
        let sets = work.workspace.anim_set_projects(Path::new(&work.place.template_dir));
        let folders: Vec<&str> = sets.iter().map(|(folder, _)| folder.as_str()).collect();
        let project = data::default_project(&folders, hkx.first().map(String::as_str));
        let projects =
            sets.into_iter().map(|(folder, files)| SetProject { folder, files: Checklist::new(files) }).collect();
        Self { root: root.to_path_buf(), rows, hkx: Checklist::new(hkx), projects, project }
    }

    /// Does `part` alone into `work`, with what is checked. `sel` goes to what it wrote.
    pub(super) fn apply(
        &self,
        part: AnimDataPart,
        (work, sel): (&mut Work, &mut Selection),
        i18n: &I18nMap,
        notify: &mut Notification,
    ) {
        let root = PathBuf::from(&work.place.template_dir);
        let hkx: Vec<&str> = self.hkx.checked_items().collect();
        let written = match part {
            AnimDataPart::Clips => self.generate_clips(work, &root, (i18n, notify)).then_some(ADSF),
            AnimDataPart::Names => {
                add_names_to_characters(work, (&root, &hkx), i18n, notify);
                None // characters of several actors: the editor stays where it is
            }
            AnimDataPart::Sets => self.add_to_sets(work, (&root, &hkx), (i18n, notify)).then_some(ASDSF),
        };

        work.reset_names();
        work.mark_dirty();
        if let Some(name) = written {
            sel.active = work.workspace.behaviors.iter().position(|b| b.name == name).unwrap_or(sel.active);
            sel.selected = None;
        }
    }

    /// The data of the checked clips. Returns whether it was written.
    fn generate_clips(&self, work: &mut Work, root: &Path, (i18n, notify): (&I18nMap, &mut Notification)) -> bool {
        let code = work.place.mod_code.trim().to_owned();
        let chosen: Vec<(&ClipSource, HkxInfo)> =
            self.rows.iter().filter(|row| row.checked).filter_map(chosen_clip).collect();
        let clips = chosen.iter().map(|(clip, info)| (*clip, info)).collect();
        let request = Generate { code: &code, clips, push: AnimSetPush::default() };
        if let Err(err) = data::generate(&mut work.workspace, root, &request) {
            notify.error(err);
            return false;
        }
        notify.success(format!("{}: {} clips", i18n.t(I18nKey::AnimDataDone), request.clips.len()));
        true
    }

    /// The checked `hkx` to the checked txt. Returns whether they were added.
    fn add_to_sets(
        &self,
        work: &mut Work,
        (root, hkx): (&Path, &[&str]),
        (i18n, notify): (&I18nMap, &mut Notification),
    ) -> bool {
        let code = work.place.mod_code.trim().to_owned();
        let sets: Vec<String> = self.projects.iter().flat_map(project_set_ids).collect();
        let push = AnimSetPush { hkx: hkx.to_vec(), sets: sets.iter().map(String::as_str).collect() };
        let request = Generate { code: &code, clips: Vec::new(), push };
        if let Err(err) = data::generate(&mut work.workspace, root, &request) {
            notify.error(err);
            return false;
        }
        let done = i18n.t(I18nKey::AnimDataDone);
        notify.success(format!("{done}: {} hkx -> {} txt", request.push.hkx.len(), request.push.sets.len()));
        true
    }
}

/// The checked hkx (of the templates at `root`) to the `animationNames` of their characters, those
/// the work's scope allows.
fn add_names_to_characters(work: &mut Work, (root, hkx): (&Path, &[&str]), i18n: &I18nMap, notify: &mut Notification) {
    let scope = work.workspace.scope;
    let allows = |behavior: &crate::patch::workspace::Behavior| scope.allows(behavior.nemesis.is_some());
    let done = match work.workspace.add_animation_names(root, hkx, allows) {
        Ok(done) => done,
        Err(err) => return notify.error(err),
    };

    if !done.characters.is_empty() {
        notify.success(format!("{}: {}", i18n.t(I18nKey::AnimNamesAdded), done.characters.join(", ")));
    }
    if !done.no_character.is_empty() {
        notify.warn(format!("{}: {}", i18n.t(I18nKey::AnimNamesNoCharacter), done.no_character.join(", ")));
    }
    work.reset_names();
}

/// `DefaultMaleData~DefaultMale/1HMSolo.txt` for each checked file of the project.
fn project_set_ids(project: &SetProject) -> impl Iterator<Item = String> + '_ {
    project.files.checked_items().map(|file| format!("{}/{file}", project.folder))
}

/// The window's contents. Returns the part whose button was clicked.
fn dialog_ui(ui: &mut egui::Ui, i18n: &I18nMap, dialog: &mut AnimDataDialog) -> Option<AnimDataPart> {
    let mut clicked = None;
    ui.weak(dialog.root.to_string_lossy().as_ref());
    egui::CollapsingHeader::new(i18n.t(I18nKey::AnimDataClips)).default_open(true).show(ui, |ui| {
        clips_ui(ui, i18n, &mut dialog.rows);
        clicked = part_button(ui, dialog, (AnimDataPart::Clips, i18n.t(I18nKey::AnimDataGenerate))).or(clicked);
    });
    egui::CollapsingHeader::new(i18n.t(I18nKey::AnimDataHkx)).default_open(true).show(ui, |ui| {
        clicked = hkx_ui(ui, i18n, dialog).or(clicked);
    });
    clicked
}

/// The button of `part`, enabled if it has anything to do. Returns `part` if clicked.
fn part_button(
    ui: &mut egui::Ui,
    dialog: &AnimDataDialog,
    (part, label): (AnimDataPart, &str),
) -> Option<AnimDataPart> {
    let button = Button::new(label).primary();
    ui.add_enabled(dialog.can_do(part), button).clicked().then_some(part)
}

fn clips_ui(ui: &mut egui::Ui, i18n: &I18nMap, rows: &mut [ClipRow]) {
    if rows.is_empty() {
        ui.colored_label(ui.visuals().warn_fg_color, i18n.t(I18nKey::AnimDataNoClips));
        return;
    }
    egui::ScrollArea::vertical().id_salt("nemesis_anim_clips").max_height(200.0).show(ui, |ui| {
        for row in rows {
            clip_row(ui, row);
        }
    });
}

/// The words of a checklist's row of buttons.
fn checklist_labels(i18n: &I18nMap) -> ChecklistLabels<'_> {
    ChecklistLabels {
        all: i18n.t(I18nKey::FilterTextAll),
        none: i18n.t(I18nKey::AnimDataNone),
        filter: i18n.t(I18nKey::FilterLabel),
    }
}

/// The mod's hkx to check, then where the checked ones go, each with its own button: the
/// characters, the anim sets. Returns the part whose button was clicked.
fn hkx_ui(ui: &mut egui::Ui, i18n: &I18nMap, dialog: &mut AnimDataDialog) -> Option<AnimDataPart> {
    if dialog.hkx.is_empty() {
        ui.colored_label(ui.visuals().warn_fg_color, i18n.t(I18nKey::AnimDataNoHkx));
        return None;
    }
    dialog.hkx.show(ui, "hkx", &checklist_labels(i18n), LIST_HEIGHT);

    let names = Button::new(i18n.t(I18nKey::AnimNamesToCharacters)).primary();
    let names = ui.add_enabled(dialog.can_do(AnimDataPart::Names), names);
    let clicked = names.on_hover_markdown(i18n.t(I18nKey::AnimNamesToCharactersHover)).clicked();

    let mut sets = None;
    egui::CollapsingHeader::new(i18n.t(I18nKey::AnimDataSet))
        .default_open(true)
        .show(ui, |ui| sets = anim_sets_ui(ui, i18n, dialog))
        .header_response
        .on_hover_markdown(i18n.t(I18nKey::AnimDataSetHover));
    if clicked { Some(AnimDataPart::Names) } else { sets }
}

/// The project, the txt of that project, and the button adding the checked hkx to them.
fn anim_sets_ui(ui: &mut egui::Ui, i18n: &I18nMap, dialog: &mut AnimDataDialog) -> Option<AnimDataPart> {
    project_combo(ui, i18n, dialog);
    if let Some(project) = dialog.projects.get_mut(dialog.project) {
        project.files.show(ui, "txt", &checklist_labels(i18n), LIST_HEIGHT);
    }
    part_button(ui, dialog, (AnimDataPart::Sets, i18n.t(I18nKey::AnimDataSetAdd)))
}

/// `Project: [DefaultMaleData~DefaultMale (1)]`: the count tells where checks are, as each project
/// keeps its own.
fn project_combo(ui: &mut egui::Ui, i18n: &I18nMap, dialog: &mut AnimDataDialog) {
    let title = |project: &SetProject| match project.files.checked_count() {
        0 => project.folder.clone(),
        count => format!("{} ({count})", project.folder),
    };
    ui.horizontal(|ui| {
        ui.label(i18n.t(I18nKey::AnimDataProject));
        let projects = &dialog.projects;
        ComboBox::from_id_salt("nemesis_anim_project")
            .search_hint(i18n.t(I18nKey::SearchLabel))
            .height(320.0)
            .show_index(ui, &mut dialog.project, projects.len(), |i| title(&projects[i]));
    });
}

/// The clip with only the triggers left on.
fn chosen_clip(row: &ClipRow) -> Option<(&ClipSource, HkxInfo)> {
    let mut info = row.info.as_ref().ok()?.clone();
    let mut on = row.triggers_on.iter();
    info.triggers.retain(|_| on.next().copied().unwrap_or(true));
    Some((&row.clip, info))
}

/// `☑ Paraglide_Equip  Animations\..\Equip.hkx  3.30 s`, or `✖` with why on hover. Read from a DAR / OAR
/// hkx standing in for it: `OAR` after it, its path on hover.
/// Below it, the annotations to write as triggers can be turned off.
fn clip_row(ui: &mut egui::Ui, row: &mut ClipRow) {
    ui.horizontal(|ui| {
        ui.add_enabled(row.info.is_ok(), egui::Checkbox::new(&mut row.checked, &row.clip.name));
        ui.weak(&row.clip.animation);
        match &row.info {
            Ok(info) => ui.label(format!("{:.2} s", info.duration)),
            Err(err) => ui.colored_label(ui.visuals().error_fg_color, "✖").on_hover_text(err),
        };
        let stand_in = row.info.as_ref().ok().and_then(|info| info.stand_in.as_deref());
        if let Some(stand_in) = stand_in {
            let replacer =
                crate::patch::anim::replacer::stands_for(stand_in).map(|(replacer, _)| replacer.short_name());
            ui.weak(replacer.unwrap_or_default()).on_hover_text(stand_in);
        }
    });
    let Ok(info) = &row.info else { return };
    if info.triggers.is_empty() {
        return;
    }
    let on = row.triggers_on.iter().filter(|on| **on).count();
    let title = format!("{on} / {} triggers", info.triggers.len());
    egui::CollapsingHeader::new(title).id_salt(("nemesis_clip_triggers", &row.clip.name)).show(ui, |ui| {
        for ((time, trigger), on) in info.triggers.iter().zip(&mut row.triggers_on) {
            ui.add(Checkbox::new(on, egui::RichText::new(format!("{trigger}:{time}")).monospace()));
        }
    });
}

/// The fields of the "new clips" window: `add_numbered_clips` takes them.
///
/// ```text
/// Actor  [DefaultMale~1]    Name  [Attack]    Count  [3]    [Create]
/// ```
/// -> `Attack_0`, `Attack_1`, `Attack_2`, with `mymod$N` numbered after what exists.
#[derive(Debug)]
pub(super) struct NewClipsForm {
    folder: String,
    name: String,
    count: usize,
    /// What `folder` can be: its candidates. See [`data::project_folders`].
    projects: Vec<String>,
}

impl NewClipsForm {
    /// Draws the window: actor, name, count, `Create`. Returns what was asked.
    pub(super) fn window(&mut self, ctx: &egui::Context, i18n: &I18nMap) -> Option<FormAsk> {
        let mut open = !ctx.input(|i| i.key_pressed(egui::Key::Escape));
        let mut create = false;
        let window = egui_widgets::window_near_pointer(ctx, i18n.t(I18nKey::AnimDataNewClips)).open(&mut open);
        window.collapsible(false).show(ctx, |ui| {
            self.fields(ui, i18n);
            create = ui.add_enabled(self.is_valid(), Button::new(i18n.t(I18nKey::CreateButton)).primary()).clicked();
        });

        if create { Some(FormAsk::Done) } else { (!open).then_some(FormAsk::Close) }
    }

    /// `Actor [..]  Name [..]  Count [..]`, each with its hover.
    fn fields(&mut self, ui: &mut egui::Ui, i18n: &I18nMap) {
        egui::Grid::new("nemesis_new_clips").num_columns(2).show(ui, |ui| {
            let hover = i18n.t(I18nKey::ClipActorHover);
            ui.label(i18n.t(I18nKey::ClipActor)).on_hover_markdown(hover);
            actor_field(ui, i18n, &mut self.folder, &self.projects, hover);
            ui.end_row();

            let hover = i18n.t(I18nKey::ClipNameHover);
            ui.label(i18n.t(I18nKey::ColumnName)).on_hover_markdown(hover);
            ui.add(TextEdit::singleline(&mut self.name).desired_width(220.0)).on_hover_markdown(hover);
            ui.end_row();

            let hover = i18n.t(I18nKey::ClipCountHover);
            ui.label(i18n.t(I18nKey::ClipCount)).on_hover_markdown(hover);
            ui.add(NumberInput::new(&mut self.count).range(1..=999).desired_width(80.0)).on_hover_markdown(hover);
            ui.end_row();
        });
    }

    /// `selected`: the open file, whose actor is the likely one.
    pub(super) fn new(selected: Option<&str>, projects: Vec<String>) -> Self {
        let folder = selected.and_then(|id| id.split_once('/')).map_or("DefaultMale~1", |(folder, _)| folder);
        Self { folder: folder.to_owned(), name: String::new(), count: 1, projects }
    }

    /// `<actor>~<index>`, and a name.
    fn is_valid(&self) -> bool {
        self.folder.contains('~') && !self.name.trim().is_empty() && !self.name.contains(['/', '\\', '~', '$'])
    }
}

/// A project that was typed, as a folder of clip data: `<name>~<number>`.
///
/// e.g. `Mine` -> `Mine~1`, `Mine~2` -> as it is
fn project_folder_of(typed: &str) -> String {
    if typed.contains('~') { typed.to_owned() } else { format!("{typed}~1") }
}

/// The project of the new clips: searched among those there are, or a new one by what is searched for.
/// With no list (a template that is not a `.bin`), it is typed.
fn actor_field(ui: &mut egui::Ui, i18n: &I18nMap, folder: &mut String, projects: &[String], hover: &str) {
    if projects.is_empty() {
        ui.add(TextEdit::singleline(folder).desired_width(220.0)).on_hover_markdown(hover);
        return;
    }
    let mut index = projects.iter().position(|project| project == folder).unwrap_or(usize::MAX);
    let mut created = None;
    let new_label = i18n.t(I18nKey::AnimDataNewProject);
    let picked = ComboBox::from_id_salt("nemesis_clip_actor")
        .selected_text(folder.as_str())
        .search_hint(i18n.t(I18nKey::SearchLabel))
        .show_creatable(ui, &mut index, projects.len(), |i| projects[i].as_str(), (new_label, &mut created))
        .on_hover_markdown(hover);
    if let Some(typed) = created {
        *folder = project_folder_of(&typed);
    } else if picked.changed() {
        projects[index].clone_into(folder);
    }
}

/// `Duplicate to...`: which project, searched among the others.
#[derive(Debug)]
pub(super) struct DuplicateForm {
    /// What goes there. e.g. `DefaultMale~1/Idle~mod$0.txt`
    files: Vec<String>,
    /// The project that usually has the same patches first.
    projects: Vec<String>,
    /// In `projects`.
    picked: usize,
}

impl DuplicateForm {
    /// Draws the window: how many files, and the project to pick. Returns what was asked.
    pub(super) fn window(&mut self, ctx: &egui::Context, i18n: &I18nMap) -> Option<FormAsk> {
        let mut open = !ctx.input(|i| i.key_pressed(egui::Key::Escape));
        let mut go = false;
        let window = egui_widgets::window_near_pointer(ctx, i18n.t(I18nKey::DuplicateTo)).open(&mut open);
        window.collapsible(false).resizable(false).show(ctx, |ui| {
            ui.label(format!("{}: {}", i18n.t(I18nKey::AnimDataDuplicateFiles), self.files.len()))
                .on_hover_text(self.files.join("\n"));
            ui.horizontal(|ui| {
                let projects = &self.projects;
                ComboBox::from_id_salt("nemesis_duplicate_to").search_hint(i18n.t(I18nKey::SearchLabel)).show_index(
                    ui,
                    &mut self.picked,
                    projects.len(),
                    |i| projects[i].as_str(),
                );
                let button = Button::new(i18n.t(I18nKey::DuplicateButton)).primary();
                go = ui.add_enabled(!projects.is_empty(), button).clicked();
            });
        });

        if go { Some(FormAsk::Done) } else { (!open).then_some(FormAsk::Close) }
    }

    /// `current`: the project the files are in.
    pub(in crate::patch) fn new(files: Vec<String>, mut projects: Vec<String>, current: &str) -> Self {
        projects.retain(|project| project != current);
        data::partner_first(&mut projects, current);
        Self { files, projects, picked: 0 }
    }
}

impl DuplicateForm {
    /// The project picked, and the files: `None` if there is none to pick.
    pub(super) fn picked(self) -> Option<(Vec<String>, String)> {
        let to = self.projects.into_iter().nth(self.picked)?;
        Some((self.files, to))
    }
}

/// `files` of the animation data `active` of `work` go to project `to`.
pub(in crate::patch) fn duplicate_files(work: &mut Work, active: usize, (files, to): (&[String], &str)) {
    let made = data::duplicate_to(&mut work.workspace.behaviors[active], files, to);
    for id in &made {
        work.object_changed(id);
    }
}

impl NewClipsForm {
    /// `Create`: the clips go into the animation data `sel.active` of `work`; the first is shown.
    pub(super) fn create(self, (work, sel): (&mut Work, &mut Selection)) {
        let code = work.place.mod_code.trim().to_owned();
        let adsf = &mut work.workspace.behaviors[sel.active];
        let added = data::add_numbered_clips(adsf, &code, self.folder.trim(), self.name.trim(), self.count);
        if let Some(first) = added.first() {
            sel.selected = Some(first.as_str().into());
        }
        for id in &added {
            work.object_changed(id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_typed_project_is_a_folder_of_clip_data() {
        assert_eq!(project_folder_of("Mine"), "Mine~1");
        assert_eq!(project_folder_of("Mine~2"), "Mine~2");
    }
}
