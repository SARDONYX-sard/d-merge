//! The problems of the whole work, as VS Code's: found when the work is read, then kept up to date one
//! object at a time. Their counts are in the status bar, and a click on them shows the Problems panel
//! ([`super::ui::problems`]).
//!
//! ```text
//! read / loaded / restored  -> every changed object is checked (in parallel)
//! an edit, after a pause    -> only that object is checked again
//! ```
//!
//! | Behavior  | Checked             | What                                                |
//! | --------- | ------------------- | --------------------------------------------------- |
//! | anim data | every file          | d-merge's parser on the patch                       |
//! | others    | what the mod changes | fields, pointers, and d-merge's parser on the patch |
//!
//! A template object nobody changed has nothing of the mod's to find, so only changes are checked.

use egui_code_editor::Diagnostic;
use gui_shared::settings::pages::patch::behavior::ParserMode;
use rayon::prelude::*;

use super::{
    editor::{diagnostics, preview::Preview},
    tool::work_state::Work,
    workspace::{Behavior, ObjectText},
};

/// The problems of the work, and how they are shown.
#[derive(Debug, Default)]
pub(super) struct Problems {
    /// `None` until the work was checked.
    pub(super) list: Option<Vec<ObjectProblems>>,
    /// The [`Work::reloads`] `list` was found of: the work changed as a whole since (read, loaded,
    /// restored) if it is another, and `list` is found again.
    checked: Option<u64>,
    /// The Problems panel shows: a click on the status bar's counts.
    pub(super) open: bool,
    /// The keyboard in the panel: `Ctrl+Shift+L` moves it there and back.
    pub(super) cursor: egui_code_editor::ProblemsCursor,
}

/// The problems of one object: a group of the Problems panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ObjectProblems {
    /// In `Workspace::behaviors`.
    pub(super) behavior: usize,
    /// e.g. `#0106`
    pub(super) id: String,
    /// Never empty. Their lines are the object's.
    pub(super) problems: Vec<Diagnostic>,
}

/// What is checked and how: the same for every object of a behavior.
#[derive(Debug, Clone, Copy)]
struct Checker<'a> {
    /// `index` in `Workspace::behaviors`
    index: usize,
    behavior: &'a Behavior,
    code: &'a str,
    /// The patch page's, which the parser checks by.
    mode: ParserMode,
}

/// Is this object checked? See the module docs.
fn is_checked(behavior: &Behavior, object: &ObjectText<'_>) -> bool {
    !behavior.is_ref(object.id) && (behavior.is_anim_data() || object.is_new || object.is_edited)
}

/// The problems of one object. `None`: it has none.
fn object_problems(checker: Checker<'_>, object: &ObjectText<'_>) -> Option<ObjectProblems> {
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
    (!found.is_empty()).then(|| ObjectProblems { behavior: index, id: id.to_owned(), problems: found })
}

/// The problems of every checked object of a behavior, in parallel: a work has thousands, and the
/// parser reads each one's patch.
fn behavior_problems(checker: Checker<'_>) -> Vec<ObjectProblems> {
    let behavior = checker.behavior;
    let objects: Vec<ObjectText<'_>> = behavior.texts().filter(|object| is_checked(behavior, object)).collect();
    objects.par_iter().filter_map(|object| object_problems(checker, object)).collect()
}

/// By behavior, then object.
fn sort(groups: &mut [ObjectProblems]) {
    groups.sort_by(|a, b| (a.behavior, &a.id).cmp(&(b.behavior, &b.id)));
}

impl Problems {
    /// Checks the whole work again if it changed as a whole (read, loaded, restored). `(code, mode)`:
    /// the mod code, and the parser's mode.
    fn check_work(&mut self, behaviors: &[Behavior], (code, mode): (&str, ParserMode)) {
        let checker = |index| Checker { index, behavior: &behaviors[index], code, mode };
        let mut found: Vec<ObjectProblems> =
            (0..behaviors.len()).flat_map(|index| behavior_problems(checker(index))).collect();
        sort(&mut found);
        self.list = Some(found);
    }

    /// The object `id` of behavior `index` changed: only its problems are found again. After typing
    /// pauses.
    fn refresh_object(&mut self, (index, behavior): (usize, &Behavior), id: &str, (code, mode): (&str, ParserMode)) {
        let checker = Checker { index, behavior, code, mode };
        let object = behavior.object_text(id).filter(|object| is_checked(behavior, object));
        let found = object.and_then(|object| object_problems(checker, &object));

        let Some(groups) = &mut self.list else { return };
        groups.retain(|group| group.behavior != index || group.id != id);
        if let Some(found) = found {
            groups.push(found);
            sort(groups);
        }
    }

    /// The errors and warnings of the work, once it was checked: the status bar's counts.
    pub(super) fn counts(&self) -> Option<(usize, usize)> {
        let groups = self.list.as_deref()?;
        Some(egui_code_editor::problems::counts(groups.iter().flat_map(|group| &group.problems)))
    }
}

impl Problems {
    /// Checks `work` again if it changed as a whole, unless a job has it or none is open. Call each
    /// frame.
    pub(super) fn diagnose(&mut self, work: &Work) {
        if work.job().is_some() || !work.place.is_open() || self.checked == Some(work.reloads()) {
            return;
        }
        self.check_work(&work.workspace.behaviors, (work.place.mod_code.trim(), work.parser_mode));
        self.checked = Some(work.reloads());
    }

    /// The object `id` of behavior `active` changed: see [`Self::refresh_object`].
    pub(super) fn refresh_in(&mut self, work: &Work, active: usize, id: &str) {
        let Some(behavior) = work.workspace.behaviors.get(active) else { return };
        let settings = (work.place.mod_code.trim(), work.parser_mode);
        self.refresh_object((active, behavior), id, settings);
    }
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

    fn work(mode: ParserMode) -> Work {
        let mut work = Work::default();
        work.workspace.behaviors.push(clip_data());
        work.place.mod_code = "m".to_owned();
        work.parser_mode = mode;
        work.place.loaded_for = Some("m".to_owned());
        work
    }

    /// What is read is checked at once; then an edit only checks its object again.
    #[test]
    fn the_work_is_checked_when_read_and_then_per_edit() {
        let (mut work, mut problems) = (work(ParserMode::Strict), Problems::default());
        assert_eq!(problems.counts(), None, "not checked yet");
        work.reset_names(); // as a read or a load does
        problems.diagnose(&work);
        assert_eq!(problems.counts(), Some((3, 0)));
        let ids: Vec<&str> = problems.list.iter().flatten().map(|group| group.id.as_str()).collect();
        assert_eq!(ids, ["Female~1/Bad~m$0.txt", "Male~1/Bad~m$0.txt", "Male~1/m$2.txt"]);

        let motion = "Male~1/m$2.txt";
        work.workspace.behaviors[0].added.insert(motion.to_owned(), "m$2\n1\n0\n0\n".to_owned());
        problems.refresh_in(&work, 0, motion);
        assert_eq!(problems.counts(), Some((2, 0)));
        work.workspace.behaviors[0].added.insert(motion.to_owned(), "m$2\nwrong\n0\n0\n".to_owned());
        problems.refresh_in(&work, 0, motion);
        assert_eq!(problems.counts(), Some((3, 0)), "a new mistake comes in");
    }

    /// The count is the parser's to check, in the mode of the patch page.
    #[test]
    fn the_mode_of_the_patch_page_decides() {
        let count = |mode| {
            let (mut work, mut problems) = (work(mode), Problems::default());
            work.workspace.behaviors[0].added.clear();
            work.workspace.behaviors[0]
                .added
                .insert("Male~1/Count~m$0.txt".to_owned(), "Count\nm$0\n1\n0\n0\n7\n".to_owned());
            work.reset_names();
            problems.diagnose(&work);
            problems.counts()
        };
        assert_eq!(count(ParserMode::Strict), Some((1, 0)));
        assert_eq!(count(ParserMode::Lenient), Some((0, 0)));
    }

    /// A Nemesis patch: the fields of what the mod changes, and the parser on its patch.
    #[test]
    fn a_behavior_has_what_it_changes_checked() {
        let dir = std::env::temp_dir().join(format!("d_merge_problems_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let template = dir.join("x.xml");
        let object = "\t\t<hkobject name=\"#0100\" class=\"hkbModifierList\" signature=\"0xa4180ca1\">\n\t\t\t<hkparam name=\"userData\">0</hkparam>\n\t\t</hkobject>\n";
        std::fs::write(&template, format!("<hkpackfile>\n{object}{}</hkpackfile>\n", object.replace("#0100", "#0101")))
            .unwrap();
        let mut behavior = Behavior::open(&template).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        let checker = |behavior: &Behavior| -> Vec<ObjectProblems> {
            behavior_problems(Checker { index: 0, behavior, code: "m", mode: ParserMode::Strict })
        };

        assert!(checker(&behavior).is_empty(), "nothing changed");
        behavior.set_text("#0100", &object.replace("userData", "typoField"));
        let problems = checker(&behavior);
        let lines = |id: &str| {
            problems
                .iter()
                .filter(|group| group.id == id)
                .flat_map(|group| &group.problems)
                .map(|problem| problem.line)
                .collect::<Vec<_>>()
        };
        assert!(lines("#0100").contains(&1), "{problems:?}");
        assert!(lines("#0101").is_empty(), "{problems:?}");
    }
}
