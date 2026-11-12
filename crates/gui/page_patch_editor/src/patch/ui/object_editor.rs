//! The right side: the XML editor of the open object, and its patch preview below.

use egui_code_editor::{
    CodeEditor, EditorAction, EditorCommand, EditorFeatures, EditorLabels, InlineDiff, Keymap, Navigation,
    languages::Xml,
};
use egui_widgets::shadcn::{Button, Checkbox, Icon};
use gui_shared::{
    i18n::{I18nKey, I18nMap},
    settings::ui::editor::{EditorMode, LspOptions},
};

use super::{preview::PreviewEvent, references::references_window};
use crate::patch::{
    EditorResult, FocusAt, PatchTool, TabEvent,
    anim::lines::AnimDataLanguage,
    editor::{
        inline_diff,
        language::NemesisXml,
        pandora::{self, PandoraXml},
        resolve,
        tabs::EditorTabs,
    },
    tree::Graph,
    workspace::PANDORA_DOC,
};

/// The id of the palette's "focus the object tree" command.
const ACTION_FOCUS_TREE: &str = "focus_tree";
/// The ids of the palette's "go back" and "go forward".
const ACTION_BACK: &str = "go_back";
const ACTION_FORWARD: &str = "go_forward";
/// What this editor acts on: see [`crate::patch::PatchTool::apply_editor_result`].
const HOST_COMMANDS: [EditorCommand; 8] = [
    EditorCommand::NextTab,
    EditorCommand::PreviousTab,
    EditorCommand::CloseTab,
    EditorCommand::TogglePreview,
    EditorCommand::ToggleInlineDiff,
    EditorCommand::NextChange,
    EditorCommand::PreviousChange,
    EditorCommand::ToggleExplorer,
];

impl PatchTool {
    pub(in crate::patch) fn object_editor(&mut self, ui: &mut egui::Ui, i18n: &I18nMap) {
        let Some(id) = self.selected.clone() else {
            egui::CentralPanel::default().show(ui, |ui| ui.weak(i18n.t(I18nKey::NemesisSelectObject)));
            return;
        };
        self.open_in_editor(&id);
        let editor_id = egui_code_editor::editor_id(("nemesis_object", self.is_user_patch, &id));
        self.apply_focus_request(ui.ctx(), editor_id);
        // Like VS Code: getting into the editor shows its object in the side bar.
        let has_focus = ui.memory(|m| m.has_focus(editor_id));
        self.note_editor_focus(ui.ctx(), has_focus);
        // `Ctrl+W` closes the tab, as in VS Code. (Vim uses that key itself.)
        if self.editor_mode == EditorMode::Normal
            && ui.memory(|m| m.has_focus(editor_id))
            && ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::W))
        {
            self.apply_tab_event(Some(TabEvent::Close(id.to_string())), &id);
            return;
        }
        self.expire_preview(ui.ctx());
        self.refresh_preview(&id);
        if let Some(older) = history_keys(ui) {
            self.walk_history(older, &id);
            ui.ctx().request_repaint();
            return;
        }
        // `Ctrl+PageDown` / `Ctrl+PageUp`, as in VS Code, in every keymap.
        if let Some(forward) = egui_code_editor::tab_switch_keys(ui) {
            self.step_tab(&id, forward);
            ui.ctx().request_repaint();
            return;
        }

        // No preview for a new object: it is written as is.
        let behavior = &self.workspace.behaviors[self.active];
        let is_new = behavior.added.contains_key(&*id) || behavior.is_ref(&id) || *id == *PANDORA_DOC;
        // What the mods have at the conflict being edited: the preview comes back when it is closed.
        let is_merging = self.merge_values.as_ref().is_some_and(|values| *values.id == *id);
        let preview_event = if is_merging {
            self.merge_panel(ui, i18n, &id);
            None
        } else if is_new || !self.show_preview {
            None
        } else {
            self.preview_panel(ui, i18n)
        };
        let result = self.editor_panel(ui, i18n, &id);

        if let Some(view) = &mut self.preview_view {
            if self.follow_preview {
                view.follow_editor_line(self.cursor_line, &self.editor_text);
            } else {
                view.stop_following();
            }
        }
        if self.apply_editor_result(result, &id) {
            self.enter_tree(ui, editor_id);
        }
        if let Some((document, line)) = references_window(ui.ctx(), &mut self.references) {
            if *document != *id {
                self.history.visit(&id);
                self.selected = Some(document.into());
            }
            self.focus_editor = Some(FocusAt::Line(line));
        }
        match preview_event {
            Some(PreviewEvent::Revert(block)) => self.revert_block(&id, block),
            Some(PreviewEvent::JumpToEditor(row)) => {
                let line = self.preview_view.as_ref().and_then(|view| view.editor_line_of(row, &self.editor_text));
                self.focus_editor = line.map(FocusAt::Line);
            }
            None => {}
        }
    }

    /// See [`resolve::MergeValues`]. What is picked goes below the cursor in the editor.
    fn merge_panel(&mut self, ui: &mut egui::Ui, i18n: &I18nMap, id: &str) {
        let Some(values) = self.merge_values.take() else { return };
        let (mut picked, mut close) = (None, false);
        let panel_id = if self.is_user_patch { "user_patch_merge" } else { "nemesis_merge" };
        egui::Panel::right(panel_id).default_size(520.0).resizable(true).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.strong(i18n.t(I18nKey::NemesisMergeTitle)).on_hover_text(i18n.t(I18nKey::NemesisMergeHover));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    close = ui.small_button("✖").clicked();
                });
            });
            ui.add(egui::Label::new(egui::RichText::new(&values.path).monospace().weak()).truncate());
            ui.separator();
            egui::ScrollArea::both().id_salt(("nemesis_merge_values", id)).auto_shrink(false).show(ui, |ui| {
                picked = merge_values_ui(ui, i18n, &values);
            });
        });
        if let Some(lines) = picked {
            let (text, last) = resolve::insert_lines(&self.editor_text, self.cursor_line, &lines);
            self.editor_text = text;
            self.workspace.behaviors[self.active].set_text(id, &self.editor_text);
            self.preview = None;
            self.object_changed(id);
            self.focus_editor = Some(FocusAt::Line(last));
        }
        if !close {
            self.merge_values = Some(values);
        }
    }

    /// `Patch preview ▲ ▼ 2/5 ✔ ... [follow]` and its lines.
    fn preview_panel(&mut self, ui: &mut egui::Ui, i18n: &I18nMap) -> Option<PreviewEvent> {
        let mut event = None;
        let panel_id = if self.is_user_patch { "user_patch_preview" } else { "nemesis_preview" };
        let is_pandora = self.preview_view.as_ref().is_some_and(|view| view.is_pandora());
        egui::Panel::right(panel_id).default_size(520.0).resizable(true).show(ui, |ui| {
            let Some((patch, check)) = &self.preview else { return };
            ui.horizontal(|ui| {
                ui.strong(i18n.t(I18nKey::NemesisPatchPreview));
                if let Some(view) = &mut self.preview_view {
                    super::preview::nav_ui(view, ui, i18n.t(I18nKey::NemesisJumpDiffHover));
                }
                match check {
                    _ if patch.is_none() => {
                        ui.weak(i18n.t(I18nKey::NemesisNoChange));
                    }
                    Ok(()) => {
                        let ok = if is_pandora { I18nKey::NemesisPandoraOpsOk } else { I18nKey::NemesisParserOk };
                        ui.colored_label(egui::Color32::from_rgb(100, 200, 120), "✔").on_hover_text(i18n.t(ok));
                    }
                    // The details are in the Problems panel: a long text here would widen this panel.
                    Err(err) => {
                        ui.colored_label(ui.visuals().error_fg_color, "✖").on_hover_text(err);
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let follow = Button::selectable(self.follow_preview, "").icon(Icon::Crosshair).small();
                    if ui.add(follow).on_hover_text(i18n.t(I18nKey::NemesisFollowCursor)).clicked() {
                        self.follow_preview = !self.follow_preview;
                    }
                });
            });
            if let Some(view) = &mut self.preview_view {
                event = super::preview::lines_ui(view, ui, i18n.t(I18nKey::NemesisRevertHunkHover));
            }
        });
        event
    }

    /// The toolbar, the editor and the Vim status line. Returns what the editor asked for: it is
    /// done afterwards, as the language still borrows the behavior here.
    fn editor_panel(&mut self, ui: &mut egui::Ui, i18n: &I18nMap, id: &str) -> EditorResult {
        let names = self.names();
        let values = self.field_values();
        // The class (and fields) around the cursor line of last frame: field names / hover.
        let is_pandora_doc = id == PANDORA_DOC;
        let context = if is_pandora_doc {
            pandora::fields_at(&self.workspace.behaviors[self.active], &self.editor_text, self.cursor_line)
        } else {
            nemesis_diff::schema::field_context(&self.editor_text, self.cursor_line)
        };
        let open_field =
            if is_pandora_doc { None } else { nemesis_diff::schema::open_field(&self.editor_text, self.cursor_line) };
        let language = NemesisXml {
            code_action: self.lsp.code_action,
            events: &names.usable_events,
            variables: &names.usable_variables,
            variable_types: &names.variable_types,
            values: &values,
            graph: &names.graph,
            fields: context.as_ref().map(|(class, fields)| (*class, fields.as_slice())),
            pointer_field: open_field.filter(|field| nemesis_diff::schema::holds_pointers(field)),
            kept_array_hover: i18n.t(I18nKey::NemesisKeptArrayHover),
            in_string_array: open_field.is_some_and(nemesis_diff::schema::holds_strings),
            // After name, id, speed, crop start, crop end and the count.
            behavior: &self.workspace.behaviors[self.active],
        };
        let anim_data = self.workspace.behaviors[self.active].is_anim_data();
        let anim_language = AnimDataLanguage {
            roles: &self.anim_roles,
            cursor_line: self.cursor_line,
            trigger_names: &self.trigger_names,
        };
        let pandora_language = PandoraXml { inner: &language };
        let shown_language: &dyn egui_code_editor::Language = if is_pandora_doc {
            &pandora_language
        } else if anim_data {
            &anim_language
        } else {
            &language
        };
        let features = editor_features(&self.lsp, anim_data);
        let keymap = match self.editor_mode {
            EditorMode::Normal => Keymap::Normal,
            EditorMode::Vim => Keymap::Vim,
        };
        let labels = EditorLabels::from_texts(gui_shared::i18n::editor_label_texts(i18n));
        let actions = [
            EditorAction { id: ACTION_FOCUS_TREE, label: i18n.t(I18nKey::NemesisFocusTree) },
            EditorAction { id: ACTION_BACK, label: i18n.t(I18nKey::NemesisGoBack) },
            EditorAction { id: ACTION_FORWARD, label: i18n.t(I18nKey::NemesisGoForward) },
            EditorCommand::NextTab.action(i18n.t(I18nKey::EditorNextTab)),
            EditorCommand::PreviousTab.action(i18n.t(I18nKey::EditorPreviousTab)),
            EditorCommand::CloseTab.action(i18n.t(I18nKey::EditorCloseTab)),
            EditorCommand::TogglePreview.action(i18n.t(I18nKey::EditorTogglePreview)),
            EditorCommand::ToggleInlineDiff.action(i18n.t(I18nKey::NemesisToggleInlineDiff)),
        ];

        let mut result = EditorResult::default();
        // Read before the panel borrows the work; what a click asks for is done after it.
        let (counts, mut toggle_problems, mut go_to_problem) = (self.problem_counts(), false, None);
        egui::CentralPanel::default().show(ui, |ui| {
            // Like VS Code's: the tabs, and at the far right what splits the editor. A new object has no
            // preview: it is written as is.
            let behavior = &self.workspace.behaviors[self.active];
            let can_split = !behavior.added.contains_key(id) && !behavior.is_ref(id) && !is_pandora_doc;
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if can_split {
                        split_preview_button(ui, i18n, &mut self.show_preview);
                    }
                    let inline = Button::selectable(self.show_inline_diff, "").icon(Icon::Diff).small();
                    if ui.add(inline).on_hover_text(i18n.t(I18nKey::NemesisInlineDiffHover)).clicked() {
                        self.show_inline_diff = !self.show_inline_diff;
                    }
                    if self.show_inline_diff
                        && let Some(line) = inline_diff_nav(ui, i18n, &self.inline_diff, self.cursor_line)
                    {
                        self.focus_editor = Some(FocusAt::Line(line));
                    }
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        result.walk_history = history_buttons(ui, i18n, &self.history);
                        result.tab_event = tab_bar(ui, i18n, &self.tabs, self.active, id, &names.graph);
                    });
                });
            });
            let read_only = self.workspace.behaviors[self.active].is_ref(id);
            if read_only {
                ui.weak(i18n.t(I18nKey::NemesisRefReadOnly));
            }
            if self.show_lsp_options {
                lsp_options_window(ui.ctx(), i18n, &mut self.lsp, &mut self.show_lsp_options);
            }
            // The editor's settings are in its status bar, as in VS Code.
            let status_id = if self.is_user_patch { "user_patch_status_bar" } else { "nemesis_status_bar" };
            egui::Panel::bottom(status_id).show(ui, |ui| {
                ui.horizontal(|ui| {
                    toggle_problems |= problem_counts(ui, i18n, counts);
                    toolbar(ui, i18n, &mut self.editor_mode, &mut self.show_lsp_options);
                    egui_code_editor::status_line(ui, &self.editor, keymap, &self.editor_text);
                });
            });
            if self.problems_open
                && let Some(problems) = &self.problems
            {
                go_to_problem = super::problems::problems_panel(ui, i18n, problems, &self.workspace.behaviors);
            }
            // Per object: each tab keeps where it was scrolled to.
            egui::ScrollArea::both().id_salt(("nemesis_editor_scroll", id)).auto_shrink(false).show(ui, |ui| {
                let output = CodeEditor::new(("nemesis_object", self.is_user_patch, id), &mut self.editor_text)
                    .language(shown_language)
                    .read_only(read_only)
                    .diagnostics(&self.field_problems)
                    .features(features)
                    .keymap(keymap)
                    .labels(labels)
                    .host_commands(&HOST_COMMANDS)
                    .actions(&actions)
                    .inline_diff(self.show_inline_diff.then_some(&self.inline_diff))
                    .show(ui, &mut self.editor);
                if let Some(cursor) = output.cursor {
                    self.cursor_line = cursor.line;
                }
                result.changed = output.changed;
                result.inline_revert = output.inline_revert;
                result.command = output.command;
                result.focus_tree = output.action == Some(ACTION_FOCUS_TREE);
                match output.action {
                    Some(ACTION_BACK) => result.walk_history = Some(true),
                    Some(ACTION_FORWARD) => result.walk_history = Some(false),
                    _ => {}
                }
                // Kept past the borrows of this frame, so owned here.
                match output.navigation {
                    Some(Navigation::Definition(location)) => {
                        result.definition = Some((location.document.into_owned(), location.line));
                    }
                    Some(Navigation::References(references)) => self.references = Some(references.into()),
                    None => {}
                }
            });
        });
        if toggle_problems {
            self.problems_open = !self.problems_open;
        }
        if let Some((behavior, object, line)) = go_to_problem {
            if behavior != self.active {
                self.active = behavior;
                self.selected = None;
            }
            let current = self.selected.clone().unwrap_or_default();
            self.open_location(&current, &object, line);
        }
        result
    }
}

/// The original lines, then each mod's with `Insert`, for all of them and for each line. Returns the
/// lines picked.
fn merge_values_ui(ui: &mut egui::Ui, i18n: &I18nMap, values: &resolve::MergeValues) -> Option<Vec<String>> {
    let line_of = |ui: &egui::Ui, line: &str| egui_code_editor::line_job(ui, line.trim_start(), &Xml);
    ui.weak(i18n.t(I18nKey::NemesisMergeOriginal));
    for line in &values.original {
        ui.add(egui::Label::new(line_of(ui, line)));
    }
    let mut picked = None;
    for side in &values.sides {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.strong(&side.name);
            let insert =
                ui.add_enabled(!side.lines.is_empty(), Button::new(i18n.t(I18nKey::NemesisMergeInsert)).small());
            if insert.on_hover_text(i18n.t(I18nKey::NemesisMergeInsertHover)).clicked() {
                picked = Some(side.lines.clone());
            }
        });
        if side.lines.is_empty() {
            ui.weak(i18n.t(I18nKey::NemesisMergeRemoves));
        }
        // Several lines: one of them may be all that is wanted.
        for line in side.lines.iter().filter(|_| side.lines.len() > 1) {
            ui.horizontal(|ui| {
                if ui.small_button("+").on_hover_text(i18n.t(I18nKey::NemesisMergeInsertHover)).clicked() {
                    picked = Some(vec![line.clone()]);
                }
                ui.add(egui::Label::new(line_of(ui, line)));
            });
        }
        if let [line] = side.lines.as_slice() {
            ui.add(egui::Label::new(line_of(ui, line)));
        }
    }
    picked
}

/// `▲ ▼ 2/5` of the changes shown in the editor, drawn right to left. Returns the line to go to.
fn inline_diff_nav(ui: &mut egui::Ui, i18n: &I18nMap, diff: &InlineDiff, cursor_line: usize) -> Option<usize> {
    let (at, count) = inline_diff::position(diff, cursor_line)?;
    ui.label(format!("{}/{count}", at + 1));
    let mut line = None;
    for (forward, glyph) in [(true, "▼"), (false, "▲")] {
        if ui.small_button(glyph).on_hover_text(i18n.t(I18nKey::NemesisJumpDiffHover)).clicked() {
            line = inline_diff::step(diff, cursor_line, forward);
        }
    }
    line
}

/// Shows or hides the patch preview beside the editor.
fn split_preview_button(ui: &mut egui::Ui, i18n: &I18nMap, show_preview: &mut bool) {
    let hover = if *show_preview { I18nKey::EditorHidePreview } else { I18nKey::EditorShowPreview };
    let button = Button::selectable(*show_preview, "").icon(Icon::Columns2).small();
    if ui.add(button).on_hover_text(i18n.t(hover)).clicked() {
        *show_preview = !*show_preview;
    }
}

/// `Alt+←` or the mouse's back button: `Some(true)`. `Alt+→` or its forward button: `Some(false)`.
/// As in VS Code, in every keymap: going to a definition is only useful if you can come back.
fn history_keys(ui: &egui::Ui) -> Option<bool> {
    use egui::{Key, Modifiers, PointerButton};
    ui.input_mut(|input| {
        let back =
            input.consume_key(Modifiers::ALT, Key::ArrowLeft) || input.pointer.button_pressed(PointerButton::Extra1);
        let forward =
            input.consume_key(Modifiers::ALT, Key::ArrowRight) || input.pointer.button_pressed(PointerButton::Extra2);
        if back {
            Some(true)
        } else if forward {
            Some(false)
        } else {
            None
        }
    })
}

/// `<-` `->` at the left of the tabs, which scroll without them. Returns `Some(true)` to go back,
/// `Some(false)` to go forward. They are for those who don't know `Alt+Left` / `Alt+Right`, so their
/// hovers tell.
fn history_buttons(
    ui: &mut egui::Ui,
    i18n: &I18nMap,
    history: &crate::patch::editor::navigate::History,
) -> Option<bool> {
    let buttons = [
        (true, Icon::ArrowLeft, I18nKey::NemesisGoBack, "Alt+←"),
        (false, Icon::ArrowRight, I18nKey::NemesisGoForward, "Alt+→"),
    ];
    let mut walk = None;
    for (older, icon, label, keys) in buttons {
        let button = Button::new("").icon(icon).small();
        let response = ui.add_enabled(history.can_walk(older), button);
        let response = response.on_hover_text(format!("{} ({keys})", i18n.t(label)));
        if response.clicked() {
            walk = Some(older);
        }
    }
    walk
}

/// `[ GetUpModList ✖ ] [ ● Root ✖ ]`. See [`crate::patch::editor::tabs`].
///
/// Only the name, to keep the tabs short: the hover has the whole label, and the tree follows the tab.
/// A click shows a tab, a double click pins it, a middle click or `✖` closes it.
fn tab_bar(
    ui: &mut egui::Ui,
    i18n: &I18nMap,
    tabs: &EditorTabs,
    behavior: usize,
    current: &str,
    graph: &Graph,
) -> Option<TabEvent> {
    let mut event = None;
    egui::ScrollArea::horizontal().id_salt("nemesis_editor_tabs").show(ui, |ui| {
        ui.horizontal(|ui| {
            for tab in tabs.of(behavior) {
                let node = graph.node_of(&tab.id);
                let marker = if node.is_some_and(|node| graph.is_changed(node)) { "● " } else { "" };
                let name = tab_name(node.map_or("", |node| graph.short_name(node)), &tab.id);
                let text = egui::RichText::new(format!("{marker}{name}")).small();
                // Italic: the preview tab, which the next opened object replaces.
                let text = if tab.pinned { text } else { text.italics() };
                let hover = node.map_or(tab.id.as_str(), |node| graph.label(node));
                let response = ui.add(Button::selectable(tab.id == current, text)).on_hover_text(hover);
                if response.clicked() {
                    event = Some(TabEvent::Show(tab.id.clone()));
                }
                if response.double_clicked() {
                    event = Some(TabEvent::KeepOpen(tab.id.clone()));
                }
                if response.middle_clicked() || ui.small_button("✖").clicked() {
                    event = Some(TabEvent::Close(tab.id.clone()));
                }
                response.context_menu(|ui| event = tab_menu(ui, i18n, &tab.id, tab.pinned).or_else(|| event.take()));
            }
        });
    });
    event
}

/// A tab's right-click menu, like VS Code's.
fn tab_menu(ui: &mut egui::Ui, i18n: &I18nMap, id: &str, pinned: bool) -> Option<TabEvent> {
    let items = [
        (I18nKey::NemesisKeepOpen, !pinned, TabEvent::KeepOpen(id.to_owned())),
        (I18nKey::CloseButton, true, TabEvent::Close(id.to_owned())),
        (I18nKey::NemesisCloseOtherTabs, true, TabEvent::CloseOthers(id.to_owned())),
        (I18nKey::NemesisCloseTabsRight, true, TabEvent::CloseToTheRight(id.to_owned())),
        (I18nKey::NemesisCloseAllTabs, true, TabEvent::CloseAll),
    ];
    let mut picked = None;
    for (label, enabled, event) in items {
        if ui.add_enabled(enabled, egui::Button::new(i18n.t(label))).clicked() {
            picked = Some(event);
            ui.close();
        }
    }
    ui.separator();
    if ui.button(i18n.t(I18nKey::NemesisCopyId)).clicked() {
        ui.ctx().copy_text(id.to_owned());
        ui.close();
    }
    picked
}

/// What a tab says: the object's name, or the last part of the id of what has none, such as a file of
/// the animation data.
///
/// e.g., `("GetUpModList", "#0106")` -> `GetUpModList`, `("", "AData~A/m$New.txt")` -> `m$New.txt`
fn tab_name<'a>(name: &'a str, id: &'a str) -> &'a str {
    if name.is_empty() { id.rsplit('/').next().unwrap_or(id) } else { name }
}

/// The language features the settings turn on. The lines of the animation data have no form to
/// format them to.
const fn editor_features(lsp: &LspOptions, anim_data: bool) -> EditorFeatures {
    EditorFeatures {
        completion: lsp.completion,
        hover: lsp.hover,
        diagnostics: lsp.diagnostics,
        format: lsp.formatter && !anim_data,
        inlay_hints: lsp.inlay_hints,
        signature_help: lsp.signature_help,
    }
}

/// `[Vim ▼] ⚙`. `⚙` has the same language switches as the hkanno editor's.
fn toolbar(ui: &mut egui::Ui, i18n: &I18nMap, mode: &mut EditorMode, show_options: &mut bool) {
    ui.horizontal(|ui| {
        let mut index = EditorMode::ALL.iter().position(|m| m == mode).unwrap_or(0);
        if egui::ComboBox::from_id_salt("nemesis_editor_mode")
            .width(80.0)
            .show_index(ui, &mut index, EditorMode::ALL.len(), |i| EditorMode::ALL[i].label())
            .changed()
        {
            *mode = EditorMode::ALL[index];
        }
        if ui
            .add(Button::selectable(*show_options, "⚙").small())
            .on_hover_text(i18n.t(I18nKey::HkannoLspOptionsTitle))
            .clicked()
        {
            *show_options = !*show_options;
        }
    });
}

/// The language features that apply to the object editor (the hkanno options window's subset).
fn lsp_options_window(ctx: &egui::Context, i18n: &I18nMap, lsp: &mut LspOptions, open: &mut bool) {
    egui_widgets::window_near_pointer(ctx, i18n.t(I18nKey::HkannoLspOptionsTitle))
        .open(open)
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| {
            let options = [
                (&mut lsp.completion, I18nKey::LspCompletion, I18nKey::LspCompletionHover),
                (&mut lsp.code_action, I18nKey::LspCodeAction, I18nKey::LspCodeActionHover),
                (&mut lsp.diagnostics, I18nKey::LspDiagnostics, I18nKey::LspDiagnosticsHover),
                (&mut lsp.formatter, I18nKey::LspFormatter, I18nKey::LspFormatterHover),
                (&mut lsp.hover, I18nKey::LspHover, I18nKey::LspHoverHover),
                (&mut lsp.inlay_hints, I18nKey::LspInlayHints, I18nKey::LspInlayHintsHover),
                (&mut lsp.signature_help, I18nKey::LspSignatureHelp, I18nKey::LspSignatureHelpHover),
            ];
            for (value, label, hover) in options {
                ui.add(Checkbox::new(value, i18n.t(label))).on_hover_text(i18n.t(hover));
            }
        });
}

/// `✖ 2  ⚠ 5` at the left of the status bar, as VS Code's: the problems of the whole work (`counts`:
/// errors, warnings; `None` until it was checked). Returns whether it was clicked: that shows or hides the Problems panel.
fn problem_counts(ui: &mut egui::Ui, i18n: &I18nMap, counts: Option<(usize, usize)>) -> bool {
    let Some((errors, warnings)) = counts else { return false };
    let visuals = ui.visuals();
    let (error_color, warn_color) = if errors + warnings == 0 {
        (visuals.weak_text_color(), visuals.weak_text_color())
    } else {
        (visuals.error_fg_color, visuals.warn_fg_color)
    };
    let mut job = egui::text::LayoutJob::default();
    let font = egui::TextStyle::Body.resolve(ui.style());
    job.append(&format!("✖ {errors}"), 0.0, egui::TextFormat::simple(font.clone(), error_color));
    job.append(&format!("  ⚠ {warnings}"), 0.0, egui::TextFormat::simple(font, warn_color));
    let clicked =
        ui.add(egui::Button::new(job).frame(false)).on_hover_text(i18n.t(I18nKey::HkannoProblemsLabel)).clicked();
    ui.separator();
    clicked
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tab_says_the_name() {
        assert_eq!(tab_name("GetUpModList", "#0106"), "GetUpModList");
        assert_eq!(tab_name("", "#0106"), "#0106");
        assert_eq!(tab_name("", "AData~A/m$New.txt"), "m$New.txt");
    }
}
