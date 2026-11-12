//! Checking many patches at once, on request.
//!
//! The editor only checks the object that is open: going through all of them at each edit would be
//! heavy, and a mod has thousands. So a row's menu asks for it, and a window lists what was found.
//!
//! ```text
//! ┌ Check results ─────────────────────────────────────────────────┐
//! │ Errors 2 / Warnings 1                                          │
//! │ ✖ DefaultMale~1/BFCO_A~bfco$121.txt:6  The count of triggers.. │  <- a click goes there
//! │ ⚠ #0106:12  Unknown field `typoField`                          │
//! └────────────────────────────────────────────────────────────────┘
//! ```
//!
//! | Behavior  | Checked                 | What                                                    |
//! | --------- | ----------------------- | ------------------------------------------------------- |
//! | anim data | the files of the row    | d-merge's parser on what the mod changes                 |
//! | others    | what the mod changes    | fields, pointers, and d-merge's parser on its patch      |
//!
//! The window keeps up as they are corrected, one object at a time:
//! see [`PatchTool::refresh_work_check`].
//!
//! The whole work can be checked too (every behavior): its rows then say their behavior,
//! e.g. `0_master  #0106:12  ...`, and a click switches to it.

use egui_code_editor::{Diagnostic, Severity};
use gui_shared::{
    i18n::{I18nKey, I18nMap},
    settings::pages::patch::behavior::ParserMode,
};

use super::anim_tree::FileKind;
use crate::patch::{
    PatchTool,
    editor::{diagnostics, preview::Preview},
    workspace::{Behavior, ObjectText},
};

/// Which files to check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::patch) enum CheckScope {
    All,
    /// e.g. `DefaultMale~1`
    Folder(String),
    /// e.g. the clips of `DefaultMale~1`
    Kind {
        folder: String,
        kind: FileKind,
    },
}

impl CheckScope {
    /// Is the file `id` in it? e.g. `DefaultMale~1/Idle~m$0.txt`
    fn contains(&self, is_adsf: bool, id: &str) -> bool {
        let in_folder = |folder: &str| id.split_once('/').is_some_and(|(of_file, _)| of_file == folder);
        match self {
            Self::All => true,
            Self::Folder(folder) => in_folder(folder),
            Self::Kind { folder, kind } => in_folder(folder) && FileKind::of(is_adsf, id) == *kind,
        }
    }
}

/// One problem found.
#[derive(Debug, Clone, PartialEq, Eq)]
struct CheckRow {
    /// In `Workspace::behaviors`.
    behavior: usize,
    id: String,
    /// 0-based line.
    line: usize,
    message: String,
    severity: Severity,
}

/// What the window shows. See the module docs.
#[derive(Debug, Clone)]
pub(in crate::patch) struct WorkCheck {
    /// In `Workspace::behaviors`. `None`: the whole work, with [`CheckScope::All`].
    behavior: Option<usize>,
    scope: CheckScope,
    rows: Vec<CheckRow>,
}

/// Is this object checked? A template object nobody changed has nothing of the mod's to find, and a
/// file of animation data of the template is its data: those are checked by what they are.
const fn is_checked(behavior: &Behavior, object: &ObjectText<'_>) -> bool {
    behavior.is_anim_data() || object.is_new || object.is_edited
}

/// What is checked and how: the same for every object of a check.
#[derive(Debug, Clone, Copy)]
struct Checker<'a> {
    /// `index` in `Workspace::behaviors`
    index: usize,
    behavior: &'a Behavior,
    code: &'a str,
    /// The patch page's, which the parser checks by.
    mode: ParserMode,
}

/// The problems of one object.
fn object_rows(checker: Checker<'_>, object: &ObjectText<'_>) -> Vec<CheckRow> {
    let Checker { index, behavior, code, mode } = checker;
    let (id, text) = (object.id, object.text);
    // Animation data only has the parser's word: see `anim_lines`.
    let mut found: Vec<Diagnostic> = if behavior.is_anim_data() {
        Vec::new()
    } else {
        diagnostics::field_diagnostics(text, behavior, object.is_new, code)
    };
    // As the editor does: the parser reads the patch, whose lines are not the object's.
    if let Some(patch) = behavior.patch(code, id)
        && let Err(error) = behavior.validate(id, &patch, mode)
    {
        let to_editor_line = |line| Preview::new(&patch).editor_line_of(line, text);
        diagnostics::add_parser_error(&mut found, text, &error, to_editor_line);
    }
    let to_row = |problem: Diagnostic| CheckRow {
        behavior: index,
        id: id.to_owned(),
        line: problem.line,
        message: problem.message,
        severity: problem.severity,
    };
    found.into_iter().map(to_row).collect()
}

/// The problems of the files of `scope`, unsorted. The objects in parallel: a work has thousands, and
/// the parser reads each one's patch.
fn run(checker: Checker<'_>, scope: &CheckScope, rows: &mut Vec<CheckRow>) {
    use rayon::prelude::*;
    let behavior = checker.behavior;
    let objects: Vec<ObjectText<'_>> = behavior
        .texts()
        .filter(|object| {
            !behavior.is_ref(object.id) && is_checked(behavior, object) && scope.contains(behavior.adsf, object.id)
        })
        .collect();
    let found: Vec<Vec<CheckRow>> = objects.par_iter().map(|object| object_rows(checker, object)).collect();
    rows.extend(found.into_iter().flatten());
}

/// `(errors, warnings)` of `rows`.
fn counts(rows: &[CheckRow]) -> (usize, usize) {
    let errors = rows.iter().filter(|row| row.severity == Severity::Error).count();
    (errors, rows.len() - errors)
}

/// By behavior, file, line.
fn sort_rows(rows: &mut [CheckRow]) {
    rows.sort_by(|a, b| (a.behavior, &a.id, a.line).cmp(&(b.behavior, &b.id, b.line)));
}

/// `Check`, `Check all`, for a row of anim data. Returns which: `Some(true)` is the row's.
pub(super) fn check_items(ui: &mut egui::Ui, i18n: &I18nMap) -> Option<bool> {
    ui.separator();
    let hover = i18n.t(I18nKey::NemesisCheckHover);
    let mut picked = None;
    for (label, is_row) in [(I18nKey::NemesisCheck, true), (I18nKey::NemesisCheckAll, false)] {
        if ui.button(i18n.t(label)).on_hover_text(hover).clicked() {
            picked = Some(is_row);
            ui.close();
        }
    }
    picked
}

impl PatchTool {
    /// What checks the objects of behavior `index`.
    fn checker(&self, index: usize) -> Checker<'_> {
        let behavior = &self.workspace.behaviors[index];
        Checker { index, behavior, code: self.mod_code.trim(), mode: self.parser_mode }
    }

    /// Checks the files of `scope` of the active behavior, and opens the window.
    pub(in crate::patch) fn start_work_check(&mut self, scope: CheckScope) {
        let mut rows = Vec::new();
        run(self.checker(self.active), &scope, &mut rows);
        sort_rows(&mut rows);
        self.work_check = Some(WorkCheck { behavior: Some(self.active), scope, rows });
    }

    /// The problems of every behavior of the work.
    fn whole_work_check(&self) -> WorkCheck {
        let mut rows = Vec::new();
        for index in 0..self.workspace.behaviors.len() {
            run(self.checker(index), &CheckScope::All, &mut rows);
        }
        sort_rows(&mut rows);
        WorkCheck { behavior: None, scope: CheckScope::All, rows }
    }

    /// Checks every behavior of the work, and opens the window.
    pub(in crate::patch) fn start_whole_work_check(&mut self) {
        let check = self.whole_work_check();
        self.diagnostics = Some(check.clone());
        self.diagnostics_stale = false;
        self.work_check = Some(check);
    }

    /// Checks the whole work again if it changed as a whole (read, loaded, restored), as VS Code checks
    /// what it opens: its counts are in the status bar. An edit only checks its object again, see
    /// [`Self::refresh_work_check`]. Call each frame.
    pub(in crate::patch) fn diagnose_work(&mut self) {
        if self.diagnostics_stale && self.job.is_none() && self.loaded_for.is_some() {
            self.diagnostics = Some(self.whole_work_check());
            self.diagnostics_stale = false;
        }
    }

    /// The errors and warnings of the work, if it was checked: the status bar's counts.
    pub(in crate::patch) fn diagnostic_counts(&self) -> Option<(usize, usize)> {
        self.diagnostics.as_ref().map(|check| counts(&check.rows))
    }

    /// The status bar's counts were clicked: the window lists the work's problems.
    pub(in crate::patch) fn show_diagnostics(&mut self) {
        self.work_check.clone_from(&self.diagnostics);
    }

    /// The object `id` of the active behavior changed: only its rows are found again, so a correction
    /// goes from the list, even with thousands of objects. After typing has paused: in the work's
    /// counts, and in the window if it is open.
    pub(in crate::patch) fn refresh_work_check(&mut self, id: &str) {
        let index = self.active;
        if index >= self.workspace.behaviors.len() {
            return;
        }
        let checker = self.checker(index);
        let behavior = checker.behavior;
        let object =
            behavior.object_text(id).filter(|object| !behavior.is_ref(object.id) && is_checked(behavior, object));
        let found = object.map(|object| object_rows(checker, &object)).unwrap_or_default();
        let adsf = behavior.adsf;
        for check in [&mut self.work_check, &mut self.diagnostics].into_iter().flatten() {
            if check.behavior.is_some_and(|checked| checked != index) {
                continue;
            }
            check.rows.retain(|row| row.behavior != index || row.id != id);
            if check.scope.contains(adsf, id) && !found.is_empty() {
                check.rows.extend(found.iter().cloned());
                sort_rows(&mut check.rows);
            }
        }
    }

    pub(in crate::patch) fn work_check_window(&mut self, ctx: &egui::Context, i18n: &I18nMap) {
        let active = self.active;
        // A check of one behavior goes with it; one of the whole work stays.
        self.work_check = self.work_check.take().filter(|check| check.behavior.is_none_or(|checked| checked == active));
        let Some(check) = &self.work_check else { return };
        let mut open = true;
        let mut picked = None;
        let behavior_names = check.behavior.is_none().then_some(self.workspace.behaviors.as_slice());
        egui_widgets::window_near_pointer(ctx, i18n.t(I18nKey::NemesisCheckTitle))
            .open(&mut open)
            .default_size([680.0, 360.0])
            .resizable(true)
            .show(ctx, |ui| picked = check_ui(ui, i18n, &check.rows, behavior_names));
        if let Some((behavior, id, line)) = picked {
            if behavior != self.active {
                self.active = behavior;
                self.selected = None;
            }
            let current = self.selected.clone().unwrap_or_default();
            self.open_location(&current, &id, line);
        }
        if !open {
            self.work_check = None;
        }
    }
}

/// The counts and the rows. `behaviors`: those of the work, if all of them were checked, for their
/// names. Returns where a click goes: `(behavior, file, line)`.
fn check_ui(
    ui: &mut egui::Ui,
    i18n: &I18nMap,
    rows: &[CheckRow],
    behaviors: Option<&[Behavior]>,
) -> Option<(usize, String, usize)> {
    if rows.is_empty() {
        ui.weak(i18n.t(I18nKey::NemesisCheckNone));
        return None;
    }
    let errors = rows.iter().filter(|row| row.severity == Severity::Error).count();
    ui.label(format!(
        "{} {errors} / {} {}",
        i18n.t(I18nKey::NemesisCheckErrors),
        i18n.t(I18nKey::NemesisCheckWarnings),
        rows.len() - errors
    ));
    let mut picked = None;
    let row_height = ui.text_style_height(&egui::TextStyle::Monospace) + 6.0;
    egui::ScrollArea::both().id_salt("nemesis_work_check").auto_shrink(false).show_rows(
        ui,
        row_height,
        rows.len(),
        |ui, range| {
            for row in &rows[range] {
                ui.horizontal(|ui| {
                    ui.colored_label(row.severity.color(ui.visuals()), row.severity.icon());
                    // e.g. `0_master  #0106:12  Unknown field` (the behavior only for the whole work)
                    let name = behaviors.and_then(|behaviors| behaviors.get(row.behavior)).map(|b| b.name.as_str());
                    let text = match name {
                        Some(name) => format!("{name}  {}:{}  {}", row.id, row.line + 1, row.message),
                        None => format!("{}:{}  {}", row.id, row.line + 1, row.message),
                    };
                    if ui.add(egui::Button::selectable(false, egui::RichText::new(text).monospace())).clicked() {
                        picked = Some((row.behavior, row.id.clone(), row.line));
                    }
                });
            }
        },
    );
    picked
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::patch::workspace::ADSF;

    fn clip_data() -> Behavior {
        let mut adsf = Behavior::open(&Path::new("meshes").join(format!("{ADSF}.bin"))).unwrap();
        let mut add = |id: &str, text: &str| adsf.added.insert(id.to_owned(), text.to_owned());
        add("Male~1/Bad~m$0.txt", "Bad\nm$0\nnot a speed\n0\n0\n0\n");
        add("Male~1/Good~m$1.txt", "Good\nm$1\n1\n0\n0\n0\n");
        add("Male~1/m$2.txt", "m$2\nnot a duration\n0\n0\n");
        add("Female~1/Bad~m$0.txt", "Bad\nm$0\nnot a speed\n0\n0\n0\n");
        adsf
    }

    /// The sorted rows of `behavior` as the only one of a work, of mod code `m`.
    fn rows_of(behavior: &Behavior, scope: &CheckScope, mode: ParserMode) -> Vec<CheckRow> {
        let mut rows = Vec::new();
        run(Checker { index: 0, behavior, code: "m", mode }, scope, &mut rows);
        sort_rows(&mut rows);
        rows
    }

    /// What is read is checked at once, for the status bar; then an edit only checks its object again.
    #[test]
    fn the_work_is_checked_when_read_and_then_per_edit() {
        let mut tool = PatchTool::default();
        tool.workspace.behaviors.push(clip_data());
        tool.mod_code = "m".to_owned();
        tool.parser_mode = ParserMode::Strict;
        tool.loaded_for = Some("m".to_owned());
        assert_eq!(tool.diagnostic_counts(), None, "not checked yet");

        tool.reset_names(); // as a read or a load does
        tool.diagnose_work();
        assert_eq!(tool.diagnostic_counts(), Some((3, 0)));

        let motion = "Male~1/m$2.txt";
        tool.workspace.behaviors[0].added.insert(motion.to_owned(), "m$2\n1\n0\n0\n".to_owned());
        tool.refresh_work_check(motion);
        assert_eq!(tool.diagnostic_counts(), Some((2, 0)));
        assert!(tool.work_check.is_none(), "the window stays closed");
        tool.show_diagnostics();
        assert_eq!(tool.work_check.as_ref().map(|check| check.rows.len()), Some(2));
    }

    /// Every behavior is checked; a correction in one leaves the rows of the others.
    #[test]
    fn the_whole_work_is_checked() {
        let mut tool = PatchTool::default();
        tool.workspace.behaviors.extend([clip_data(), clip_data()]);
        tool.mod_code = "m".to_owned();
        tool.parser_mode = ParserMode::Strict;
        tool.start_whole_work_check();
        let rows = |tool: &PatchTool| {
            tool.work_check.as_ref().unwrap().rows.iter().map(|row| row.behavior).collect::<Vec<_>>()
        };
        assert_eq!(rows(&tool), [0, 0, 0, 1, 1, 1]);

        tool.active = 1;
        let motion = "Male~1/m$2.txt";
        tool.workspace.behaviors[1].added.remove(motion);
        tool.refresh_work_check(motion);
        assert_eq!(rows(&tool), [0, 0, 0, 1, 1]);
    }

    fn found(scope: &CheckScope) -> Vec<String> {
        rows_of(&clip_data(), scope, ParserMode::Strict).into_iter().map(|row| row.id).collect()
    }

    #[test]
    fn it_goes_through_what_was_asked_for() {
        let bad_male = "Male~1/Bad~m$0.txt".to_owned();
        let motion = "Male~1/m$2.txt".to_owned();
        let bad_female = "Female~1/Bad~m$0.txt".to_owned();

        assert_eq!(found(&CheckScope::All), [bad_female, bad_male.clone(), motion.clone()]);
        assert_eq!(found(&CheckScope::Folder("Male~1".to_owned())), [bad_male.clone(), motion.clone()]);
        let kind = |kind| CheckScope::Kind { folder: "Male~1".to_owned(), kind };
        assert_eq!(found(&kind(FileKind::Clip)), [bad_male]);
        assert_eq!(found(&kind(FileKind::Motion)), [motion]);
        assert!(found(&CheckScope::Folder("None~1".to_owned())).is_empty());
    }

    /// The count is the parser's to check, in the mode of the patch page.
    #[test]
    fn the_mode_of_the_patch_page_decides() {
        let mut adsf = clip_data();
        adsf.added.clear();
        adsf.added.insert("Male~1/Count~m$0.txt".to_owned(), "Count\nm$0\n1\n0\n0\n7\n".to_owned());
        assert_eq!(rows_of(&adsf, &CheckScope::All, ParserMode::Strict).len(), 1);
        assert!(rows_of(&adsf, &CheckScope::All, ParserMode::Lenient).is_empty());
    }

    #[test]
    fn a_correction_leaves_the_list_and_a_new_mistake_comes_in() {
        let mut tool = PatchTool::default();
        tool.workspace.behaviors.push(clip_data());
        tool.mod_code = "m".to_owned();
        tool.parser_mode = ParserMode::Strict;
        tool.start_work_check(CheckScope::Folder("Male~1".to_owned()));
        let rows = |tool: &PatchTool| tool.work_check.as_ref().unwrap().rows.len();
        assert_eq!(rows(&tool), 2);

        let motion = "Male~1/m$2.txt";
        tool.workspace.behaviors[0].added.insert(motion.to_owned(), "m$2\n1\n0\n0\n".to_owned());
        tool.refresh_work_check(motion);
        assert_eq!(rows(&tool), 1);

        tool.workspace.behaviors[0].added.insert(motion.to_owned(), "m$2\nwrong\n0\n0\n".to_owned());
        tool.refresh_work_check(motion);
        assert_eq!(rows(&tool), 2);

        tool.workspace.behaviors[0].added.remove(motion);
        tool.refresh_work_check(motion);
        assert_eq!(rows(&tool), 1, "deleted");

        // Out of what was asked for: it stays out.
        let other = "Female~1/Bad~m$0.txt";
        tool.refresh_work_check(other);
        assert_eq!(rows(&tool), 1);
    }

    /// A new animation set the parser rejects has to say why.
    #[test]
    fn an_animation_set_has_the_parser_on_it() {
        // (The clip data's needs no file, and nothing here reads the sets of the template.)
        let mut asdsf = Behavior::open(&Path::new("meshes").join(format!("{ADSF}.bin"))).unwrap();
        (asdsf.adsf, asdsf.asdsf) = (false, true);
        let id = "DefaultMaleData~DefaultMale/m$Start.txt";
        asdsf.added.insert(id.to_owned(), "V3\nabc\ndef\n".to_owned());

        let rows = rows_of(&asdsf, &CheckScope::All, ParserMode::Strict);
        let row = rows.iter().find(|row| row.message.contains("triggers_len")).expect("the parser's reason");
        assert_eq!((row.id.as_str(), row.line, row.severity), (id, 1, Severity::Error));
    }

    /// A Nemesis patch: the fields of what the mod changes, and the parser on its patch.
    #[test]
    fn a_behavior_has_what_it_changes_checked() {
        let dir = std::env::temp_dir().join(format!("d_merge_check_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let template = dir.join("x.xml");
        let object = "\t\t<hkobject name=\"#0100\" class=\"hkbModifierList\" signature=\"0xa4180ca1\">\n\t\t\t<hkparam name=\"userData\">0</hkparam>\n\t\t</hkobject>\n";
        std::fs::write(&template, format!("<hkpackfile>\n{object}{}</hkpackfile>\n", object.replace("#0100", "#0101")))
            .unwrap();
        let mut behavior = Behavior::open(&template).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();

        assert!(rows_of(&behavior, &CheckScope::All, ParserMode::Strict).is_empty(), "nothing changed");
        let typo = object.replace("userData", "typoField");
        behavior.set_text("#0100", &typo);
        let rows = rows_of(&behavior, &CheckScope::All, ParserMode::Strict);
        assert!(rows.iter().any(|row| row.id == "#0100" && row.line == 1), "{rows:?}");
        assert!(rows.iter().all(|row| row.id != "#0101"), "{rows:?}");
    }
}
