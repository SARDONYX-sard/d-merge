//! The events / variables panel: list, add, remove.
//!
//! ```text
//! [Objects] [Events / Variables]
//! [Events (279)] [Variables (78)]     one list at a time
//! [MyEvent      ] [Add]               (variables: [myVar] [REAL ▼] [0.0] [Add])
//! ───────────────
//!   277 REAL MyVar           ✎ ✖   <- added by this mod (can be renamed, removed)
//!   0   INT8 moveStart               (the type: variables only)
//! ```

use egui_widgets::shadcn::{Button, TextEdit};
use gui_shared::i18n::{I18nKey, I18nMap};
use nemesis_diff::vars::VariableType;
use stdx::text::contains_ignore_case;

use crate::patch::{
    PatchTool,
    vars_edit::{Action, GRAPH_DATA, Ids, Renaming, VALUE_SET},
    workspace::STRING_DATA,
};

impl PatchTool {
    pub(in crate::patch) fn vars_panel(&mut self, ui: &mut egui::Ui, i18n: &I18nMap) {
        let behavior = &self.workspace.behaviors[self.active];
        let ids = [STRING_DATA, GRAPH_DATA, VALUE_SET].map(|class| behavior.object_of_class(class).map(str::to_string));
        let [Some(string_data), Some(graph_data), value_set] = ids else {
            ui.weak(i18n.t(I18nKey::NemesisNoVars));
            return;
        };

        if let Some(error) = &self.vars_error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
        let names = self.names();
        let filter = self.explorer.filter.as_str();
        let (events, template_events) = (&names.events, names.template_events);
        let (variables, template_variables) = (&names.variables, names.template_variables);

        // `[Events (279)] [Variables (78)]`: one list at a time, its "add" row always on top.
        ui.horizontal(|ui| {
            for (variables_tab, key, count) in
                [(false, I18nKey::NemesisEvents, events.len()), (true, I18nKey::NemesisVariables, variables.len())]
            {
                let label = format!("{} ({count})", i18n.t(key));
                if ui.add(Button::selectable(self.show_variables == variables_tab, label)).clicked() {
                    self.show_variables = variables_tab;
                }
            }
        });

        let mut action = None;
        if self.show_variables {
            ui.horizontal(|ui| {
                let new = &mut self.new_names;
                let name = TextEdit::singleline(&mut new.variable)
                    .hint_text(i18n.t(I18nKey::NemesisNewVariableHint))
                    .desired_width(110.0);
                ui.add(name).on_hover_text(i18n.t(I18nKey::NemesisNewVariableHover));
                egui::ComboBox::from_id_salt("nemesis_var_type")
                    .width(70.0)
                    .selected_text(type_label(new.kind))
                    .show_ui(ui, |ui| {
                        for kind in VariableType::ALL {
                            ui.selectable_value(&mut new.kind, kind, type_label(kind));
                        }
                    })
                    .response
                    .on_hover_text(i18n.t(I18nKey::NemesisNewVariableTypeHover));
                ui.add(TextEdit::singleline(&mut new.value).hint_text("0").desired_width(50.0))
                    .on_hover_text(i18n.t(I18nKey::NemesisNewVariableValueHover));
                let enabled = value_set.is_some();
                if ui.add_enabled(enabled, Button::new(i18n.t(I18nKey::NemesisAdd)).small()).clicked() {
                    action = Some(Action::AddVariable);
                }
            });
            ui.separator();
            let list = NameList {
                names: variables,
                types: Some(&names.variable_types),
                template_count: template_variables,
                is_variable: true,
            };
            action = name_rows(ui, i18n, list, filter, &mut self.new_names.renaming).or(action);
        } else {
            ui.horizontal(|ui| {
                let name = TextEdit::singleline(&mut self.new_names.event)
                    .hint_text(i18n.t(I18nKey::NemesisNewEventHint))
                    .desired_width(140.0);
                ui.add(name).on_hover_text(i18n.t(I18nKey::NemesisNewEventHover));
                if ui.add(Button::new(i18n.t(I18nKey::NemesisAdd)).small()).clicked() {
                    action = Some(Action::AddEvent);
                }
            });
            ui.separator();
            let list = NameList { names: events, types: None, template_count: template_events, is_variable: false };
            action = name_rows(ui, i18n, list, filter, &mut self.new_names.renaming).or(action);
        }

        let Some(action) = action else { return };
        let ids = Ids { string_data, graph_data, value_set };
        match self.apply(action, &ids, events, variables) {
            Ok(changed) => {
                self.pane.opened = None; // the open object may be one of them
                for id in changed {
                    self.object_changed(&id);
                }
            }
            Err(message) => self.vars_error = Some(message),
        }
    }
}

/// One list of the panel.
#[derive(Debug, Clone, Copy)]
struct NameList<'a> {
    names: &'a [String],
    /// Variables: the type of each, by index.
    types: Option<&'a [Box<str>]>,
    /// How many belong to the template. Only the mod's own entries can be changed: other mods
    /// refer to the template's by name.
    template_count: usize,
    is_variable: bool,
}

/// `277 REAL MyVar ✎ ✖` in a scroll area that lays out only the visible rows (lists have ~1000
/// names). Newest first. Returns what a row asked for.
fn name_rows(
    ui: &mut egui::Ui,
    i18n: &I18nMap,
    list: NameList<'_>,
    filter: &str,
    renaming: &mut Option<Renaming>,
) -> Option<Action> {
    let shown: Vec<usize> =
        (0..list.names.len()).rev().filter(|&i| contains_ignore_case(&list.names[i], filter)).collect();
    let row_height = ui.spacing().interact_size.y.max(ui.text_style_height(&egui::TextStyle::Monospace) + 6.0);
    let mut action = None;
    // Sideways too: a long name widened the side bar, which takes its content's width.
    egui::ScrollArea::both().auto_shrink(false).show_rows(ui, row_height, shown.len(), |ui, range| {
        for &index in &shown[range] {
            ui.horizontal(|ui| action = name_row(ui, i18n, list, index, renaming).or(action));
        }
    });
    action
}

/// One row: the number, the type, then the name or, while renaming it, a field.
fn name_row(
    ui: &mut egui::Ui,
    i18n: &I18nMap,
    list: NameList<'_>,
    index: usize,
    renaming: &mut Option<Renaming>,
) -> Option<Action> {
    let NameList { names, types, template_count, is_variable } = list;
    ui.label(egui::RichText::new(format!("{index:>4}")).monospace().weak());
    if let Some(kind) = types.and_then(|types| types.get(index)) {
        ui.label(egui::RichText::new(format!("{kind:<7}")).monospace().color(type_color(ui)));
    }
    let is_being_renamed = renaming.as_ref().is_some_and(|r| r.is_variable == is_variable && r.index == index);
    if is_being_renamed {
        return rename_field(ui, renaming, is_variable, index);
    }
    let name = egui::RichText::new(&names[index]).monospace();
    // Cut off at the side bar's edge: the whole name on hover.
    let hover = egui_widgets::CutHover::new();
    if index < template_count {
        let label = ui.label(name);
        hover.show_text(ui, label, &names[index]);
        return None;
    }
    let label = ui.label(name.strong());
    hover.show_text(ui, label, &names[index]);
    if ui.small_button("✎").on_hover_text(i18n.t(I18nKey::NemesisRename)).clicked() {
        *renaming = Some(Renaming { is_variable, index, text: names[index].clone(), focus: true });
    }
    ui.small_button("✖").clicked().then_some(if is_variable {
        Action::RemoveVariable(index)
    } else {
        Action::RemoveEvent(index)
    })
}

/// The field of the name being renamed: `Enter` applies, leaving it any other way cancels.
fn rename_field(ui: &mut egui::Ui, renaming: &mut Option<Renaming>, is_variable: bool, index: usize) -> Option<Action> {
    let state = renaming.as_mut()?;
    let response = ui.add(TextEdit::singleline(&mut state.text).desired_width(160.0));
    if std::mem::take(&mut state.focus) {
        response.request_focus();
    }
    if !response.lost_focus() {
        return None;
    }
    if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
        return Some(Action::Rename { is_variable, index });
    }
    *renaming = None;
    None
}

/// The color of a variable's type, to tell it from the name at a glance.
fn type_color(ui: &egui::Ui) -> egui::Color32 {
    if ui.visuals().dark_mode { egui::Color32::from_rgb(86, 182, 194) } else { egui::Color32::from_rgb(0, 110, 130) }
}

const fn type_label(kind: VariableType) -> &'static str {
    match kind {
        VariableType::Bool => "BOOL",
        VariableType::Int8 => "INT8",
        VariableType::Int16 => "INT16",
        VariableType::Int32 => "INT32",
        VariableType::Real => "REAL",
    }
}
