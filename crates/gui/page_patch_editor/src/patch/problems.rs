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
//! The parser checks in `Strict` mode, whatever the patch page's: see `Behavior::validate`.
//!
//! A new object nothing links to is warned of too ([`diagnostics::not_linked`]). That depends on the
//! other objects: an edit of `#0106` that adds `#mymod$0` links it. So an edit also checks again the
//! objects that were or are not linked.

use egui_code_editor::Diagnostic;
use fast_hash::HashSet;
use rayon::prelude::*;

use super::{
    editor::{diagnostics, preview::Preview},
    tool::work_state::Work,
    tree::Graph,
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
    /// The ids of the new objects nothing links to. See [`Graph::unlinked`].
    not_linked: &'a HashSet<&'a str>,
}

/// The ids of `graph`'s new objects nothing links to.
fn not_linked_ids(graph: &Graph) -> HashSet<&str> {
    graph.unlinked().iter().map(|&node| graph.id(node)).collect()
}

/// Is this object checked? See the module docs.
fn is_checked(behavior: &Behavior, object: &ObjectText<'_>) -> bool {
    !behavior.is_ref(object.id) && (behavior.is_anim_data() || object.is_new || object.is_edited)
}

/// The problems of one object. `None`: it has none.
fn object_problems(checker: Checker<'_>, object: &ObjectText<'_>) -> Option<ObjectProblems> {
    let Checker { index, behavior, code, not_linked } = checker;
    let (id, text) = (object.id, object.text);
    // Animation data only has the parser's word: see `anim_lines`.
    let mut found: Vec<Diagnostic> = if behavior.is_anim_data() {
        Vec::new()
    } else {
        diagnostics::field_diagnostics(text, behavior, object.is_new, code)
    };
    if not_linked.contains(id) {
        found.insert(0, diagnostics::not_linked(text));
    }
    // As the editor does: the parser reads the patch, whose lines are not the object's.
    if let Some(patch) = behavior.patch(code, id)
        && let Err(error) = behavior.validate(id, &patch)
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
    /// Checks the whole work again if it changed as a whole (read, loaded, restored). `code`: the mod
    /// code.
    fn check_work(&mut self, behaviors: &[Behavior], code: &str) {
        let mut found = Vec::new();
        for (index, behavior) in behaviors.iter().enumerate() {
            // Only new objects can be not linked: no graph for a behavior without any.
            let has_new = !behavior.is_anim_data() && !behavior.added.is_empty();
            let graph = has_new.then(|| Graph::new(behavior));
            let not_linked = graph.as_ref().map(not_linked_ids).unwrap_or_default();

            let checker = Checker { index, behavior, code, not_linked: &not_linked };
            found.extend(behavior_problems(checker));
        }
        sort(&mut found);
        self.list = Some(found);
    }

    /// The objects `ids` of behavior `index` changed: only their problems are found again. After
    /// typing pauses.
    fn refresh_objects(&mut self, checker: Checker<'_>, ids: &[&str]) {
        let Some(groups) = &mut self.list else { return };
        let (index, behavior) = (checker.index, checker.behavior);
        for &id in ids {
            let object = behavior.object_text(id).filter(|object| is_checked(behavior, object));
            let found = object.and_then(|object| object_problems(checker, &object));
            groups.retain(|group| group.behavior != index || group.id != id);
            groups.extend(found);
        }
        sort(groups);
    }

    /// The ids of behavior `index` the list warns of as not linked.
    fn warned_not_linked(&self, index: usize) -> Vec<String> {
        let groups = self.list.as_deref().unwrap_or_default();
        let warned = groups.iter().filter(|group| group.behavior == index);
        let warned = warned.filter(|group| group.problems.iter().any(diagnostics::is_not_linked));
        warned.map(|group| group.id.clone()).collect()
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
        self.check_work(&work.workspace.behaviors, work.place.mod_code.trim());
        self.checked = Some(work.reloads());
    }

    /// The object `id` of behavior `active` changed: it is checked again, and so are the objects that
    /// were or are not linked (see the module docs). See [`Self::refresh_objects`].
    pub(super) fn refresh_in(&mut self, work: &mut Work, active: usize, id: &str) {
        if active >= work.workspace.behaviors.len() {
            return;
        }
        let names = work.names(active);
        let not_linked = not_linked_ids(&names.graph);

        // (Owned: the list they are read from is changed after.)
        let was_warned = self.warned_not_linked(active);
        let mut ids: Vec<&str> = std::iter::once(id)
            .chain(not_linked.iter().copied())
            .chain(was_warned.iter().map(String::as_str))
            .collect();
        ids.sort_unstable();
        ids.dedup();

        let behavior = &work.workspace.behaviors[active];
        let code = work.place.mod_code.trim();
        let checker = Checker { index: active, behavior, code, not_linked: &not_linked };
        self.refresh_objects(checker, &ids);
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

    fn work() -> Work {
        let mut work = Work::default();
        work.workspace.behaviors.push(clip_data());
        work.place.mod_code = "m".to_owned();
        work.place.loaded_for = Some("m".to_owned());
        work
    }

    /// What is read is checked at once; then an edit only checks its object again.
    #[test]
    fn the_work_is_checked_when_read_and_then_per_edit() {
        let (mut work, mut problems) = (work(), Problems::default());
        assert_eq!(problems.counts(), None, "not checked yet");
        work.reset_names(); // as a read or a load does
        problems.diagnose(&work);
        assert_eq!(problems.counts(), Some((3, 0)));
        let ids: Vec<&str> = problems.list.iter().flatten().map(|group| group.id.as_str()).collect();
        assert_eq!(ids, ["Female~1/Bad~m$0.txt", "Male~1/Bad~m$0.txt", "Male~1/m$2.txt"]);

        let motion = "Male~1/m$2.txt";
        work.workspace.behaviors[0].added.insert(motion.to_owned(), "m$2\n1\n0\n0\n".to_owned());
        problems.refresh_in(&mut work, 0, motion);
        assert_eq!(problems.counts(), Some((2, 0)));
        work.workspace.behaviors[0].added.insert(motion.to_owned(), "m$2\nwrong\n0\n0\n".to_owned());
        problems.refresh_in(&mut work, 0, motion);
        assert_eq!(problems.counts(), Some((3, 0)), "a new mistake comes in");
    }

    /// Always `Strict`: a wrong count (`7` triggers, none listed) is an error, though the patch page
    /// may be `Lenient`.
    #[test]
    fn the_parser_is_strict() {
        let (mut work, mut problems) = (work(), Problems::default());
        work.workspace.behaviors[0].added.clear();
        work.workspace.behaviors[0]
            .added
            .insert("Male~1/Count~m$0.txt".to_owned(), "Count\nm$0\n1\n0\n0\n7\n".to_owned());
        work.reset_names();
        problems.diagnose(&work);
        assert_eq!(problems.counts(), Some((1, 0)));
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
            let not_linked = HashSet::default();
            behavior_problems(Checker { index: 0, behavior, code: "m", not_linked: &not_linked })
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

    /// A new object nothing links to is warned of; an edit of another object that links it takes the
    /// warning away.
    #[test]
    fn a_new_object_is_warned_of_until_something_links_it() {
        let dir = std::env::temp_dir().join(format!("d_merge_not_linked_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let template = dir.join("x.xml");
        let root = "\t\t<hkobject name=\"#0100\" class=\"hkbModifierList\" signature=\"0xa4180ca1\">\n\t\t\t<hkparam name=\"userData\">0</hkparam>\n\t\t</hkobject>\n";
        std::fs::write(&template, format!("<hkpackfile>\n{root}</hkpackfile>\n")).unwrap();
        let mut behavior = Behavior::open(&template).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        behavior.added.insert("#m$0".to_owned(), root.replace("#0100", "#m$0"));

        let mut work = Work::default();
        work.workspace.behaviors.push(behavior);
        work.place.mod_code = "m".to_owned();
        work.place.loaded_for = Some("m".to_owned());
        work.reset_names();
        let mut problems = Problems::default();
        let is_warned = |problems: &Problems| {
            let groups = problems.list.iter().flatten().filter(|group| group.id == "#m$0");
            groups.flat_map(|group| &group.problems).any(diagnostics::is_not_linked)
        };

        problems.diagnose(&work);
        assert!(is_warned(&problems), "{:?}", problems.list);

        work.workspace.behaviors[0].set_text("#0100", &root.replace(">0<", ">#m$0<"));
        work.object_changed("#0100");
        problems.refresh_in(&mut work, 0, "#0100");
        assert!(!is_warned(&problems), "{:?}", problems.list);
    }
}
