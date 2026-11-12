//! Adding a mod's animations to the `animationNames` of its actor's characters: a behavior plays a hkx
//! only if the character lists it. Which character is told by where the hkx is: its `animations`
//! folder is the actor's, whose characters are beside it.
//!
//! ```text
//! meshes\actors\character\animations\Para\Equip.hkx           (a hkx of the mod)
//!   actor:     meshes/actors/character
//!   name:      Animations\Para\Equip.hkx                        (relative to the actor)
//!   templates: meshes/actors/character/characters/defaultmale.bin,
//!              meshes/actors/character/characters female/defaultfemale.bin
//!   -> their hkbCharacterStringData:
//!      <hkparam name="animationNames" numelements="N">
//!          ...
//!          <hkcstring>Animations\Para\Equip.hkx</hkcstring>   (pushed, if not there)
//!      </hkparam>
//! ```
//! The count is the merge's to make: see `nemesis_diff::align_numelements`.

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use crate::patch::workspace::{Behavior, Workspace};

/// The class holding a character's names.
const CHARACTER_STRING_DATA: &str = "hkbCharacterStringData";
/// Its field of the animations.
const ANIMATION_NAMES: &str = "animationNames";

/// A hkx of the mod (as `data::anim_hkx_files` lists them) as its actor folder (lowercase, `/`) and its
/// name in the character. `None`: it is not in an `animations` folder.
///
/// e.g. `meshes\actors\character\animations\Para\Equip.hkx`
/// -> `meshes/actors/character`, `Animations\Para\Equip.hkx`
pub(in crate::patch) fn actor_and_name(hkx: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = hkx.split(['\\', '/']).collect();
    let at = parts.iter().position(|part| part.eq_ignore_ascii_case("animations"))?;
    let actor = parts[..at].join("/").to_ascii_lowercase();
    let name = format!("Animations\\{}", parts[at + 1..].join("\\"));
    (at > 0 && at + 1 < parts.len()).then_some((actor, name))
}

/// The character templates of `actor` under `template_root`: the `.bin` / `.xml` files of its folders
/// whose names start with `characters` (`characters female` too).
///
/// e.g. `meshes/actors/character` -> `.../characters/defaultmale.bin`, `.../characters female/defaultfemale.bin`
pub(in crate::patch) fn character_templates(template_root: &Path, actor: &str) -> Vec<PathBuf> {
    let Ok(folders) = std::fs::read_dir(template_root.join(actor)) else { return Vec::new() };
    let mut found: Vec<PathBuf> = folders
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .filter(|path| {
            path.file_name().is_some_and(|name| name.to_string_lossy().to_ascii_lowercase().starts_with("characters"))
        })
        .filter_map(|folder| std::fs::read_dir(folder).ok())
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("bin") || ext.eq_ignore_ascii_case("xml"))
        })
        .collect();
    found.sort();
    found
}

/// `object` (a `hkbCharacterStringData`) with `names` added at the end of its `animationNames`, those
/// it has left out (ignoring ASCII case, as the game does). `None`: it has no such list, or nothing to add.
pub(in crate::patch) fn with_animation_names(object: &str, names: &[&str]) -> Option<String> {
    let lines: Vec<&str> = object.split_inclusive('\n').collect();
    let open = format!("<hkparam name=\"{ANIMATION_NAMES}\"");
    let start = lines.iter().position(|line| line.trim_start().starts_with(open.as_str()))?;
    let indent = &lines[start][..lines[start].len() - lines[start].trim_start().len()];
    // An empty list on one line: written open, as Nemesis' push to an empty array (`kept_array`).
    let empty = lines[start].trim_end().ends_with("</hkparam>");
    let end = if empty { start } else { start + lines[start..].iter().position(|line| line.trim() == "</hkparam>")? };
    let known: Vec<&str> = lines[start..end]
        .iter()
        .filter_map(|line| line.trim().strip_prefix("<hkcstring>")?.strip_suffix("</hkcstring>"))
        .collect();
    let mut added: Vec<&str> = Vec::new();
    for name in names {
        let is_known = |other: &&str| other.eq_ignore_ascii_case(name);
        if !known.iter().any(is_known) && !added.iter().any(is_known) {
            added.push(name);
        }
    }
    if added.is_empty() {
        return None;
    }
    let mut out = String::with_capacity(object.len() + added.len() * 64);
    lines[..end].iter().for_each(|line| out.push_str(line));
    if empty {
        out.push_str(lines[start].trim_end().strip_suffix("</hkparam>").unwrap_or(lines[start]));
        out.push('\n');
    }
    for name in &added {
        out.push_str(&format!("{indent}\t<hkcstring>{name}</hkcstring>\n"));
    }
    if empty {
        out.push_str(&format!("{indent}</hkparam>\n"));
        lines[start + 1..].iter().for_each(|line| out.push_str(line));
    } else {
        lines[end..].iter().for_each(|line| out.push_str(line));
    }
    Some(out)
}

/// What [`Workspace::add_animation_names`] did.
#[derive(Debug, Default, PartialEq, Eq)]
pub(in crate::patch) struct NamesAdded {
    /// The characters changed, e.g. `defaultmale`.
    pub(in crate::patch) characters: Vec<String>,
    /// The actors with no character template there, e.g. `meshes/actors/custom`.
    pub(in crate::patch) no_character: Vec<String>,
}

impl Workspace {
    /// Adds the hkx of the mod (`data::anim_hkx_files` paths) to the `animationNames` of their actors'
    /// characters. See the module docs. `allows`: whether the work may patch a template (its scope).
    pub(in crate::patch) fn add_animation_names(
        &mut self,
        template_root: &Path,
        hkx: &[&str],
        allows: impl Fn(&Behavior) -> bool,
    ) -> Result<NamesAdded, String> {
        let mut by_actor: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for (actor, name) in hkx.iter().filter_map(|hkx| actor_and_name(hkx)) {
            by_actor.entry(actor).or_default().push(name);
        }
        let mut done = NamesAdded::default();
        for (actor, names) in &by_actor {
            let templates = character_templates(template_root, actor);
            if templates.is_empty() {
                done.no_character.push(actor.clone());
            }
            let names: Vec<&str> = names.iter().map(String::as_str).collect();
            for template in templates {
                let relative = template.strip_prefix(template_root).unwrap_or(&template).with_extension("bin");
                let meshes_path = relative.to_string_lossy().replace('\\', "/").to_ascii_lowercase();
                let at = self.open_behavior(&template, &meshes_path)?;
                let behavior = &mut self.behaviors[at];
                if !allows(behavior) {
                    continue;
                }
                let Some(id) = behavior.object_of_class(CHARACTER_STRING_DATA).map(str::to_owned) else { continue };
                let current = behavior.text(&id).unwrap_or_default();
                if let Some(edited) = with_animation_names(current, &names) {
                    behavior.set_text(&id, &edited);
                    done.characters.push(behavior.name.clone());
                }
            }
        }
        Ok(done)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hkx_tells_its_actor_and_its_name() {
        let found = actor_and_name(r"meshes\actors\character\animations\Para\Equip.hkx");
        assert_eq!(found, Some(("meshes/actors/character".to_owned(), r"Animations\Para\Equip.hkx".to_owned())));
        let first = actor_and_name(r"meshes\actors\character\_1stperson\animations\x.hkx").unwrap();
        assert_eq!(first.0, "meshes/actors/character/_1stperson");
        assert_eq!(actor_and_name(r"meshes\actors\character\behaviors\0_master.hkx"), None);
    }

    const STRING_DATA: &str = "\t\t<hkobject name=\"#0002\" class=\"hkbCharacterStringData\" signature=\"0x655b42bc\">\n\
                               \t\t\t<hkparam name=\"animationNames\" numelements=\"1\">\n\
                               \t\t\t\t<hkcstring>Animations\\Idle.hkx</hkcstring>\n\
                               \t\t\t</hkparam>\n\
                               \t\t</hkobject>\n";

    #[test]
    fn the_names_are_pushed_once() {
        let names = [r"Animations\Para\Equip.hkx", r"animations\idle.hkx", r"Animations\Para\Equip.hkx"];
        let edited = with_animation_names(STRING_DATA, &names).unwrap();
        assert_eq!(edited.matches("<hkcstring>").count(), 2, "{edited}");
        assert!(edited.contains("\t\t\t\t<hkcstring>Animations\\Para\\Equip.hkx</hkcstring>\n\t\t\t</hkparam>"));
        assert_eq!(with_animation_names(STRING_DATA, &[r"ANIMATIONS\IDLE.HKX"]), None, "nothing to add");

        let patch = nemesis_diff::object_patch("mymod", STRING_DATA, &edited).unwrap();
        nemesis_diff::validate(&patch).unwrap();
    }

    #[test]
    fn an_empty_list_is_pushed_to() {
        let empty = STRING_DATA.replace(
            "numelements=\"1\">\n\t\t\t\t<hkcstring>Animations\\Idle.hkx</hkcstring>\n\t\t\t</hkparam>",
            "numelements=\"0\"></hkparam>",
        );
        let edited = with_animation_names(&empty, &[r"Animations\A.hkx"]).unwrap();
        let patch = nemesis_diff::object_patch("mymod", &empty, &edited).unwrap();
        assert!(patch.contains("<hkcstring>Animations\\A.hkx</hkcstring>"), "{patch}");
        nemesis_diff::validate(&patch).unwrap();
    }

    /// The shipped templates of the character have their names object.
    #[test]
    fn the_characters_of_the_character_are_found() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../resource/assets/templates");
        let found = character_templates(&root, "meshes/actors/character");
        let names: Vec<String> =
            found.iter().filter_map(|path| path.file_stem()).map(|stem| stem.to_string_lossy().into_owned()).collect();
        assert_eq!(names, ["defaultmale", "defaultfemale"]);
    }

    /// From the hkx to the patches of both characters, which d-merge's parser takes.
    #[test]
    fn a_hkx_of_the_character_goes_to_both_characters() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../resource/assets/templates");
        let mut workspace = Workspace::default();
        let hkx = [r"meshes\actors\character\animations\Para\Equip.hkx", r"meshes\actors\custom\animations\x.hkx"];
        let done = workspace.add_animation_names(&root, &hkx, |_| true).unwrap();

        assert_eq!(done.characters, ["defaultmale", "defaultfemale"]);
        assert_eq!(done.no_character, ["meshes/actors/custom"]);
        for behavior in &workspace.behaviors {
            let id = behavior.edits.keys().next().unwrap();
            let patch = behavior.patch("mymod", id).unwrap();
            assert!(patch.contains(r"<hkcstring>Animations\Para\Equip.hkx</hkcstring>"), "{patch}");
            nemesis_diff::validate(&patch).unwrap();
        }
    }
}
