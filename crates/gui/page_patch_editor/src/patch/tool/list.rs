//! The list of objects: the keyboard in it, what it shows, and its changes.

use std::rc::Rc;

use stdx::text::{cmp_ignore_case, contains_ignore_case};

use crate::patch::{FocusAt, ObjectList, PatchTool, tree, workspace::PANDORA_DOC};

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
    /// The flat list, as it was computed. See [`PatchTool::refresh_list`].
    pub(in crate::patch) list: Option<crate::patch::ObjectList>,

    /// Where the flat list (or the animation data's tree) was scrolled. See [`offset_to_show`].
    pub(in crate::patch) list_scroll: ListScroll,
    /// The row of the flat list to scroll to: the keys moved to it, or the editor got the keyboard.
    pub(in crate::patch) list_scroll_to: Option<usize>,
    /// The keyboard is in the tree or the list (Vim `Space e`).
    pub(in crate::patch) tree_focus: bool,
    /// Show the selected object again, even if it was shown already: the editor got the keyboard.
    pub(in crate::patch) reveal: bool,
}

/// Should a list scroll to the selection? Only when `forced`: the editor got the keyboard, as VS Code's
/// explorer follows the file being edited. A selection alone moves nothing, or a list being used would.
pub(in crate::patch) const fn take_reveal(selected: Option<&Rc<str>>, forced: bool) -> bool {
    forced && selected.is_some()
}
/// The key recorder's texts. See [`keybinding::recorder`].
pub(in crate::patch) fn recorder_texts(i18n: &gui_shared::i18n::I18nMap) -> keybinding::RecorderTexts<'_> {
    use gui_shared::i18n::I18nKey;
    keybinding::RecorderTexts {
        hint: i18n.t(I18nKey::KeybindingRecordHint),
        also_bound: i18n.t(I18nKey::KeybindingAlsoBound),
        edit_json: i18n.t(I18nKey::KeybindingEditInJson),
        labels: egui_code_editor::EditorLabels::from_texts(gui_shared::i18n::editor_label_texts(i18n)),
        unsaved: i18n.t(I18nKey::KeybindingUnsaved),
        save: i18n.t(I18nKey::KeybindingSave),
        dont_save: i18n.t(I18nKey::KeybindingDontSave),
        cancel: i18n.t(I18nKey::CancelButton),
    }
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

    /// Takes the commands of this frame's keys (see [`keybinding`]): carried out by those who ask for
    /// them with [`Self::key_command`] while it is drawn. `keys`: `None` when another has the keyboard
    /// (the main work's tool while the user patch window is open).
    ///
    /// While the palette's `⚙` records new keys, it has them all: see [`keybinding::KeyUi`].
    pub(in crate::patch) fn take_key_commands(
        &mut self,
        ctx: &egui::Context,
        keys: Option<&mut keybinding::Keybindings>,
        i18n: &gui_shared::i18n::I18nMap,
        notify: &mut egui_widgets::notify::Notification,
    ) {
        self.key_commands.clear();
        let Some(keys) = keys else { return };
        let context = self.key_context(ctx);
        let frame = self.key_ui.frame(ctx, keys, &context, recorder_texts(i18n));
        self.key_commands = frame.commands;
        match frame.saved {
            Some(Ok(())) => notify.success(i18n.t(gui_shared::i18n::I18nKey::KeybindingSaved)),
            Some(Err(err)) => notify.error(err),
            None => {}
        }
    }

    /// Was `command` asked for by this frame's keys? It is done then: asked again, `false`.
    ///
    /// e.g. `self.key_command(keybinding::commands::TABS_CLOSE)`
    pub(in crate::patch) fn key_command(&mut self, command: &str) -> bool {
        keybinding::host::take(&mut self.key_commands, command)
    }

    /// Who has the keyboard: what a binding's `when` sees.
    pub(in crate::patch) fn key_context(&self, ctx: &egui::Context) -> keybinding::KeyContext {
        let keymap = egui_code_editor::Keymap::from_vim(self.pane.mode.is_vim());
        let editor_focus = self.pane.had_focus;
        keybinding::KeyContext {
            page: "patchEditor",
            editor_focus,
            vim: keymap.is_vim(),
            vim_mode: self.pane.state.keymap.vim_mode(keymap),
            explorer_focus: self.explorer.tree_focus,
            explorer_view: Some(if self.explorer.tree_view { "tree" } else { "list" }),
            problems_focus: self.problems.cursor.focused,
            problems_visible: self.problems.open,
            input_focus: !editor_focus && ctx.egui_wants_keyboard_input(),
            user_patch: self.is_user_patch,
        }
    }

    /// VS Code's "Copy Path": the path of the file object `id` is in, see [`Self::object_file`].
    pub(in crate::patch) fn copy_object_path(&self, ctx: &egui::Context, id: &str) {
        ctx.copy_text(self.object_file(id).display().to_string());
    }

    /// VS Code's "Reveal in File Explorer": the file object `id` is in, selected in its folder; what
    /// exists of the folder if it isn't written yet.
    pub(in crate::patch) fn reveal_object(&self, id: &str, notify: &mut egui_widgets::notify::Notification) {
        if let Err(err) = gui_shared::fs::reveal_file(&self.object_file(id)) {
            notify.error(err);
        }
    }

    /// `Shift+Alt+C` / `Shift+Alt+R` (key bindings), as VS Code: of the selected object. Also what a
    /// tab's menu or Vim's `<Space>V` asked for ([`Self::file_asked`]).
    pub(in crate::patch) fn file_keys(&mut self, ctx: &egui::Context, notify: &mut egui_widgets::notify::Notification) {
        use keybinding::commands::{FILE_COPY_PATH, FILE_REVEAL};
        let (copy, reveal) = (self.key_command(FILE_COPY_PATH), self.key_command(FILE_REVEAL));
        let selected = self.selected.as_deref().map(str::to_owned);
        let asked = [(copy, FileAction::CopyPath), (reveal, FileAction::Reveal)]
            .into_iter()
            .filter_map(|(on, action)| on.then_some(action).zip(selected.clone()));
        for (action, id) in asked.chain(self.file_asked.take()) {
            match action {
                FileAction::CopyPath => self.copy_object_path(ctx, &id),
                FileAction::Reveal => self.reveal_object(&id, notify),
            }
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

    /// The file object `id` of the active behavior is in, to find it on disk: its patch file in the
    /// mod if this work has one for it (as the export writes it, whether written yet or not), else
    /// the template it is read from.
    ///
    /// e.g. `D:/mods/My/Nemesis_Engine/mod/mymod/0_master/#0106.txt`,
    /// `D:/mods/My/Pandora_Engine/mod/mymod/patches/defaultmale~0_master.xml`,
    /// `<templates>/meshes/actors/character/behaviors/0_master.xml`
    pub(in crate::patch) fn object_file(&self, id: &str) -> std::path::PathBuf {
        let behavior = &self.workspace.behaviors[self.active];
        let patched = behavior.edits.contains_key(id) || behavior.added.contains_key(id);
        let (output, code) = (self.output.trim(), self.mod_code.trim());
        if !patched || output.is_empty() || code.is_empty() {
            return behavior.template_path.clone();
        }
        let output = std::path::Path::new(output);
        let format = self.workspace.export_format;
        if format == crate::patch::workspace::ExportFormat::PandoraV5
            && !behavior.is_anim_data()
            && let Some(file) = behavior.pandora_patch_path(output, code)
        {
            return file;
        }
        output.join(behavior.patch_dir(code, format.is_ext())).join(behavior.patch_file(id))
    }

    /// The keyboard goes back to the editor.
    pub(in crate::patch) const fn leave_tree(&mut self) {
        self.explorer.tree_focus = false;
        self.pane.focus = Some(FocusAt::Keep);
    }

    /// The flat list has the same keys as the tree, without the folding.
    pub(in crate::patch) fn list_keys(&mut self, graph: &tree::Graph) {
        let rows = self.explorer.list.as_ref().map_or(&[][..], |list| list.rows.as_slice());
        let selected = self.selected.as_deref();
        let mut current = rows.iter().position(|&node| selected == Some(graph.id(node))).unwrap_or(0);
        let mut leave = false;
        let before = current;
        for input in keybinding::host::take_mapped(&mut self.key_commands, keybinding::host::tree_key) {
            leave |= input == tree::TreeKey::Leave;
            current = input.step(current, rows.len()).unwrap_or(current);
        }
        if let Some(&node) = rows.get(current) {
            self.selected = Some(graph.id(node).into());
        }
        if current != before {
            self.explorer.list_scroll_to = Some(current);
        }
        if leave {
            self.leave_tree();
        }
    }

    /// Recomputes the flat list when the objects, the filter, "changed only" or "unlinked only" changed.
    ///
    /// New objects come first and show where they are linked: `+ #mymod$0  hkbFoo  ← #0106`.
    ///
    /// Without a filter the labels do not matter, so typing in the editor does not recompute it.
    pub(in crate::patch) fn refresh_list(&mut self, graph: &tree::Graph) {
        let generation = if self.explorer.filter.is_empty() { graph.tree_generation() } else { graph.generation() };
        let fresh = self.explorer.list.as_ref().is_some_and(|list| {
            list.generation == generation
                && list.filter == self.explorer.filter
                && list.changed_only == self.explorer.changed_only
                && list.unlinked_only == self.explorer.unlinked_only
        });
        if fresh {
            return;
        }
        let filter = self.explorer.filter.as_str();
        let rows = graph
            .nodes()
            .filter(|&node| !self.explorer.changed_only || graph.is_changed(node))
            .filter(|&node| !self.explorer.unlinked_only || graph.is_unlinked(node))
            .filter(|&node| contains_ignore_case(graph.label(node), filter))
            .collect();
        // Anim data is grouped by actor: see `ui::anim_tree`.
        let mut rows: Vec<tree::Node> = rows;
        if self.workspace.behaviors[self.active].is_anim_data() {
            rows.sort_by(|&a, &b| cmp_ignore_case(graph.id(a), graph.id(b)));
        }
        self.explorer.list = Some(ObjectList {
            generation,
            filter: self.explorer.filter.clone(),
            changed_only: self.explorer.changed_only,
            unlinked_only: self.explorer.unlinked_only,
            rows,
        });
    }

    /// Deletes a new object, or drops the edits of a template object.
    pub(in crate::patch) fn revert_or_delete(&mut self, id: &str) {
        let behavior = &mut self.workspace.behaviors[self.active];
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
        let code = self.mod_code.trim().to_string();
        let id = self.workspace.behaviors[self.active].add_object(&code, from);
        self.object_changed(&id);
        self.selected = Some(id.into());
    }
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
        let mut tool =
            PatchTool { mod_code: "mymod".to_owned(), output: "D:/mods/My".to_owned(), ..Default::default() };
        tool.workspace.behaviors.push(clips);

        let patch = Path::new("D:/mods/My").join("Nemesis_Engine/mod/mymod").join(ADSF).join("Male~1/m$0.txt");
        assert_eq!(tool.object_file("Male~1/m$0.txt"), patch);
        let template = tool.workspace.behaviors[0].template_path.clone();
        assert_eq!(tool.object_file("Male~1/other.txt"), template, "not patched");
        tool.output.clear();
        assert_eq!(tool.object_file("Male~1/m$0.txt"), template, "no mod to be in");
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
