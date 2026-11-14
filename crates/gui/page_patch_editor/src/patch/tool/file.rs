//! The file an object is in, and what is done with it: VS Code's "Copy Path" / "Reveal in File Explorer".

use crate::patch::tool::work_state::Work;

/// What is done with the file an object is in, as VS Code's "Copy Path" / "Reveal in File Explorer".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::patch) enum FileAction {
    CopyPath,
    Reveal,
}

/// VS Code's "Copy Path" / "Reveal in File Explorer" of the file object `id` of the active behavior
/// is in (see [`object_file`]): its path to the clipboard, or the file selected in its folder (what
/// exists of the folder if it isn't written yet).
pub(in crate::patch) fn file_action(
    ctx: &egui::Context,
    (work, active): (&Work, usize),
    (action, id): (FileAction, &str),
    notify: &mut egui_widgets::notify::Notification,
) {
    let behavior = &work.workspace.behaviors[active];
    let file = object_file((behavior, id), &work.place, work.workspace.export_format);
    match action {
        FileAction::CopyPath => ctx.copy_text(file.display().to_string()),
        FileAction::Reveal => {
            if let Err(err) = gui_shared::fs::reveal_file(&file) {
                notify.error(err);
            }
        }
    }
}

/// The file object `id` of `behavior` is in, to find it on disk: its patch file in the mod if this
/// work has one for it (as the export, in `format`, writes it, whether written yet or not), else the
/// template it is read from.
///
/// e.g. `D:/mods/My/Nemesis_Engine/mod/mymod/0_master/#0106.txt`,
/// `D:/mods/My/Pandora_Engine/mod/mymod/patches/defaultmale~0_master.xml`,
/// `<templates>/meshes/actors/character/behaviors/0_master.xml`
fn object_file(
    (behavior, id): (&crate::patch::workspace::Behavior, &str),
    place: &crate::patch::tool::place::WorkPlace,
    format: crate::patch::workspace::ExportFormat,
) -> std::path::PathBuf {
    let patched = behavior.edits.contains_key(id) || behavior.added.contains_key(id);
    let (output, code) = (place.output.trim(), place.mod_code.trim());
    if !patched || output.is_empty() || code.is_empty() {
        return behavior.template_path.clone();
    }

    let output = std::path::Path::new(output);
    if format == crate::patch::workspace::ExportFormat::PandoraV5
        && !behavior.is_anim_data()
        && let Some(file) = behavior.pandora_patch_path(output, code)
    {
        return file;
    }
    output.join(behavior.patch_dir(code, format.is_ext())).join(behavior.patch_file(id))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A patched object is in its patch file in the mod; one that isn't, in its template.
    #[test]
    fn the_file_of_an_object() {
        use std::path::Path;

        use crate::patch::workspace::{ADSF, Behavior};
        let mut clips = Behavior::open(&Path::new("meshes").join(format!("{ADSF}.bin"))).unwrap();
        clips.added.insert("Male~1/m$0.txt".to_owned(), String::new());
        let mut work = Work::default();
        work.place.mod_code = "mymod".to_owned();
        work.place.output = "D:/mods/My".to_owned();
        work.workspace.behaviors.push(clips);

        let patch = Path::new("D:/mods/My").join("Nemesis_Engine/mod/mymod").join(ADSF).join("Male~1/m$0.txt");
        assert_eq!(
            object_file((&work.workspace.behaviors[0], "Male~1/m$0.txt"), &work.place, work.workspace.export_format),
            patch
        );
        let template = work.workspace.behaviors[0].template_path.clone();
        assert_eq!(
            object_file((&work.workspace.behaviors[0], "Male~1/other.txt"), &work.place, work.workspace.export_format),
            template,
            "not patched"
        );
        work.place.output.clear();
        assert_eq!(
            object_file((&work.workspace.behaviors[0], "Male~1/m$0.txt"), &work.place, work.workspace.export_format),
            template,
            "no mod to be in"
        );
    }
}
