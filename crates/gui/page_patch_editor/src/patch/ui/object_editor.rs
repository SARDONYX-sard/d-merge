//! The right side: the XML editor of the open object, and its patch preview below.

use egui_code_editor::{CodeEditor, EditorAction, EditorFeatures, EditorLabels, Keymap, Navigation};
use egui_widgets::shadcn::{Button, Checkbox, Icon};
use gui_shared::{
    i18n::{I18nKey, I18nMap},
    settings::ui::editor::{EditorMode, LspOptions},
};

use super::{preview::PreviewEvent, references::references_window};
use crate::patch::{
    EditorResult, FocusAt, PatchTool, TabEvent, anim_lines::AnimDataLanguage, editor_tabs::EditorTabs,
    language::NemesisXml, tree::Graph,
};

/// The id of the palette's "focus the object tree" command.
const ACTION_FOCUS_TREE: &str = "focus_tree";
/// The ids of the palette's "go back" and "go forward".
const ACTION_BACK: &str = "go_back";
const ACTION_FORWARD: &str = "go_forward";

impl PatchTool {
    pub(in crate::patch) fn object_editor(&mut self, ui: &mut egui::Ui, i18n: &I18nMap) {
        let Some(id) = self.selected.clone() else {
            egui::CentralPanel::default().show(ui, |ui| ui.weak(i18n.t(I18nKey::NemesisSelectObject)));
            return;
        };
        self.open_in_editor(&id);
        let editor_id = egui_code_editor::editor_id(("nemesis_object", &id));
        self.apply_focus_request(ui.ctx(), editor_id);
        // `Ctrl+W` closes the tab, as in VS Code. (Vim uses that key itself.)
        if self.editor_mode == EditorMode::Normal
            && ui.memory(|m| m.has_focus(editor_id))
            && ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::W))
        {
            self.apply_tab_event(Some(TabEvent::Close(id.clone())), &id);
            return;
        }
        self.expire_preview(ui.ctx());
        self.refresh_preview(&id);
        if let Some(older) = history_keys(ui) {
            self.walk_history(older, &id);
            ui.ctx().request_repaint();
            return;
        }

        // No preview for a new object: it is written as is.
        let behavior = &self.workspace.behaviors[self.active];
        let is_new = behavior.added.contains_key(&id) || behavior.is_ref(&id);
        let preview_event = if is_new || !self.show_preview { None } else { self.preview_panel(ui, i18n) };
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
            if document != id {
                self.history.visit(&id);
                document.clone_into(self.selected.get_or_insert_default());
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

    /// `Patch preview ▲ ▼ 2/5 ✔ ... [follow]` and its lines.
    fn preview_panel(&mut self, ui: &mut egui::Ui, i18n: &I18nMap) -> Option<PreviewEvent> {
        let mut event = None;
        egui::Panel::right("nemesis_preview").default_size(520.0).resizable(true).show(ui, |ui| {
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
                        ui.colored_label(egui::Color32::from_rgb(100, 200, 120), "✔")
                            .on_hover_text(i18n.t(I18nKey::NemesisParserOk));
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
        let context = nemesis_diff::schema::field_context(&self.editor_text, self.cursor_line);
        let language = NemesisXml {
            events: &names.usable_events,
            variables: &names.usable_variables,
            variable_types: &names.variable_types,
            values: &values,
            graph: &names.graph,
            fields: context.as_ref().map(|(class, fields)| (class.as_str(), fields.as_slice())),
            // After name, id, speed, crop start, crop end and the count.
            behavior: &self.workspace.behaviors[self.active],
        };
        let anim_data = self.workspace.behaviors[self.active].is_anim_data();
        let anim_language = AnimDataLanguage {
            roles: &self.anim_roles,
            cursor_line: self.cursor_line,
            trigger_names: &self.trigger_names,
        };
        let shown_language: &dyn egui_code_editor::Language = if anim_data { &anim_language } else { &language };
        let features = EditorFeatures { inlay_hints: anim_data, ..editor_features(&self.lsp) };
        let keymap = match self.editor_mode {
            EditorMode::Normal => Keymap::Normal,
            EditorMode::Vim => Keymap::Vim,
        };
        let labels = EditorLabels::from_texts(gui_shared::i18n::editor_label_texts(i18n));
        let actions = [
            EditorAction { id: ACTION_FOCUS_TREE, label: i18n.t(I18nKey::NemesisFocusTree) },
            EditorAction { id: ACTION_BACK, label: i18n.t(I18nKey::NemesisGoBack) },
            EditorAction { id: ACTION_FORWARD, label: i18n.t(I18nKey::NemesisGoForward) },
        ];

        let mut result = EditorResult::default();
        egui::CentralPanel::default().show(ui, |ui| {
            result.tab_event = tab_bar(ui, i18n, &self.tabs, self.active, id, &names.graph);
            let read_only = self.workspace.behaviors[self.active].is_ref(id);
            if read_only {
                ui.weak(i18n.t(I18nKey::NemesisRefReadOnly));
            }
            if self.show_lsp_options {
                lsp_options_window(ui.ctx(), i18n, &mut self.lsp, &mut self.show_lsp_options);
            }
            // The editor's settings are in its status bar, as in VS Code.
            egui::Panel::bottom("nemesis_status_bar").show(ui, |ui| {
                ui.horizontal(|ui| {
                    toolbar(ui, i18n, &mut self.editor_mode, &mut self.show_lsp_options);
                    let diff = Button::selectable(self.show_inline_diff, "").icon(Icon::Diff).small();
                    if ui.add(diff).on_hover_text(i18n.t(I18nKey::NemesisInlineDiffHover)).clicked() {
                        self.show_inline_diff = !self.show_inline_diff;
                    }
                    let diff_split = Button::selectable(self.show_preview, "").icon(Icon::Diff).small();
                    if ui
                        .add(diff_split)
                        .on_hover_text({
                            if self.show_preview {
                                i18n.t(I18nKey::EditorHidePreview)
                            } else {
                                i18n.t(I18nKey::EditorShowPreview)
                            }
                        })
                        .clicked()
                    {
                        self.show_preview = !self.show_preview;
                    }
                    egui_code_editor::status_line(ui, &self.editor, keymap, &self.editor_text);
                });
            });
            result.jump_to_line = super::problems::problems_panel(ui, i18n, &self.field_problems);
            // Per object: each tab keeps where it was scrolled to.
            egui::ScrollArea::both().id_salt(("nemesis_editor_scroll", id)).auto_shrink(false).show(ui, |ui| {
                let output = CodeEditor::new(("nemesis_object", id), &mut self.editor_text)
                    .language(shown_language)
                    .read_only(read_only)
                    .diagnostics(&self.field_problems)
                    .features(features)
                    .keymap(keymap)
                    .labels(labels)
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
                result.walk_history = match output.action {
                    Some(ACTION_BACK) => Some(true),
                    Some(ACTION_FORWARD) => Some(false),
                    _ => None,
                };
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
        result
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

/// `[ #0106 GetUpModList ✖ ] [ ● #0200 Root ✖ ]`. See [`crate::patch::editor_tabs`].
///
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
                let name = node.map_or("", |node| graph.short_name(node));
                let text = egui::RichText::new(format!("{marker}{} {name}", tab.id)).small();
                // Italic: the preview tab, which the next opened object replaces.
                let text = if tab.pinned { text } else { text.italics() };
                let response =
                    ui.add(Button::selectable(tab.id == current, text)).on_hover_text(graph_label(graph, node));
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
        (I18nKey::NemesisCloseTab, true, TabEvent::Close(id.to_owned())),
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

/// The whole label of a node, for the tab's hover.
fn graph_label(graph: &Graph, node: Option<crate::patch::tree::Node>) -> String {
    node.map_or_else(String::new, |node| graph.label(node).to_owned())
}

/// The language features the settings turn on, and what XML has no use for.
const fn editor_features(lsp: &LspOptions) -> EditorFeatures {
    EditorFeatures {
        completion: lsp.completion,
        hover: lsp.hover,
        diagnostics: lsp.diagnostics,
        format: false,
        inlay_hints: false,
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
                (&mut lsp.diagnostics, I18nKey::LspDiagnostics, I18nKey::LspDiagnosticsHover),
                (&mut lsp.hover, I18nKey::LspHover, I18nKey::LspHoverHover),
                (&mut lsp.signature_help, I18nKey::LspSignatureHelp, I18nKey::LspSignatureHelpHover),
            ];
            for (value, label, hover) in options {
                ui.add(Checkbox::new(value, i18n.t(label))).on_hover_text(i18n.t(hover));
            }
        });
}
