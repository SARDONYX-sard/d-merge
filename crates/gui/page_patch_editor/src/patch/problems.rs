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

use egui_code_editor::{Diagnostic, Severity};
use gui_shared::settings::pages::patch::behavior::ParserMode;
use rayon::prelude::*;

use super::{
    PatchTool,
    editor::{diagnostics, preview::Preview},
    workspace::{Behavior, ObjectText},
};

/// One problem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Problem {
    /// In `Workspace::behaviors`.
    pub(super) behavior: usize,
    /// e.g. `#0106`
    pub(super) id: String,
    /// 0-based line of the object.
    pub(super) line: usize,
    pub(super) message: String,
    pub(super) severity: Severity,
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

/// The problems of one object.
fn object_problems(checker: Checker<'_>, object: &ObjectText<'_>) -> Vec<Problem> {
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
    let to_problem = |problem: Diagnostic| Problem {
        behavior: index,
        id: id.to_owned(),
        line: problem.line,
        message: problem.message,
        severity: problem.severity,
    };
    found.into_iter().map(to_problem).collect()
}

/// The problems of every checked object of a behavior, in parallel: a work has thousands, and the
/// parser reads each one's patch.
fn behavior_problems(checker: Checker<'_>) -> Vec<Problem> {
    let behavior = checker.behavior;
    let objects: Vec<ObjectText<'_>> = behavior.texts().filter(|object| is_checked(behavior, object)).collect();
    objects.par_iter().flat_map_iter(|object| object_problems(checker, object)).collect()
}

/// By behavior, object, line.
fn sort(problems: &mut [Problem]) {
    problems.sort_by(|a, b| (a.behavior, &a.id, a.line).cmp(&(b.behavior, &b.id, b.line)));
}

/// `(errors, warnings)` of `problems`.
pub(super) fn counts(problems: &[Problem]) -> (usize, usize) {
    let errors = problems.iter().filter(|problem| problem.severity == Severity::Error).count();
    (errors, problems.len() - errors)
}

impl PatchTool {
    /// What checks the objects of behavior `index`.
    fn checker(&self, index: usize) -> Checker<'_> {
        let behavior = &self.workspace.behaviors[index];
        Checker { index, behavior, code: self.mod_code.trim(), mode: self.parser_mode }
    }

    /// Checks the whole work again if it changed as a whole (read, loaded, restored). Call each frame.
    pub(super) fn diagnose_work(&mut self) {
        if !self.problems_stale || self.job.is_some() || self.loaded_for.is_none() {
            return;
        }
        let mut found: Vec<Problem> =
            (0..self.workspace.behaviors.len()).flat_map(|index| behavior_problems(self.checker(index))).collect();
        sort(&mut found);
        self.problems = Some(found);
        self.problems_stale = false;
    }

    /// The errors and warnings of the work, once it was checked: the status bar's counts.
    pub(super) fn problem_counts(&self) -> Option<(usize, usize)> {
        self.problems.as_deref().map(counts)
    }

    /// The object `id` of the active behavior changed: only its problems are found again. After
    /// typing pauses.
    pub(super) fn refresh_problems(&mut self, id: &str) {
        let index = self.active;
        let Some(behavior) = self.workspace.behaviors.get(index) else { return };
        let object = behavior.object_text(id).filter(|object| is_checked(behavior, object));
        let found = object.map(|object| object_problems(self.checker(index), &object)).unwrap_or_default();
        let Some(problems) = &mut self.problems else { return };
        problems.retain(|problem| problem.behavior != index || problem.id != id);
        if !found.is_empty() {
            problems.extend(found);
            sort(problems);
        }
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

    fn tool(mode: ParserMode) -> PatchTool {
        let mut tool = PatchTool::default();
        tool.workspace.behaviors.push(clip_data());
        tool.mod_code = "m".to_owned();
        tool.parser_mode = mode;
        tool.loaded_for = Some("m".to_owned());
        tool
    }

    /// What is read is checked at once; then an edit only checks its object again.
    #[test]
    fn the_work_is_checked_when_read_and_then_per_edit() {
        let mut tool = tool(ParserMode::Strict);
        assert_eq!(tool.problem_counts(), None, "not checked yet");
        tool.reset_names(); // as a read or a load does
        tool.diagnose_work();
        assert_eq!(tool.problem_counts(), Some((3, 0)));
        let ids: Vec<&str> = tool.problems.iter().flatten().map(|problem| problem.id.as_str()).collect();
        assert_eq!(ids, ["Female~1/Bad~m$0.txt", "Male~1/Bad~m$0.txt", "Male~1/m$2.txt"]);

        let motion = "Male~1/m$2.txt";
        tool.workspace.behaviors[0].added.insert(motion.to_owned(), "m$2\n1\n0\n0\n".to_owned());
        tool.refresh_problems(motion);
        assert_eq!(tool.problem_counts(), Some((2, 0)));
        tool.workspace.behaviors[0].added.insert(motion.to_owned(), "m$2\nwrong\n0\n0\n".to_owned());
        tool.refresh_problems(motion);
        assert_eq!(tool.problem_counts(), Some((3, 0)), "a new mistake comes in");
    }

    /// The count is the parser's to check, in the mode of the patch page.
    #[test]
    fn the_mode_of_the_patch_page_decides() {
        let count = |mode| {
            let mut tool = tool(mode);
            tool.workspace.behaviors[0].added.clear();
            tool.workspace.behaviors[0]
                .added
                .insert("Male~1/Count~m$0.txt".to_owned(), "Count\nm$0\n1\n0\n0\n7\n".to_owned());
            tool.reset_names();
            tool.diagnose_work();
            tool.problem_counts()
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
        let checker = |behavior: &Behavior| -> Vec<Problem> {
            behavior_problems(Checker { index: 0, behavior, code: "m", mode: ParserMode::Strict })
        };

        assert!(checker(&behavior).is_empty(), "nothing changed");
        behavior.set_text("#0100", &object.replace("userData", "typoField"));
        let problems = checker(&behavior);
        assert!(problems.iter().any(|problem| problem.id == "#0100" && problem.line == 1), "{problems:?}");
        assert!(problems.iter().all(|problem| problem.id != "#0101"), "{problems:?}");
    }
}
