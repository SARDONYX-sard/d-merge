//! The list of objects: the keyboard in it, what it shows, and its changes.

use std::rc::Rc;

use crate::patch::{FocusAt, PatchTool, tool::editing::EditorPane, workspace::PANDORA_DOC};

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

impl PatchTool {
    /// Notes whether the editor `has_focus` this frame. Getting it asks the side bar to show the
    /// object again (next frame: the side bar is drawn first).
    pub(in crate::patch) fn note_editor_focus(&mut self, ctx: &egui::Context, has_focus: bool) {
        if has_focus && !self.pane.had_focus {
            self.explorer.reveal = true;
            ctx.request_repaint();
        }
        self.pane.had_focus = has_focus;
    }

    /// The keyboard goes to the object tree or list, as it shows, its filter kept (Vim `Space e`, or
    /// the palette).
    pub(in crate::patch) fn enter_tree(&mut self, ui: &egui::Ui, editor: egui::Id) {
        // The view shown, as VS Code's "Focus Side Bar": the tree or the list, or the changes. The
        // views without keys of their own (the search, the variables) give way to the explorer.
        use crate::patch::ui::side_bar::SideView;
        self.explorer.tree_focus = true;
        if !matches!(self.side.view, SideView::Explorer | SideView::Changes) || self.side.hidden {
            self.side.view = SideView::Explorer;
        }
        self.side.hidden = false;
        self.side.objects_closed = false;
        ui.memory_mut(|m| m.surrender_focus(editor));
    }

    /// Reads this frame's keys: see [`super::keys::Keys::frame`].
    pub(in crate::patch) fn take_key_commands(
        &mut self,
        ctx: &egui::Context,
        bindings: Option<&mut keybinding::Keybindings>,
        i18n: &gui_shared::i18n::I18nMap,
        notify: &mut egui_widgets::notify::Notification,
    ) {
        let context = key_context(ctx, (&self.pane, &self.explorer, &self.problems), self.is_user_patch);
        self.keys.frame(ctx, bindings, &context, (i18n, notify));
    }

    /// VS Code's "Copy Path" / "Reveal in File Explorer" of the file object `id` is in (see
    /// [`object_file`]): its path to the clipboard, or the file selected in its folder (what exists
    /// of the folder if it isn't written yet).
    pub(in crate::patch) fn file_action(
        &self,
        ctx: &egui::Context,
        (action, id): (FileAction, &str),
        notify: &mut egui_widgets::notify::Notification,
    ) {
        let behavior = &self.work.workspace.behaviors[self.active];
        let file = object_file((behavior, id), &self.work.place, self.work.workspace.export_format);
        match action {
            FileAction::CopyPath => ctx.copy_text(file.display().to_string()),
            FileAction::Reveal => {
                if let Err(err) = gui_shared::fs::reveal_file(&file) {
                    notify.error(err);
                }
            }
        }
    }

    /// `Shift+Alt+C` / `Shift+Alt+R` (key bindings), as VS Code: of the selected object. Also what a
    /// tab's menu or Vim's `<Space>V` asked for ([`Self::file_asked`]).
    pub(in crate::patch) fn file_keys(&mut self, ctx: &egui::Context, notify: &mut egui_widgets::notify::Notification) {
        use keybinding::commands::{FILE_COPY_PATH, FILE_REVEAL};
        let (copy, reveal) = (self.keys.take(FILE_COPY_PATH), self.keys.take(FILE_REVEAL));
        let selected = self.selected.as_deref().map(str::to_owned);
        let asked = [(copy, FileAction::CopyPath), (reveal, FileAction::Reveal)]
            .into_iter()
            .filter_map(|(on, action)| on.then_some(action).zip(selected.clone()));
        for (action, id) in asked.chain(self.file_asked.take()) {
            self.file_action(ctx, (action, &id), notify);
        }
    }

    /// `Ctrl+Shift+L`, as VS Code's `Ctrl+Shift+M`: the Problems panel shows and gets the keyboard;
    /// if it has it, it hides and the editor gets it back. See [`egui_code_editor::problems`].
    pub(in crate::patch) fn toggle_problems_focus(&mut self, ui: &egui::Ui, editor: egui::Id) {
        let cursor = &mut self.problems.cursor;
        if cursor.focused {
            cursor.focused = false;
            self.problems.open = false;
            self.pane.focus = Some(FocusAt::Keep);
            return;
        }
        self.problems.open = true;
        if self.problems.list.is_none() {
            return; // Not checked yet: no panel to take the keyboard.
        }
        self.problems.cursor.focused = true;
        self.explorer.tree_focus = false;
        ui.memory_mut(|m| m.surrender_focus(editor));
    }

    /// The keyboard goes back to the editor.
    pub(in crate::patch) const fn leave_tree(&mut self) {
        self.explorer.tree_focus = false;
        self.pane.focus = Some(FocusAt::Keep);
    }

    /// Deletes a new object, or drops the edits of a template object.
    pub(in crate::patch) fn revert_or_delete(&mut self, id: &str) {
        let behavior = &mut self.work.workspace.behaviors[self.active];
        behavior.edits.remove(id);
        let is_pandora_doc = id == PANDORA_DOC && behavior.pandora_doc.take().is_some();
        if behavior.added.remove(id).is_some() || is_pandora_doc {
            let neighbor = self.tabs.close(self.active, id);
            if self.selected.as_deref() == Some(id) {
                self.selected = neighbor.map(Rc::from);
            }
        }
        self.pane.opened = None;
        self.refresh_problems(id);
        self.object_changed(id);
    }

    pub(in crate::patch) fn new_object(&mut self, from: Option<&str>) {
        let code = self.work.place.mod_code.trim().to_string();
        let id = self.work.workspace.behaviors[self.active].add_object(&code, from);
        self.object_changed(&id);
        self.selected = Some(id.into());
    }
}

/// Who has the keyboard: what a binding's `when` sees. `user_patch`: the user patch's window.
fn key_context(
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
pub(in crate::patch) fn object_file(
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
        let mut tool = PatchTool::default();
        tool.work.place.mod_code = "mymod".to_owned();
        tool.work.place.output = "D:/mods/My".to_owned();
        tool.work.workspace.behaviors.push(clips);

        let patch = Path::new("D:/mods/My").join("Nemesis_Engine/mod/mymod").join(ADSF).join("Male~1/m$0.txt");
        assert_eq!(
            object_file(
                (&tool.work.workspace.behaviors[0], "Male~1/m$0.txt"),
                &tool.work.place,
                tool.work.workspace.export_format
            ),
            patch
        );
        let template = tool.work.workspace.behaviors[0].template_path.clone();
        assert_eq!(
            object_file(
                (&tool.work.workspace.behaviors[0], "Male~1/other.txt"),
                &tool.work.place,
                tool.work.workspace.export_format
            ),
            template,
            "not patched"
        );
        tool.work.place.output.clear();
        assert_eq!(
            object_file(
                (&tool.work.workspace.behaviors[0], "Male~1/m$0.txt"),
                &tool.work.place,
                tool.work.workspace.export_format
            ),
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
        let ctx = egui::Context::default();
        let mut tool = PatchTool::default();
        tool.note_editor_focus(&ctx, true);
        assert!(std::mem::take(&mut tool.explorer.reveal));
        tool.note_editor_focus(&ctx, true);
        assert!(!tool.explorer.reveal);
        tool.note_editor_focus(&ctx, false);
        tool.note_editor_focus(&ctx, true);
        assert!(tool.explorer.reveal);
    }
}
