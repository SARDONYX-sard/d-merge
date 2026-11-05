//! The events / variables panel: list, add, remove.
//!
//! ```text
//! [Objects] [Events / Variables]
//! [Events (279)] [Variables (78)]     one list at a time
//! [MyEvent      ] [Add]               (variables: [myVar] [REAL ▼] [0.0] [Add])
//! ───────────────
//!   277 MyEvent                ✖   <- added by this mod (removable)
//!   0   moveStart
//! ```

use egui_widgets::shadcn::{Button, TextEdit};
use gui_shared::i18n::{I18nKey, I18nMap};
use nemesis_diff::vars::VariableType;

use crate::patch::{
    PatchTool,
    vars_edit::{Action, GRAPH_DATA, Ids, VALUE_SET},
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
        let filter = self.filter.to_lowercase();
        let names = self.names();
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
                if ui.add_enabled(enabled, Button::new(i18n.t(I18nKey::NemesisAdd)).small()).clicked() {
                    action = Some(Action::AddVariable);
                }
            });
            ui.separator();
            name_rows(ui, variables, template_variables, &filter, &mut |i| {
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
            name_rows(ui, events, template_events, &filter, &mut |i| {
                action = Some(Action::RemoveEvent(i));
            });
        }

        let Some(action) = action else { return };
        let ids = Ids { string_data, graph_data, value_set };
        match self.apply(action, &ids, events, variables) {
            Ok(changed) => {
                self.editor_for = None; // the open object may be one of them
                for id in changed {
                    self.object_changed(&id);
                }
            }
            Err(message) => self.vars_error = Some(message),
        }
    }
}

/// `277 MyEvent ✖` in a scroll area that lays out only the visible rows (lists have ~1000 names).
/// Newest first; only the mod's own entries have `✖`.
fn name_rows(ui: &mut egui::Ui, names: &[String], template_count: usize, filter: &str, remove: &mut dyn FnMut(usize)) {
    let shown: Vec<usize> =
        (0..names.len()).rev().filter(|&i| filter.is_empty() || names[i].to_lowercase().contains(filter)).collect();
    let row_height = ui.text_style_height(&egui::TextStyle::Monospace) + 6.0;
    egui::ScrollArea::vertical().auto_shrink(false).show_rows(ui, row_height, shown.len(), |ui, range| {
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
    });
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
