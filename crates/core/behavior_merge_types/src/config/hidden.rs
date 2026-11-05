//! Operations of a mod that the merge leaves out, like a file hidden with MO2's `.mohidden`.
//!
//! The mod's files stay as they are. An operation is named as a conflict check names it:
//!
//! | `target`                                     | `path`                           | Left out                  |
//! | -------------------------------------------- | -------------------------------- | ------------------------- |
//! | `meshes/actors/.../0_master.bin`             | `#0106/hkbModifierList/enable`   | that field's change       |
//! | `meshes/actors/.../0_master.bin`             | `#0106/hkbModifierList/items/[1:3]` | what it writes to these elements |
//! | `meshes/animationdatasinglefile.bin`         | `DefaultMale~1/Idle~348/..`      | the mod's whole file      |
//! | `meshes/animationsetdatasinglefile.bin`      | `DData~D/_MTSolo.txt/..`         | the mod's whole file      |
//!
//! Animation data goes by the file: its lines are positions, so a field of it cannot go alone.
//! What adds or pushes is never hidden: it conflicts with nothing.

use std::{collections::BTreeSet, ops::Range};

use fast_hash::{HashMap, HashSet};
use json_patch::{JsonPatch, json_path::JsonPath};

/// One hidden operation.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct HiddenPatch {
    /// e.g. `para`: the folder in `Nemesis_Engine/mod/`
    pub mod_code: String,
    /// e.g. `meshes/actors/character/behaviors/0_master.bin`
    pub target: String,
    /// See the module docs.
    pub path: String,
}

/// What is hidden of one mod in one template.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HiddenRules {
    /// Fields, array ranges as written, and the files of animation data (`DefaultMale~1/Idle~348`).
    paths: HashSet<String>,
    /// By the array: `#0106/hkbModifierList/items` -> `[1..3]`
    ranges: HashMap<String, Vec<Range<usize>>>,
}

impl HiddenRules {
    fn add(&mut self, path: &str) {
        self.paths.insert(path.to_owned());
        if let Some((array, range)) = array_range(path) {
            self.ranges.entry(array.to_owned()).or_default().push(range);
        }
    }

    /// Is the file of animation data of `project` and `name` hidden?
    ///
    /// e.g. `DefaultMale~1` and `Idle~348`; `DefaultMaleData~DefaultMale` and `_MTSolo.txt`
    #[must_use]
    pub fn hides_file(&self, project: &str, name: &str) -> bool {
        self.paths.contains(&format!("{project}/{name}"))
    }

    /// What is left of a patch of a behavior.
    #[must_use]
    pub fn apply<'a>(&self, path: &JsonPath<'a>, patch: JsonPatch<'a>) -> Vec<JsonPatch<'a>> {
        let joined = path.join("/");
        if self.paths.contains(&joined) {
            return Vec::new();
        }
        match self.ranges.get(&joined) {
            Some(hidden) => patch.without_ranges(hidden),
            None => vec![patch],
        }
    }
}

/// `#1/C/items/[1:3]` -> (`#1/C/items`, `1..3`)
fn array_range(path: &str) -> Option<(&str, Range<usize>)> {
    let (array, range) = path.rsplit_once("/[")?;
    let (start, end) = range.strip_suffix(']')?.split_once(':')?;
    Some((array, start.parse().ok()?..end.parse().ok()?))
}

/// The place an operation goes by: animation data goes by the file. See the module docs.
fn place_of<'a>(target: &str, path: &'a str) -> &'a str {
    let is_anim_data = ["animationdatasinglefile", "animationsetdatasinglefile"]
        .iter()
        .any(|stem| target.rsplit('/').next().is_some_and(|file| file.to_ascii_lowercase().starts_with(stem)));
    if !is_anim_data {
        return path;
    }
    path.match_indices('/').nth(1).map_or(path, |(at, _)| &path[..at])
}

fn rules_key(mod_code: &str, target: &str) -> String {
    format!("{mod_code}/{target}").replace('\\', "/").to_ascii_lowercase()
}

/// All that is hidden. See the module docs. Empty, what a merge is usually given, costs nothing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HiddenPatches {
    list: BTreeSet<HiddenPatch>,
    rules: HashMap<String, HiddenRules>,
}

impl HiddenPatches {
    #[must_use]
    pub fn new(list: impl IntoIterator<Item = HiddenPatch>) -> Self {
        let mut hidden = Self { list: list.into_iter().collect(), rules: HashMap::default() };
        hidden.build_rules();
        hidden
    }

    fn build_rules(&mut self) {
        self.rules.clear();
        for patch in &self.list {
            let rules = self.rules.entry(rules_key(&patch.mod_code, &patch.target)).or_default();
            rules.add(place_of(&patch.target, &patch.path));
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    /// What goes in the file it is kept in.
    pub fn iter(&self) -> impl Iterator<Item = &HiddenPatch> {
        self.list.iter()
    }

    /// What is hidden of `mod_code` in `target`, if anything.
    #[must_use]
    pub fn rules_of(&self, mod_code: &str, target: &str) -> Option<&HiddenRules> {
        if self.is_empty() {
            return None;
        }
        self.rules.get(&rules_key(mod_code, target))
    }

    /// Is this operation hidden? `path`: as a conflict names it. Of animation data, any field of a
    /// hidden file is.
    #[must_use]
    pub fn contains(&self, mod_code: &str, target: &str, path: &str) -> bool {
        self.rules_of(mod_code, target).is_some_and(|rules| rules.paths.contains(place_of(target, path)))
    }

    /// Hides the operation, or shows it again if it is hidden.
    pub fn toggle(&mut self, mod_code: &str, target: &str, path: &str) {
        let place = place_of(target, path);
        let same = |patch: &HiddenPatch| {
            rules_key(&patch.mod_code, &patch.target) == rules_key(mod_code, target)
                && place_of(&patch.target, &patch.path) == place
        };
        if self.contains(mod_code, target, path) {
            self.list.retain(|patch| !same(patch));
        } else {
            self.list.insert(HiddenPatch {
                mod_code: mod_code.to_owned(),
                target: target.to_owned(),
                path: place.to_owned(),
            });
        }
        self.build_rules();
    }
}

#[cfg(test)]
mod tests {
    use json_patch::{Action, Op, json_path};

    use super::*;

    const MASTER: &str = "meshes/actors/character/behaviors/0_master.bin";
    const ADSF: &str = "meshes/animationdatasinglefile.bin";

    fn pure() -> JsonPatch<'static> {
        JsonPatch { action: Action::Pure { op: Op::Replace }, value: "x".into() }
    }

    #[test]
    fn a_field_is_hidden_for_its_mod_and_template_only() {
        let mut hidden = HiddenPatches::default();
        assert!(hidden.rules_of("para", MASTER).is_none());

        hidden.toggle("para", MASTER, "#1/C/a");
        assert!(hidden.contains("para", MASTER, "#1/C/a"));
        assert!(hidden.contains("PARA", MASTER, "#1/C/a"), "a folder name has no case on Windows");
        assert!(!hidden.contains("other", MASTER, "#1/C/a"));
        assert!(!hidden.contains("para", "meshes/x.bin", "#1/C/a"));
        assert!(!hidden.contains("para", MASTER, "#1/C/b"));

        let rules = hidden.rules_of("para", MASTER).unwrap();
        assert_eq!(rules.apply(&json_path!["#1", "C", "a"], pure()), []);
        assert_eq!(rules.apply(&json_path!["#1", "C", "b"], pure()), [pure()]);

        hidden.toggle("para", MASTER, "#1/C/a");
        assert!(hidden.is_empty());
        assert!(hidden.rules_of("para", MASTER).is_none());
    }

    #[test]
    fn a_range_of_an_array_leaves_the_rest() {
        let hidden = HiddenPatches::new([HiddenPatch {
            mod_code: "para".into(),
            target: MASTER.into(),
            path: "#1/C/items/[1:2]".into(),
        }]);
        assert!(hidden.contains("para", MASTER, "#1/C/items/[1:2]"));
        assert!(!hidden.contains("para", MASTER, "#1/C/items/[0:1]"));

        let seq = |range, values: Vec<&'static str>| JsonPatch {
            action: Action::Seq { op: Op::Replace, range },
            value: values.into(),
        };
        let rules = hidden.rules_of("para", MASTER).unwrap();
        let left = rules.apply(&json_path!["#1", "C", "items"], seq(0..3, vec!["a", "b", "c"]));
        assert_eq!(left, [seq(0..1, vec!["a"]), seq(2..3, vec!["c"])]);
    }

    #[test]
    fn animation_data_goes_by_the_file() {
        let mut hidden = HiddenPatches::default();
        hidden.toggle("para", ADSF, "DefaultMale~1/Idle~348/play_back_speed");
        assert!(hidden.contains("para", ADSF, "DefaultMale~1/Idle~348/play_back_speed"));
        assert!(hidden.contains("para", ADSF, "DefaultMale~1/Idle~348/name"), "another field of the file");
        assert!(!hidden.contains("para", ADSF, "DefaultMale~1/Other~1/name"));
        assert!(hidden.rules_of("para", ADSF).unwrap().hides_file("DefaultMale~1", "Idle~348"));

        // From another field of the file: back again.
        hidden.toggle("para", ADSF, "DefaultMale~1/Idle~348/name");
        assert!(hidden.is_empty());
    }

    #[test]
    fn the_list_comes_back_from_what_it_gave() {
        let mut hidden = HiddenPatches::default();
        hidden.toggle("para", MASTER, "#1/C/a");
        hidden.toggle("amco", ADSF, "D~1/Idle~348/name");
        let again = HiddenPatches::new(hidden.iter().cloned());
        assert_eq!(again, hidden);
    }
}
