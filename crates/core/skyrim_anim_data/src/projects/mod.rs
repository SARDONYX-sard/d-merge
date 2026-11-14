//! The projects of the vanilla game: what animation data is per, under different names.
//!
//! A project is a kind of actor. Each format has its own way of writing the same one:
//! ```text
//! animationsetdatasinglefile.txt   HorseProjectData\HorseProject.txt
//! a Nemesis patch of it            animationsetdatasinglefile/HorseProjectData~HorseProject/...
//! a Nemesis patch of the clip data animationdatasinglefile/HorseProject~1/...
//! the behavior file, as Pandora    actors\horse\horseproject.hkx
//! ```
//!
//! Mods can add projects of their own, so what is not in this table is not wrong.
//! e.g. `NemesisData~NemesisProject` of Nemesis' own mod.

mod generated;

use self::generated::PROJECTS;

/// One project of the vanilla game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Project {
    /// Its folder in the Nemesis patches of `animationsetdatasinglefile`.
    ///
    /// e.g. `HorseProjectData~HorseProject`
    pub asdsf_dir: &'static str,
    /// e.g. `HorseProject`. With `~1`, its folder in the Nemesis patches of `animationdatasinglefile`.
    pub name: &'static str,
    /// Its file under `meshes`. e.g. `actors\horse\horseproject.hkx`
    pub behavior_path: &'static str,
}

/// All of them, in the order of `animationsetdatasinglefile.txt`.
#[inline]
#[must_use]
pub fn vanilla_projects() -> &'static [Project] {
    &PROJECTS
}

/// The vanilla project of a folder in the patches of `animationsetdatasinglefile`, ignoring ASCII case.
#[must_use]
pub fn project_of_asdsf_dir(dir: &str) -> Option<&'static Project> {
    PROJECTS.iter().find(|project| project.asdsf_dir.eq_ignore_ascii_case(dir))
}

/// Is this file, in a folder of `animationsetdatasinglefile`, the list of a project's anim sets?
///
/// That list isn't patched this way: a mod having it is a mistake of its author, and the file can't be
/// read as an anim set.
///
/// The name alone doesn't tell: no vanilla anim set has the name of its project, but
/// `$header$/$header$.txt`, the list of projects, and the projects of mods are something else.
///
/// e.g. `HorseProjectData~HorseProject`, `horseproject.txt`: yes. `DefaultMaleData~DefaultMale`,
/// `_MTSolo.txt`: no.
#[must_use]
pub fn is_project_set_list(dir: &str, file_name: &str) -> bool {
    let Some(project) = project_of_asdsf_dir(dir) else { return false };
    file_name.len().checked_sub(".txt".len()).and_then(|stem_len| file_name.split_at_checked(stem_len)).is_some_and(
        |(stem, extension)| extension.eq_ignore_ascii_case(".txt") && stem.eq_ignore_ascii_case(project.name),
    )
}

/// The name of a template in Pandora, which is also the name of its patch file.
///
/// A project is known by the name of its file, and has what is in the folders beside that:
/// ```text
/// meshes/actors/character/behaviors/0_master.bin              defaultfemale~0_master
/// meshes/actors/character/_1stperson/behaviors/0_master.bin   firstperson~0_master
/// meshes/actors/canine/behaviors wolf/wolfbehavior.bin        dogproject~wolfbehavior
/// meshes/actors/character/characters/defaultmale.bin          defaultmale_character
/// meshes/actors/troll/character assets/skeleton.bin           trollproject_skeleton
/// ```
/// Of the projects sharing a folder, the first one is taken: each of them has the same behaviors.
///
/// `None`: it is not in a folder of a vanilla project.
#[must_use]
pub fn pack_file_name(meshes_path: &str) -> Option<String> {
    pack_file_names(meshes_path).into_iter().next()
}

/// All the names Pandora knows a template by, the one of [`pack_file_name`] first: mods use them all.
///
/// e.g. `meshes/actors/character/behaviors/0_master.bin` -> `defaultfemale~0_master`,
/// `defaultmale~0_master`
///
/// Its name without a project (`0_master`) is one too, but of the first project that has it, so that is
/// for the caller to decide.
#[must_use]
pub fn pack_file_names(meshes_path: &str) -> Vec<String> {
    let path = meshes_path.to_ascii_lowercase().replace('\\', "/");
    let inner = path.strip_prefix("meshes/").unwrap_or(&path);
    let Some((dir, file)) = inner.rsplit_once('/') else { return Vec::new() };
    let stem = file.rsplit_once('.').map_or(file, |(stem, _)| stem);
    let Some((project_dir, folder)) = dir.rsplit_once('/') else { return Vec::new() };

    let identifiers = PROJECTS.iter().filter_map(|project| {
        let project_path = project.behavior_path.to_ascii_lowercase().replace('\\', "/");
        let (parent, file) = project_path.rsplit_once('/')?;
        (parent == project_dir).then(|| file.rsplit_once('.').map_or(file, |(stem, _)| stem).to_owned())
    });
    if folder.starts_with("behaviors") {
        return identifiers.map(|identifier| format!("{identifier}~{stem}")).collect();
    }
    if folder.starts_with("character assets") {
        return identifiers.map(|identifier| format!("{identifier}_skeleton")).collect();
    }
    if folder.starts_with("characters") {
        // The one of its own name, as `characters female/defaultfemale.bin`, is the only one.
        let identifiers: Vec<String> = identifiers.collect();
        if identifiers.iter().any(|identifier| identifier == stem) {
            return vec![format!("{stem}_character")];
        }
        return identifiers.into_iter().map(|identifier| format!("{identifier}_character")).collect();
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_template_has_its_name_in_pandora() {
        let names = [
            ("meshes/actors/character/behaviors/0_master.bin", "defaultfemale~0_master"),
            ("meshes/actors/character/_1stperson/behaviors/0_master.bin", "firstperson~0_master"),
            ("meshes/actors/canine/behaviors wolf/wolfbehavior.bin", "dogproject~wolfbehavior"),
            ("Meshes\\Actors\\Character\\Characters\\DefaultMale.bin", "defaultmale_character"),
            ("meshes/actors/character/characters female/defaultfemale.bin", "defaultfemale_character"),
            ("meshes/actors/character/_1stperson/characters/firstperson.bin", "firstperson_character"),
            ("meshes/actors/troll/behaviors/trollbehavior.bin", "trollproject~trollbehavior"),
        ];
        for (template, name) in names {
            assert_eq!(pack_file_name(template).as_deref(), Some(name), "{template}");
        }
        assert_eq!(
            pack_file_names("meshes/actors/character/behaviors/0_master.bin"),
            ["defaultfemale~0_master", "defaultmale~0_master"]
        );
        assert_eq!(pack_file_name("meshes/animationsetdatasinglefile.bin"), None);
        assert_eq!(pack_file_name("meshes/actors/mymod/behaviors/mine.bin"), None);
    }

    #[test]
    fn a_project_is_found_by_its_folder_in_any_case() {
        let horse = project_of_asdsf_dir("horseprojectdata~HORSEPROJECT").unwrap();
        assert_eq!(horse.name, "HorseProject");
        assert_eq!(horse.behavior_path, "actors\\horse\\horseproject.hkx");
        assert_eq!(project_of_asdsf_dir("NemesisData~NemesisProject"), None);
    }

    #[test]
    fn only_the_file_named_as_a_vanilla_project_is_its_list() {
        assert!(is_project_set_list("HorseProjectData~HorseProject", "horseproject.txt"));
        assert!(is_project_set_list("DefaultMaleData~DefaultMale", "DEFAULTMALE.TXT"));

        assert!(!is_project_set_list("DefaultMaleData~DefaultMale", "_MTSolo.txt"));
        assert!(!is_project_set_list("HorseProjectData~HorseProject", "horseproject.bak"));
        assert!(!is_project_set_list("HorseProjectData~HorseProject", "txt"));
        // The list of projects, which is meant to be patched.
        assert!(!is_project_set_list("$header$", "$header$.txt"));
        // A mod's own project may have what it likes.
        assert!(!is_project_set_list("NemesisData~NemesisProject", "NemesisProject.txt"));
    }

    /// The table against the templates, when they are there: `resource` is a submodule.
    #[test]
    fn the_table_is_what_the_templates_say() {
        let templates = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../resource/xml/templates/meshes/");
        let Ok(asdsf) = std::fs::read(format!("{templates}animationsetdatasinglefile.txt")) else { return };
        let asdsf = String::from_utf8_lossy(&asdsf);
        let mut lines = asdsf.lines();
        let count: usize = lines.next().unwrap().trim().parse().unwrap();
        let expected: Vec<String> = lines.take(count).map(|line| line.replace('\\', "~").replace(".txt", "")).collect();
        let actual: Vec<&str> = PROJECTS.iter().map(|project| project.asdsf_dir).collect();
        assert_eq!(actual, expected);

        // Each is in the list of the clip data too, which `<name>~1` relies on.
        let adsf = std::fs::read(format!("{templates}animationdatasinglefile.txt")).unwrap();
        let adsf = String::from_utf8_lossy(&adsf);
        let mut lines = adsf.lines();
        let count: usize = lines.next().unwrap().trim().parse().unwrap();
        let clip_projects: Vec<&str> = lines.take(count).collect();
        for project in vanilla_projects() {
            let file = format!("{}.txt", project.name);
            assert!(clip_projects.contains(&file.as_str()), "{}", project.name);
        }

        // No anim set has the name of its project, which `is_project_set_list` relies on.
        for project in vanilla_projects() {
            let file = format!("{}.txt", project.name);
            assert!(!asdsf.lines().any(|line| line.eq_ignore_ascii_case(&file)), "{}", project.name);
        }
    }
}
