//! The "animation data" window: which clips of the mod get data, and which hkx join which anim sets.
//!
//! ```text
//! ┌ Animation data ──────────────────────────────────────────┐
//! │ D:/mods/Paraglider                                       │
//! │ ▼ Clips                                                  │
//! │   ☑ Paraglide_Equip  Animations\..\Equip.hkx   3.30 s    │
//! │   ☐ Broken           Animations\..\Gone.hkx    ✖         │  <- hover: why
//! │ ▼ Anim sets                                              │
//! │   ☑ Also add the checked hkx to their characters         │  <- anim::character
//! │   Project: [DefaultMaleData~DefaultMale (1) ▼]           │
//! │   hkx  1 / 12  [All] [None]   txt  1 / 374  [All] [None] │
//! │   [filter               ]     [filter                ]   │
//! │   ☑ meshes\..\Equip.hkx       ☑ 1HMSolo.txt              │
//! │   ☐ meshes\..\Idle.hkx        ☐ 1HMDual.txt              │
//! │ [Generate]                                               │
//! └──────────────────────────────────────────────────────────┘
//! ```
//! The two halves are independent: the user decides which hkx go to which txt.
//! See [`crate::patch::anim::data`] for what is generated.

use std::path::{Path, PathBuf};

use egui_widgets::{
    checklist::{Checklist, ChecklistLabels},
    notify::Notification,
    shadcn::{Button, ComboBox, TextEdit},
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
    /// The checked hkx also go to the `animationNames` of their actors' characters. See
    /// [`crate::patch::anim::character`].
    add_names: bool,
}

/// What a form's window asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::patch) enum FormAsk {
    /// Its button: generate / create / duplicate.
    Done,
    /// `✖` or `Esc`.
    Close,
}

impl AnimDataDialog {
    /// Draws the window. Returns what was asked.
    pub(super) fn window(&mut self, ctx: &egui::Context, i18n: &I18nMap) -> Option<FormAsk> {
        let mut open = true;
        let window = egui_widgets::window_near_pointer(ctx, i18n.t(I18nKey::NemesisAnimData)).open(&mut open);
        let generate =
            window.default_width(720.0).show(ctx, |ui| dialog_ui(ui, i18n, self)).and_then(|shown| shown.inner);

        if generate == Some(true) { Some(FormAsk::Done) } else { (!open).then_some(FormAsk::Close) }
    }

    fn checked_clips(&self) -> usize {
        self.rows.iter().filter(|row| row.checked).count()
    }

    fn checked_sets(&self) -> usize {
        self.projects.iter().map(|project| project.files.checked_count()).sum()
    }

    /// Whether `Generate` has anything to do. hkx without a txt (or the reverse) is not a request,
    /// unless they go to the characters.
    fn can_generate(&self) -> bool {
        let hkx = self.hkx.checked_count() > 0;
        self.checked_clips() > 0 || (hkx && (self.checked_sets() > 0 || self.add_names))
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
        Self { root: root.to_path_buf(), rows, hkx: Checklist::new(hkx), projects, project, add_names: true }
    }

    /// `Generate`: what was checked goes into `work`, and `sel` goes to the generated files.
    pub(in crate::patch) fn generate(
        self,
        (work, sel): (&mut Work, &mut Selection),
        i18n: &I18nMap,
        notify: &mut Notification,
    ) {
        let code = work.place.mod_code.trim().to_owned();
        let chosen: Vec<(&ClipSource, HkxInfo)> =
            self.rows.iter().filter(|row| row.checked).filter_map(chosen_clip).collect();
        let sets: Vec<String> = self.projects.iter().flat_map(project_set_ids).collect();
        let request = Generate {
            code: &code,
            clips: chosen.iter().map(|(clip, info)| (*clip, info)).collect(),
            push: AnimSetPush {
                hkx: self.hkx.checked_items().collect(),
                sets: sets.iter().map(String::as_str).collect(),
            },
        };
        let summary = format!(
            "{}: {} clips, {} hkx -> {} txt",
            i18n.t(I18nKey::NemesisAnimDataDone),
            request.clips.len(),
            request.push.hkx.len(),
            request.push.sets.len(),
        );

        let first_result = if request.clips.is_empty() { ASDSF } else { ADSF };
        let root = PathBuf::from(&work.place.template_dir);
        if let Err(err) = data::generate(&mut work.workspace, &root, &request) {
            notify.error(err);
            return;
        }
        notify.success(summary);
        if self.add_names {
            add_names_to_characters(work, (&root, &request.push.hkx), i18n, notify);
        }

        // To the generated files.
        work.reset_names();
        work.mark_dirty();
        sel.active = work.workspace.behaviors.iter().position(|b| b.name == first_result).unwrap_or(sel.active);
        sel.selected = None;
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
        notify.success(format!("{}: {}", i18n.t(I18nKey::NemesisAnimNamesAdded), done.characters.join(", ")));
    }
    if !done.no_character.is_empty() {
        notify.warn(format!("{}: {}", i18n.t(I18nKey::NemesisAnimNamesNoCharacter), done.no_character.join(", ")));
    }
    work.reset_names();
}

/// `DefaultMaleData~DefaultMale/1HMSolo.txt` for each checked file of the project.
fn project_set_ids(project: &SetProject) -> impl Iterator<Item = String> + '_ {
    project.files.checked_items().map(|file| format!("{}/{file}", project.folder))
}

/// The window's contents. Returns whether `Generate` was clicked.
fn dialog_ui(ui: &mut egui::Ui, i18n: &I18nMap, dialog: &mut AnimDataDialog) -> bool {
    ui.weak(dialog.root.to_string_lossy().as_ref());
    egui::CollapsingHeader::new(i18n.t(I18nKey::NemesisAnimDataClips)).default_open(true).show(ui, |ui| {
        clips_ui(ui, i18n, &mut dialog.rows);
    });
    egui::CollapsingHeader::new(i18n.t(I18nKey::NemesisAnimDataSet))
        .default_open(true)
        .show(ui, |ui| {
            anim_sets_ui(ui, i18n, dialog);
        })
        .header_response
        .on_hover_text(i18n.t(I18nKey::NemesisAnimDataSetHover));
    ui.separator();
    ui.add_enabled(dialog.can_generate(), Button::new(i18n.t(I18nKey::NemesisAnimDataGenerate)).primary()).clicked()
}

fn clips_ui(ui: &mut egui::Ui, i18n: &I18nMap, rows: &mut [ClipRow]) {
    if rows.is_empty() {
        ui.colored_label(ui.visuals().warn_fg_color, i18n.t(I18nKey::NemesisAnimDataNoClips));
        return;
    }
    egui::ScrollArea::vertical().id_salt("nemesis_anim_clips").max_height(200.0).show(ui, |ui| {
        for row in rows {
            clip_row(ui, row);
        }
    });
}

/// The project, and side by side the hkx and the txt of that project.
fn anim_sets_ui(ui: &mut egui::Ui, i18n: &I18nMap, dialog: &mut AnimDataDialog) {
    ui.checkbox(&mut dialog.add_names, i18n.t(I18nKey::NemesisAnimNamesToCharacters))
        .on_hover_text(i18n.t(I18nKey::NemesisAnimNamesToCharactersHover));
    project_combo(ui, i18n, dialog);
    let AnimDataDialog { hkx, projects, project, .. } = dialog;
    let labels = ChecklistLabels {
        all: i18n.t(I18nKey::FilterTextAll),
        none: i18n.t(I18nKey::NemesisAnimDataNone),
        filter: i18n.t(I18nKey::FilterLabel),
    };
    ui.columns(2, |columns| {
        if hkx.is_empty() {
            columns[0].colored_label(columns[0].visuals().warn_fg_color, i18n.t(I18nKey::NemesisAnimDataNoHkx));
        } else {
            hkx.show(&mut columns[0], "hkx", &labels, LIST_HEIGHT);
        }
        if let Some(project) = projects.get_mut(*project) {
            project.files.show(&mut columns[1], "txt", &labels, LIST_HEIGHT);
        }
    });
}

/// `Project: [DefaultMaleData~DefaultMale (1)]`: the count tells where checks are, as each project
/// keeps its own.
fn project_combo(ui: &mut egui::Ui, i18n: &I18nMap, dialog: &mut AnimDataDialog) {
    let title = |project: &SetProject| match project.files.checked_count() {
        0 => project.folder.clone(),
        count => format!("{} ({count})", project.folder),
    };
    ui.horizontal(|ui| {
        ui.label(i18n.t(I18nKey::NemesisAnimDataProject));
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

/// `☑ Paraglide_Equip  Animations\..\Equip.hkx  3.30 s`, or `✖` with why on hover.
/// Below it, the annotations to write as triggers can be turned off.
fn clip_row(ui: &mut egui::Ui, row: &mut ClipRow) {
    ui.horizontal(|ui| {
        ui.add_enabled(row.info.is_ok(), egui::Checkbox::new(&mut row.checked, &row.clip.name));
        ui.weak(&row.clip.animation);
        match &row.info {
            Ok(info) => ui.label(format!("{:.2} s", info.duration)),
            Err(err) => ui.colored_label(ui.visuals().error_fg_color, "✖").on_hover_text(err),
        };
    });
    let Ok(info) = &row.info else { return };
    if info.triggers.is_empty() {
        return;
    }
    let on = row.triggers_on.iter().filter(|on| **on).count();
    let title = format!("{on} / {} triggers", info.triggers.len());
    egui::CollapsingHeader::new(title).id_salt(("nemesis_clip_triggers", &row.clip.name)).show(ui, |ui| {
        for ((time, trigger), on) in info.triggers.iter().zip(&mut row.triggers_on) {
            ui.checkbox(on, egui::RichText::new(format!("{trigger}:{time}")).monospace());
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
        let window = egui_widgets::window_near_pointer(ctx, i18n.t(I18nKey::NemesisNewClips)).open(&mut open);
        window.collapsible(false).show(ctx, |ui| {
            self.fields(ui, i18n);
            create = ui.add_enabled(self.is_valid(), Button::new(i18n.t(I18nKey::NemesisCreate)).primary()).clicked();
        });

        if create { Some(FormAsk::Done) } else { (!open).then_some(FormAsk::Close) }
    }

    /// `Actor [..]  Name [..]  Count [..]`, each with its hover.
    fn fields(&mut self, ui: &mut egui::Ui, i18n: &I18nMap) {
        egui::Grid::new("nemesis_new_clips").num_columns(2).show(ui, |ui| {
            let hover = i18n.t(I18nKey::NemesisClipActorHover);
            ui.label(i18n.t(I18nKey::NemesisClipActor)).on_hover_text(hover);
            actor_field(ui, i18n, &mut self.folder, &self.projects, hover);
            ui.end_row();

            let hover = i18n.t(I18nKey::NemesisClipNameHover);
            ui.label(i18n.t(I18nKey::ColumnName)).on_hover_text(hover);
            ui.add(TextEdit::singleline(&mut self.name).desired_width(220.0)).on_hover_text(hover);
            ui.end_row();

            let hover = i18n.t(I18nKey::NemesisClipCountHover);
            ui.label(i18n.t(I18nKey::NemesisClipCount)).on_hover_text(hover);
            ui.add(egui::DragValue::new(&mut self.count).range(1..=999)).on_hover_text(hover);
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
        ui.add(TextEdit::singleline(folder).desired_width(220.0)).on_hover_text(hover);
        return;
    }
    let mut index = projects.iter().position(|project| project == folder).unwrap_or(usize::MAX);
    let mut created = None;
    let new_label = i18n.t(I18nKey::NemesisNewProject);
    let picked = ComboBox::from_id_salt("nemesis_clip_actor")
        .selected_text(folder.as_str())
        .search_hint(i18n.t(I18nKey::SearchLabel))
        .show_creatable(ui, &mut index, projects.len(), |i| projects[i].as_str(), (new_label, &mut created))
        .on_hover_text(hover);
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
        let window = egui_widgets::window_near_pointer(ctx, i18n.t(I18nKey::NemesisDuplicateTo)).open(&mut open);
        window.collapsible(false).resizable(false).show(ctx, |ui| {
            ui.label(format!("{}: {}", i18n.t(I18nKey::NemesisDuplicateFiles), self.files.len()))
                .on_hover_text(self.files.join("\n"));
            ui.horizontal(|ui| {
                let projects = &self.projects;
                ComboBox::from_id_salt("nemesis_duplicate_to").search_hint(i18n.t(I18nKey::SearchLabel)).show_index(
                    ui,
                    &mut self.picked,
                    projects.len(),
                    |i| projects[i].as_str(),
                );
                let button = Button::new(i18n.t(I18nKey::NemesisDuplicate)).primary();
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
    pub(in crate::patch) fn picked(self) -> Option<(Vec<String>, String)> {
        let to = self.projects.into_iter().nth(self.picked)?;
        Some((self.files, to))
    }
}

/// Where a file of the animation data `active` of `work` can go. The template's are read once.
pub(in crate::patch) fn anim_projects(work: &mut Work, active: usize) -> Vec<String> {
    let is_adsf = work.workspace.behaviors[active].adsf;
    if is_adsf && work.template_projects.is_none() {
        work.template_projects = Some(data::template_projects(Path::new(&work.place.template_dir)));
    }
    let template = if is_adsf { work.template_projects.as_deref().unwrap_or_default() } else { &[] };
    data::project_folders(&work.workspace.behaviors[active], template)
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
    pub(in crate::patch) fn create(self, (work, sel): (&mut Work, &mut Selection)) {
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
