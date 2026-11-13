//! The code editor of the open object, and what it is given: its language (Nemesis XML, the lines
//! of the animation data, or the Pandora document), its keys, its palette actions.
//!
//! ```text
//! editor_panel
//! ├ editor_keys      Space p / Space d / Space w / ]c / [c of the key bindings
//! ├ toolbar          (toolbar.rs)
//! ├ status_bar       (bottom.rs)
//! ├ problems_ui      (bottom.rs)
//! └ code_editor      the language, the editor, then what it asked for -> EditorResult
//! ```

use std::rc::Rc;

use egui_code_editor::{
    CodeEditor, CodeEditorOutput, EditorAction, EditorCommand, EditorLabels, EditorRequest, Keymap, Language,
    Navigation, ReferenceList,
};
use gui_shared::i18n::{I18nKey, I18nMap};
use keybinding::{
    commands,
    host::{self, OPEN_JSON_ACTION},
};

use super::bottom::{ProblemsOutput, editor_features};
use crate::patch::{
    EditorResult, FocusAt, PatchTool,
    anim::lines::AnimDataLanguage,
    editor::{language::NemesisXml, names::Names, pandora::PandoraXml},
    workspace::PANDORA_DOC,
};

/// The palette's "focus the object tree", "go back", "go forward".
const ACTION_FOCUS_TREE: &str = "focus_tree";
const ACTION_BACK: &str = "go_back";
const ACTION_FORWARD: &str = "go_forward";

/// What this editor acts on: see [`PatchTool::apply_editor_result`].
const HOST_COMMANDS: [EditorCommand; 9] = [
    EditorCommand::NextTab,
    EditorCommand::PreviousTab,
    EditorCommand::CloseTab,
    EditorCommand::TogglePreview,
    EditorCommand::ToggleInlineDiff,
    EditorCommand::NextChange,
    EditorCommand::PreviousChange,
    EditorCommand::ToggleExplorer,
    EditorCommand::RevealFile,
];

impl PatchTool {
    /// The toolbar, the editor, the Problems panel and the status bar of object `id`. Returns what
    /// the editor asked for: it is done afterwards, as the language still borrows the behavior here.
    pub(super) fn editor_panel(&mut self, ui: &mut egui::Ui, i18n: &I18nMap, id: &str) -> EditorResult {
        let format_and_save = self.editor_keys();
        let problem_keys = host::take_mapped(&mut self.key_commands, host::problems_key);
        let requests = self.editor_requests(format_and_save);

        // The status bar counts the whole work's problems, of which the open object's are a part.
        self.pane.state.set_status_problems(self.problem_counts());
        let names = self.names();
        let keymap = Keymap::from_vim(self.pane.mode.is_vim());
        let editor_id = egui_code_editor::editor_id(("nemesis_object", self.is_user_patch, id));
        let editor_focused = ui.memory(|m| m.has_focus(editor_id));

        let mut result = EditorResult::default();
        let mut toggle_problems = false;
        let mut problems = ProblemsOutput::default();
        egui::CentralPanel::default().show(ui, |ui| {
            let toolbar = self.toolbar(ui, i18n, id, (&names.graph, editor_focused));
            (result.walk_history, result.tab_event) = (toolbar.walk_history, toolbar.tab_event);

            if self.workspace.behaviors[self.active].is_ref(id) {
                ui.weak(i18n.t(I18nKey::NemesisRefReadOnly));
            }

            toggle_problems = self.status_bar(ui, i18n, keymap);
            problems = self.problems_ui(ui, i18n, &problem_keys);
            self.code_editor(ui, i18n, id, (&names, keymap), &requests, &mut result);
        });

        if format_and_save {
            result.command = Some(EditorCommand::Save); // after the format of this frame
        }
        self.apply_problems(toggle_problems, problems);
        result
    }

    /// Vim's `Space p` / `Space d` / `Space w` and `]c` / `[c` (key bindings). Returns whether
    /// `Space w` (format and save) was pressed.
    fn editor_keys(&mut self) -> bool {
        if self.key_command(commands::EDITOR_TOGGLE_PREVIEW) {
            self.pane.show_preview = !self.pane.show_preview;
        }
        if self.key_command(commands::EDITOR_TOGGLE_INLINE_DIFF) {
            self.pane.show_inline_diff = !self.pane.show_inline_diff;
        }

        if self.key_command(commands::EDITOR_NEXT_CHANGE) {
            self.pane.step_change(true);
        }
        if self.key_command(commands::EDITOR_PREVIOUS_CHANGE) {
            self.pane.step_change(false);
        }

        self.key_command(commands::EDITOR_FORMAT_AND_SAVE)
    }

    /// The editor's own commands, by the key bindings' keys: it reads none of its keys itself then.
    fn editor_requests(&mut self, format_and_save: bool) -> Vec<EditorRequest> {
        let mut requests = host::take_mapped(&mut self.key_commands, host::editor_request);
        if format_and_save {
            requests.push(EditorRequest::Format);
        }
        requests
    }

    /// The editor itself, and what it asked for into `result`.
    fn code_editor(
        &mut self,
        ui: &mut egui::Ui,
        i18n: &I18nMap,
        id: &str,
        (names, keymap): (&Rc<Names>, Keymap),
        requests: &[EditorRequest],
        result: &mut EditorResult,
    ) {
        let values = self.pane.field_values(self.active, &self.workspace.behaviors[self.active]);
        let behavior = &self.workspace.behaviors[self.active];
        let is_pandora_doc = id == PANDORA_DOC;

        // The class (and fields) around the cursor line of last frame: field names, hover.
        let context = if is_pandora_doc {
            crate::patch::editor::pandora::fields_at(behavior, &self.pane.text, self.pane.cursor_line)
        } else {
            nemesis_diff::schema::field_context(&self.pane.text, self.pane.cursor_line)
        };
        let open_field = if is_pandora_doc {
            None
        } else {
            nemesis_diff::schema::open_field(&self.pane.text, self.pane.cursor_line)
        };

        let language = NemesisXml {
            code_action: self.pane.lsp.code_action,
            events: &names.usable_events,
            variables: &names.usable_variables,
            variable_types: &names.variable_types,
            values: &values,
            graph: &names.graph,
            fields: context.as_ref().map(|(class, fields)| (*class, fields.as_slice())),
            pointer_field: open_field.filter(|field| nemesis_diff::schema::holds_pointers(field)),
            kept_array_hover: i18n.t(I18nKey::NemesisKeptArrayHover),
            in_string_array: open_field.is_some_and(nemesis_diff::schema::holds_strings),
            behavior,
        };
        let anim_language = AnimDataLanguage {
            roles: &self.pane.anim_roles,
            cursor_line: self.pane.cursor_line,
            trigger_names: &self.pane.trigger_names,
        };
        let pandora_language = PandoraXml { inner: &language };
        let shown_language: &dyn Language = if is_pandora_doc {
            &pandora_language
        } else if behavior.is_anim_data() {
            &anim_language
        } else {
            &language
        };

        let actions = palette_actions(&self.key_ui, i18n);
        let labels = EditorLabels::from_texts(gui_shared::i18n::editor_label_texts(i18n));
        let read_only = behavior.is_ref(id);
        let features = editor_features(&self.pane.lsp, behavior.is_anim_data());

        // Per object: each tab keeps where it was scrolled to.
        let scroll = egui::ScrollArea::both().id_salt(("nemesis_editor_scroll", id)).auto_shrink(false);
        scroll.show(ui, |ui| {
            let output = CodeEditor::new(("nemesis_object", self.is_user_patch, id), &mut self.pane.text)
                .language(shown_language)
                .read_only(read_only)
                .diagnostics(&self.pane.problems)
                .features(features)
                .keymap(keymap)
                .labels(labels)
                .host_commands(&HOST_COMMANDS)
                .requests(requests)
                .request_keys(self.key_ui.request_keys())
                .actions(&actions)
                .inline_diff(self.pane.show_inline_diff.then_some(&self.pane.inline_diff))
                .show(ui, &mut self.pane.state);

            if let Some(cursor) = output.cursor {
                self.pane.cursor_line = cursor.line;
            }
            if let Some(references) = read_output(output, result) {
                self.pane.references = Some(references);
            }
        });
    }

    /// What a click in the Problems panel or on the status bar's counts asked for.
    fn apply_problems(&mut self, toggle: bool, problems: ProblemsOutput) {
        if toggle {
            self.problems.open = !self.problems.open;
        }
        if problems.left {
            self.pane.focus = Some(FocusAt::Keep);
        }

        if let Some((behavior, object, line)) = problems.go_to {
            if behavior != self.active {
                self.active = behavior;
                self.selected = None;
            }
            let current = self.selected.clone().unwrap_or_default();
            self.open_location(&current, &object, line);
        }
    }
}

/// The palette's own actions, with their keys as the key bindings have them and a `⚙` to change
/// them, found by their English names and command ids too, as VS Code's.
fn palette_actions<'a>(key_ui: &'a keybinding::KeyUi, i18n: &'a I18nMap) -> [EditorAction<'a>; 10] {
    let act = |action: EditorAction<'static>, key: I18nKey| {
        key_ui.palette_action(EditorAction { label: i18n.t(key), ..action }, key.default_eng())
    };
    let new = |id: &'static str| EditorAction::new(id, "");
    [
        act(new(ACTION_FOCUS_TREE), I18nKey::NemesisFocusTree),
        act(new(ACTION_BACK), I18nKey::NemesisGoBack),
        act(new(ACTION_FORWARD), I18nKey::NemesisGoForward),
        act(EditorCommand::NextTab.action(""), I18nKey::EditorNextTab),
        act(EditorCommand::PreviousTab.action(""), I18nKey::EditorPreviousTab),
        act(EditorCommand::CloseTab.action(""), I18nKey::EditorCloseTab),
        act(EditorCommand::TogglePreview.action(""), I18nKey::EditorTogglePreview),
        act(EditorCommand::ToggleInlineDiff.action(""), I18nKey::NemesisToggleInlineDiff),
        act(EditorCommand::RevealFile.action(""), crate::patch::ui::object_list::REVEAL_LABEL),
        act(new(OPEN_JSON_ACTION), I18nKey::KeybindingOpenJson),
    ]
}

/// What the editor asked for this frame, into `result`. Returns the references to list, owned: they
/// are kept past this frame's borrows.
fn read_output(output: CodeEditorOutput<'_>, result: &mut EditorResult) -> Option<ReferenceList> {
    result.changed = output.changed;
    result.inline_revert = output.inline_revert;
    result.command = output.command;

    // The `⚙` of a palette action or an editor command: record its keys.
    result.configure = output.configure.and_then(host::command_of_action);
    if let Some(request) = output.configure_request {
        result.configure = host::command_of_request(request);
    }
    if output.action == Some(OPEN_JSON_ACTION) {
        result.configure = Some(OPEN_JSON_ACTION);
    }

    result.focus_tree = output.action == Some(ACTION_FOCUS_TREE);
    match output.action {
        Some(ACTION_BACK) => result.walk_history = Some(true),
        Some(ACTION_FORWARD) => result.walk_history = Some(false),
        _ => {}
    }

    match output.navigation {
        Some(Navigation::Definition(location)) => {
            result.definition = Some((location.document.into_owned(), location.line));
            None
        }
        Some(Navigation::References(references)) => Some(references.into()),
        None => None,
    }
}
