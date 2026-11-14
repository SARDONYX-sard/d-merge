//! The list of objects: the keyboard in it, what it shows, and its changes.

use std::rc::Rc;

use crate::patch::tool::{editing::EditorPane, work_state::Work};

/// The state of the objects in the explorer.
///
/// ```text
/// [filter.......]  ⋯ changed only / unlinked only    [≡] tree_view
/// tree (trees, by behavior) or flat list (list)       tree_focus: the keyboard is in it
/// ```
#[derive(Debug, Default)]
pub(in crate::patch) struct Explorer {
    pub(in crate::patch) filter: String,
    /// Only the objects changed, and the way to them in the tree.
    pub(in crate::patch) changed_only: bool,
    /// Only the new objects nothing links to. See [`crate::patch::tree::Graph::unlinked`].
    pub(in crate::patch) unlinked_only: bool,
    /// A tree from the root container, else a flat list.
    pub(in crate::patch) tree_view: bool,
    /// The tree of each behavior, by template path.
    pub(in crate::patch) trees: fast_hash::HashMap<String, crate::patch::tree::TreeState>,
    /// The flat list, as it was computed. See `Explorer::refresh_list`.
    pub(in crate::patch) list: Option<crate::patch::ObjectList>,

    /// Where the flat list (or the animation data's tree) was scrolled. See [`offset_to_show`].
    pub(in crate::patch) list_scroll: ListScroll,
    /// The row of the flat list to scroll to: the keys moved to it, or the editor got the keyboard.
    pub(in crate::patch) list_scroll_to: Option<usize>,
    /// The keyboard is in the tree or the list (Vim `Space e`).
    pub(in crate::patch) tree_focus: bool,
    /// Show the selected object again, even if it was shown already: the editor got the keyboard.
    pub(in crate::patch) reveal: bool,
    /// What is open in the animation data's list, and its cursor.
    pub(in crate::patch) anim_tree: crate::patch::ui::anim_tree::AnimTreeState,
}

/// Should a list scroll to the selection? Only when `forced`: the editor got the keyboard, as VS Code's
/// explorer follows the file being edited. A selection alone moves nothing, or a list being used would.
pub(in crate::patch) const fn take_reveal(selected: Option<&Rc<str>>, forced: bool) -> bool {
    forced && selected.is_some()
}

/// What is done with the file an object is in, as VS Code's "Copy Path" / "Reveal in File Explorer".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::patch) enum FileAction {
    CopyPath,
    Reveal,
}

pub(in crate::patch) use egui_tree::list::{ListScroll, fixed_row, offset_to_show, row_height};

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

/// Who has the keyboard: what a binding's `when` sees. `user_patch`: the user patch's window.
pub(in crate::patch) fn key_context(
    ctx: &egui::Context,
    (pane, explorer, problems): (&EditorPane, &Explorer, &crate::patch::problems::Problems),
    user_patch: bool,
) -> keybinding::KeyContext {
    let keymap = egui_code_editor::Keymap::from_vim(pane.mode.is_vim());
    let editor_focus = pane.had_focus;
    keybinding::KeyContext {
        page: "patchEditor",
        editor_focus,
        vim: keymap.is_vim(),
        vim_mode: pane.state.keymap.vim_mode(keymap),
        explorer_focus: explorer.tree_focus,
        explorer_view: Some(if explorer.tree_view { "tree" } else { "list" }),
        problems_focus: problems.cursor.focused,
        problems_visible: problems.open,
        input_focus: !editor_focus && ctx.egui_wants_keyboard_input(),
        user_patch,
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

    #[test]
    fn a_list_scrolls_only_when_the_editor_gets_the_keyboard() {
        let a: Rc<str> = "#a".into();
        assert!(!take_reveal(Some(&a), false), "a selection alone");
        assert!(take_reveal(Some(&a), true), "the editor got the keyboard");
        assert!(!take_reveal(None, true), "nothing to show");
    }

    /// Only getting the keyboard asks: keeping it does not, or the side bar could not be scrolled.
    #[test]
    fn getting_into_the_editor_asks_the_side_bar_to_follow() {
        let mut pane = EditorPane::default();
        assert!(pane.note_focus(true));
        assert!(!pane.note_focus(true));
        assert!(!pane.note_focus(false));
        assert!(pane.note_focus(true));
    }
}
