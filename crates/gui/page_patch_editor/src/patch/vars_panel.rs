//! Events / variables of the active behavior: list, add, remove (the ones this mod added).
//!
//! ```text
//! [Objects] [Events / Variables]
//! [Events (279)] [Variables (78)]     one list at a time
//! [MyEvent      ] [Add]               (variables: [myVar] [REAL ▼] [0.0] [Add])
//! ───────────────
//!   277 MyEvent                ✖   <- added by this mod (removable)
//!   0   moveStart
//! ```
//! Adding edits the string data, graph data (and value set) objects at once; see
//! [`nemesis_diff::vars`]. Template entries cannot be removed (other mods index them).

use egui_widgets::shadcn::{Button, TextEdit};
use gui_shared::i18n::{I18nKey, I18nMap};
use nemesis_diff::vars::{self, VariableType};

use super::PatchTool;

const STRING_DATA: &str = "hkbBehaviorGraphStringData";
const GRAPH_DATA: &str = "hkbBehaviorGraphData";
const VALUE_SET: &str = "hkbVariableValueSet";

/// The active behavior's names, cached until the next edit.
#[derive(Debug, Default)]
pub(super) struct Names {
    pub(super) events: Vec<String>,
    pub(super) variables: Vec<String>,
    /// How many of each the template has (the rest are this mod's).
    template_events: usize,
    template_variables: usize,
    /// `(id, "class name")` for the `#` completion (new objects change it too).
    pub(super) pointers: Vec<(String, String)>,
}

/// Inputs of the "add" rows.
#[derive(Debug)]
pub(super) struct NewNames {
    event: String,
    variable: String,
    kind: VariableType,
    value: String,
}

impl Default for NewNames {
    fn default() -> Self {
        Self {
            event: String::new(),
            variable: String::new(),
            kind: VariableType::Int32,
            value: String::new(),
        }
    }
}

impl PatchTool {
    /// The active behavior's events / variables (recomputed after edits).
    pub(super) fn names(&mut self) -> &Names {
        let active = self.active;
        if self.names.as_ref().is_some_and(|(cached, _)| *cached != active) {
            self.names = None;
        }
        let behavior = &self.workspace.behaviors[active];
        &self.names.get_or_insert_with(|| (active, names_of(behavior))).1
    }

    pub(super) fn vars_panel(&mut self, ui: &mut egui::Ui, i18n: &I18nMap) {
        let behavior = &self.workspace.behaviors[self.active];
        let ids = [STRING_DATA, GRAPH_DATA, VALUE_SET]
            .map(|class| behavior.object_of_class(class).map(str::to_string));
        let [Some(string_data), Some(graph_data), value_set] = ids else {
            ui.weak(i18n.t(I18nKey::NemesisNoVars));
            return;
        };

        if let Some(error) = &self.vars_error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
        let filter = self.filter.to_lowercase();
        let names = self.names();
        let (events, template_events) = (names.events.clone(), names.template_events);
        let (variables, template_variables) = (names.variables.clone(), names.template_variables);

        // `[Events (279)] [Variables (78)]`: one list at a time, its "add" row always on top.
        ui.horizontal(|ui| {
            for (variables_tab, key, count) in [
                (false, I18nKey::NemesisEvents, events.len()),
                (true, I18nKey::NemesisVariables, variables.len()),
            ] {
                let label = format!("{} ({count})", i18n.t(key));
                if ui.add(Button::selectable(self.show_variables == variables_tab, label)).clicked()
                {
                    self.show_variables = variables_tab;
                }
            }
        });

        let mut action = None;
        if self.show_variables {
            ui.horizontal(|ui| {
                let new = &mut self.new_names;
                ui.add(TextEdit::singleline(&mut new.variable).desired_width(110.0));
                egui::ComboBox::from_id_salt("nemesis_var_type")
                    .width(70.0)
                    .selected_text(type_label(new.kind))
                    .show_ui(ui, |ui| {
                        for kind in VariableType::ALL {
                            ui.selectable_value(&mut new.kind, kind, type_label(kind));
                        }
                    });
                ui.add(TextEdit::singleline(&mut new.value).hint_text("0").desired_width(50.0));
                let enabled = value_set.is_some();
                if ui
                    .add_enabled(enabled, Button::new(i18n.t(I18nKey::NemesisAdd)).small())
                    .clicked()
                {
                    action = Some(Action::AddVariable);
                }
            });
            ui.separator();
            name_rows(ui, &variables, template_variables, &filter, &mut |i| {
                action = Some(Action::RemoveVariable(i));
            });
        } else {
            ui.horizontal(|ui| {
                ui.add(TextEdit::singleline(&mut self.new_names.event).desired_width(140.0));
                if ui.add(Button::new(i18n.t(I18nKey::NemesisAdd)).small()).clicked() {
                    action = Some(Action::AddEvent);
                }
            });
            ui.separator();
            name_rows(ui, &events, template_events, &filter, &mut |i| {
                action = Some(Action::RemoveEvent(i));
            });
        }

        let Some(action) = action else { return };
        let ids = Ids { string_data, graph_data, value_set };
        match self.apply(action, &ids, &events, &variables) {
            Ok(()) => {
                self.names = None;
                self.editor_for = None; // the open object may be one of them
                self.mark_dirty();
            }
            Err(message) => self.vars_error = Some(message),
        }
    }

    fn apply(
        &mut self,
        action: Action,
        ids: &Ids,
        events: &[String],
        variables: &[String],
    ) -> Result<(), String> {
        let behavior = &mut self.workspace.behaviors[self.active];
        let text = |id: &str| behavior.text(id).unwrap_or_default().to_string();
        let new = &mut self.new_names;
        let valid = |name: &str, existing: &[String]| -> Result<String, String> {
            let name = name.trim();
            if name.is_empty() || name.contains(char::is_whitespace) {
                return Err(format!("Invalid name: `{name}`"));
            }
            if existing.iter().any(|n| n.eq_ignore_ascii_case(name)) {
                return Err(format!("Already exists: `{name}`"));
            }
            Ok(name.to_string())
        };
        let mut edits: Vec<(String, String)> = Vec::new();
        match action {
            Action::AddEvent => {
                let name = valid(&new.event, events)?;
                let (s, g) =
                    vars::add_event(&text(&ids.string_data), &text(&ids.graph_data), &name)
                        .ok_or("eventNames / eventInfos not found")?;
                edits = vec![(ids.string_data.clone(), s), (ids.graph_data.clone(), g)];
                new.event.clear();
            }
            Action::AddVariable => {
                let name = valid(&new.variable, variables)?;
                let value = new
                    .kind
                    .word_value(&new.value)
                    .ok_or_else(|| format!("Invalid value: `{}`", new.value))?;
                let value_set = ids.value_set.as_deref().ok_or("No hkbVariableValueSet")?;
                let (s, g, v) = vars::add_variable(
                    &text(&ids.string_data),
                    &text(&ids.graph_data),
                    &text(value_set),
                    &name,
                    new.kind,
                    value,
                )
                .ok_or("variableNames / variableInfos / wordVariableValues not found")?;
                edits = vec![
                    (ids.string_data.clone(), s),
                    (ids.graph_data.clone(), g),
                    (value_set.to_string(), v),
                ];
                new.variable.clear();
                new.value.clear();
            }
            Action::RemoveEvent(i) => {
                for (id, param) in
                    [(&ids.string_data, "eventNames"), (&ids.graph_data, "eventInfos")]
                {
                    edits.push((id.clone(), vars::remove(&text(id), param, i).ok_or(param)?));
                }
            }
            Action::RemoveVariable(i) => {
                let mut targets =
                    vec![(&ids.string_data, "variableNames"), (&ids.graph_data, "variableInfos")];
                if let Some(value_set) = &ids.value_set {
                    targets.push((value_set, "wordVariableValues"));
                }
                for (id, param) in targets {
                    edits.push((id.clone(), vars::remove(&text(id), param, i).ok_or(param)?));
                }
            }
        }
        for (id, edited) in edits {
            behavior.set_text(&id, edited);
        }
        self.vars_error = None;
        Ok(())
    }
}

fn names_of(behavior: &super::Behavior) -> Names {
    let pointers = super::language::pointer_items(behavior);
    let Some(id) = behavior.object_of_class(STRING_DATA) else {
        return Names { pointers, ..Names::default() };
    };
    let current = behavior.text(id).unwrap_or_default();
    let template = behavior.template_object(id).map(|o| o.text.as_str()).unwrap_or_default();
    Names {
        events: vars::names(current, "eventNames"),
        variables: vars::names(current, "variableNames"),
        template_events: vars::names(template, "eventNames").len(),
        template_variables: vars::names(template, "variableNames").len(),
        pointers,
    }
}

#[derive(Debug, Clone, Copy)]
enum Action {
    AddEvent,
    AddVariable,
    RemoveEvent(usize),
    RemoveVariable(usize),
}

struct Ids {
    string_data: String,
    graph_data: String,
    value_set: Option<String>,
}

/// `277 MyEvent ✖` in a scroll area that lays out only the visible rows (lists have ~1000 names).
/// Newest first; only the mod's own entries have `✖`.
fn name_rows(
    ui: &mut egui::Ui,
    names: &[String],
    template_count: usize,
    filter: &str,
    remove: &mut dyn FnMut(usize),
) {
    let shown: Vec<usize> = (0..names.len())
        .rev()
        .filter(|&i| filter.is_empty() || names[i].to_lowercase().contains(filter))
        .collect();
    let row_height = ui.text_style_height(&egui::TextStyle::Monospace) + 6.0;
    egui::ScrollArea::vertical().auto_shrink(false).show_rows(
        ui,
        row_height,
        shown.len(),
        |ui, range| {
            for &i in &shown[range] {
                ui.horizontal(|ui| {
                    let text = egui::RichText::new(format!("{i:>4} {}", names[i])).monospace();
                    if i >= template_count {
                        ui.label(text.strong());
                        if ui.small_button("✖").clicked() {
                            remove(i);
                        }
                    } else {
                        ui.label(text);
                    }
                });
            }
        },
    );
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
