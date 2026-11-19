//! The events / variables panel: list, add, edit, remove. What can be changed: see [`vars_edit`].
//!
//! ```text
//! [Objects] [Events / Variables]
//! [Events (279)] [Variables (78)]     one list at a time
//! [MyEvent      ] [Add]               (variables: [myVar] [REAL ▼] [0.0] [Add])
//! ───────────────
//!   277 MyVar: REAL = 0.2           ✎ ✖   <- added by this mod (name, type, value; removed)
//!     0 iState: INT32 = 0                 <- the template's: only looked at
//!   277 [MyVar] : [REAL ▼] = [0.2]  ✔ ✖   <- being edited: Enter applies, Esc cancels
//! ```
//!
//! A click on a name opens where it is: a variable's initial value (`wordVariableValues`), an event's
//! name (`eventNames`). A template's entry that must change all the same is edited there.

use egui_widgets::{
    HoverMarkdown as _,
    shadcn::{Button, ComboBox, Icon, TextEdit},
};
use gui_shared::i18n::{I18nKey, I18nMap};
use nemesis_diff::vars::VariableType;
use stdx::text::contains_ignore_case;

use crate::patch::{
    editor::names::Names,
    vars_edit::{self, Action, Editing, GRAPH_DATA, Ids, NewNames, VALUE_SET},
    workspace::{Behavior, STRING_DATA},
};

/// What the panel did or asked for in a frame.
#[derive(Debug, Default)]
pub(super) struct VarsOut {
    /// The ids of the objects an add / edit / remove edited.
    pub(super) changed: Vec<String>,
    /// The object and the 0-based line a click on a name opens. e.g. `(#0093, 12)`
    pub(super) open: Option<(String, usize)>,
}

/// What a row of a list asked for.
#[derive(Debug, Clone, Copy)]
enum RowAsk {
    Change(Action),
    /// The name of entry `.0` was clicked.
    Open(usize),
}

/// See the module docs. Its own state only: what is typed, the last error, which list is shown.
#[derive(Debug, Default)]
pub(super) struct VarsPanel {
    new: NewNames,
    error: Option<String>,
    /// Variables are listed (else events).
    show_variables: bool,
}

impl VarsPanel {
    /// Draws the panel of `behavior` (its `names`), the lists filtered by `filter`. Returns what it
    /// edited, and the place a click on a name opens.
    pub(super) fn show(
        &mut self,
        ui: &mut egui::Ui,
        i18n: &I18nMap,
        behavior: &mut Behavior,
        names: &Names,
        filter: &str,
    ) -> VarsOut {
        let ids = [STRING_DATA, GRAPH_DATA, VALUE_SET].map(|class| behavior.object_of_class(class).map(str::to_string));
        let [Some(string_data), Some(graph_data), value_set] = ids else {
            ui.weak(i18n.t(I18nKey::VarsNone));
            return VarsOut::default();
        };

        if let Some(error) = &self.error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
        let (events, template_events) = (&names.events, names.template_events);
        let (variables, template_variables) = (&names.variables, names.template_variables);

        // `[Events (279)] [Variables (78)]`: one list at a time, its "add" row always on top.
        ui.horizontal(|ui| {
            for (variables_tab, key, count) in
                [(false, I18nKey::VarsEvents, events.len()), (true, I18nKey::VarsVariables, variables.len())]
            {
                let label = format!("{} ({count})", i18n.t(key));
                if ui.add(Button::selectable(self.show_variables == variables_tab, label)).clicked() {
                    self.show_variables = variables_tab;
                }
            }
        });

        let mut asked = None;
        if self.show_variables {
            asked = new_variable_row(ui, i18n, &mut self.new, value_set.is_some()).map(RowAsk::Change);
            ui.separator();
            let list = NameList {
                names: variables,
                types: Some((&names.variable_types, &names.variable_words)),
                template_count: template_variables,
                is_variable: true,
            };
            asked = name_rows(ui, i18n, list, filter, &mut self.new.editing).or(asked);
        } else {
            ui.horizontal_wrapped(|ui| {
                let name = TextEdit::singleline(&mut self.new.event)
                    .hint_text(i18n.t(I18nKey::VarsNewEventHint))
                    .desired_width(140.0);
                ui.add(name).on_hover_markdown(i18n.t(I18nKey::VarsNewEventHover));
                if ui.add(Button::new(i18n.t(I18nKey::AddButton)).small()).clicked() {
                    asked = Some(RowAsk::Change(Action::AddEvent));
                }
            });
            ui.separator();
            let list = NameList { names: events, types: None, template_count: template_events, is_variable: false };
            asked = name_rows(ui, i18n, list, filter, &mut self.new.editing).or(asked);
        }

        let ids = Ids { string_data, graph_data, value_set };
        match asked {
            None => VarsOut::default(),
            Some(RowAsk::Open(index)) => VarsOut { open: self.place_of(behavior, &ids, index), ..VarsOut::default() },
            Some(RowAsk::Change(action)) => {
                match vars_edit::apply(behavior, &mut self.new, action, &ids, (events, variables)) {
                    Ok(changed) => {
                        self.error = None;
                        VarsOut { changed, ..VarsOut::default() }
                    }
                    Err(message) => {
                        self.error = Some(message);
                        VarsOut::default()
                    }
                }
            }
        }
    }

    /// Where entry `index` of the list shown is: a variable's initial value, an event's name.
    fn place_of(&self, behavior: &Behavior, ids: &Ids, index: usize) -> Option<(String, usize)> {
        let (id, param) = if self.show_variables {
            (ids.value_set.as_ref()?, "wordVariableValues")
        } else {
            (&ids.string_data, "eventNames")
        };
        let line = nemesis_diff::vars::element_line(behavior.text(id)?, param, index)?;
        Some((id.clone(), line))
    }
}

/// `[myVar] [REAL ▼] [0.0] [Add]`, wrapped, not wider than the side bar. `can_add`: the behavior has
/// a value set for the initial value.
fn new_variable_row(ui: &mut egui::Ui, i18n: &I18nMap, new: &mut NewNames, can_add: bool) -> Option<Action> {
    let mut action = None;
    ui.horizontal_wrapped(|ui| {
        let name = TextEdit::singleline(&mut new.variable)
            .hint_text(i18n.t(I18nKey::VarsNewVariableHint))
            .desired_width(110.0);
        ui.add(name).on_hover_markdown(i18n.t(I18nKey::VarsNewVariableHover));

        let mut kind = VariableType::ALL.iter().position(|kind| *kind == new.kind).unwrap_or_default();
        let types = ComboBox::from_id_salt("nemesis_var_type").width(90.0);
        let response = types.show_index(ui, &mut kind, VariableType::ALL.len(), |at| VariableType::ALL[at].name());
        if response.on_hover_text(i18n.t(I18nKey::VarsNewVariableTypeHover)).changed() {
            new.kind = VariableType::ALL[kind];
        }

        ui.add(TextEdit::singleline(&mut new.value).hint_text("0").desired_width(50.0))
            .on_hover_markdown(i18n.t(I18nKey::VarsNewVariableValueHover));
        if ui.add_enabled(can_add, Button::new(i18n.t(I18nKey::AddButton)).small()).clicked() {
            action = Some(Action::AddVariable);
        }
    });
    action
}

/// One list of the panel.
#[derive(Debug, Clone, Copy)]
struct NameList<'a> {
    names: &'a [String],
    /// Variables: the type and the initial value (as written) of each, by index.
    types: Option<(&'a [Box<str>], &'a [i64])>,
    /// How many belong to the template: those are only looked at. See [`vars_edit`].
    template_count: usize,
    is_variable: bool,
}

impl NameList<'_> {
    /// The type of variable `index` and its initial value as typed. e.g. `(Some(Real), "0.2")`;
    /// `(None, "")` for an event, or a type without one here (`POINTER`).
    fn variable(&self, index: usize) -> (Option<VariableType>, String) {
        let Some((types, words)) = self.types else { return (None, String::new()) };
        let kind = types.get(index).and_then(|name| VariableType::from_name(name));
        let word = words.get(index).copied().unwrap_or_default();
        (kind, kind.map(|kind| kind.show(word)).unwrap_or_default())
    }
}

/// `277 MyVar: REAL = 0.2 ✎ ✖` in a scroll area that lays out only the visible rows (lists have
/// ~1000 names). Newest first. Returns what a row asked for.
fn name_rows(
    ui: &mut egui::Ui,
    i18n: &I18nMap,
    list: NameList<'_>,
    filter: &str,
    editing: &mut Option<Editing>,
) -> Option<RowAsk> {
    let shown: Vec<usize> =
        (0..list.names.len()).rev().filter(|&i| contains_ignore_case(&list.names[i], filter)).collect();
    let row_height = ui.spacing().interact_size.y.max(ui.text_style_height(&egui::TextStyle::Monospace) + 6.0);
    let mut action = None;
    // Up and down only: a long name is cut with `…` (sideways, the width changed with the rows drawn).
    egui::ScrollArea::vertical().auto_shrink(false).show_rows(ui, row_height, shown.len(), |ui, range| {
        for &index in &shown[range] {
            // The list's width exactly: a row a pixel wider would widen the side bar, a little more
            // each time the view is shown again.
            egui_tree::list::fixed_row(ui, row_height, |ui| {
                ui.label(egui::RichText::new(format!("{index:>4}")).monospace().weak());
                let is_edited = editing.as_ref().is_some_and(|e| e.is_variable == list.is_variable && e.index == index);
                let asked = if is_edited {
                    edit_fields(ui, i18n, editing).map(RowAsk::Change)
                } else {
                    name_row(ui, i18n, list, index, editing)
                };
                action = asked.or(action);
            });
        }
    });
    action
}

/// One row: `MyVar: REAL = 0.2` (an event: its name), then `✎ ✖` if it is this mod's. A click on
/// the name opens where it is.
fn name_row(
    ui: &mut egui::Ui,
    i18n: &I18nMap,
    list: NameList<'_>,
    index: usize,
    editing: &mut Option<Editing>,
) -> Option<RowAsk> {
    let NameList { names, template_count, is_variable, .. } = list;
    let own = index >= template_count;
    let (kind, value) = list.variable(index);

    // `: REAL = 0.2` after the name, which is cut with `…` to the room left.
    let suffix = list.types.is_some().then(|| variable_suffix(ui, list, index, &value));
    let button_count = if own { 2.0 } else { 0.0 };
    let buttons = button_count * (Button::SMALL_ICON_WIDTH + ui.spacing().item_spacing.x);
    let suffix_width = suffix.as_ref().map_or(0.0, |galley| galley.size().x + ui.spacing().item_spacing.x);
    let color = if own { ui.visuals().strong_text_color() } else { ui.visuals().text_color() };
    let fitted = egui_widgets::fit_text(ui, &names[index], color, ui.available_width() - buttons - suffix_width);
    // Its own hover only: egui's label would show one more for a cut text.
    let label = ui.add(egui::Label::new(fitted.galley).sense(egui::Sense::click()).show_tooltip_when_elided(false));
    let opened = label.clicked();
    let label = label.on_hover_cursor(egui::CursorIcon::PointingHand);
    egui_widgets::CutHover::new().show_if(ui, label, fitted.elided, |ui| {
        ui.add(egui::Label::new(egui::RichText::new(&names[index]).monospace()).wrap());
    });
    if let Some(suffix) = suffix {
        ui.label(suffix);
    }

    if !own {
        return opened.then_some(RowAsk::Open(index));
    }
    let hover = if is_variable { I18nKey::VarsEdit } else { I18nKey::VarsRename };
    if ui.add(Button::new("").icon(Icon::Pencil).small()).on_hover_markdown(i18n.t(hover)).clicked() {
        *editing = Some(Editing { is_variable, index, name: names[index].clone(), value, kind, focus: true });
    }
    if ui.add(Button::new("").icon(Icon::X).small()).clicked() {
        let action = if is_variable { Action::RemoveVariable(index) } else { Action::RemoveEvent(index) };
        return Some(RowAsk::Change(action));
    }
    opened.then_some(RowAsk::Open(index))
}

/// `: REAL = 0.2` of variable `index`, its type in its color. A type without an initial value here
/// has only itself: `: POINTER`.
fn variable_suffix(ui: &egui::Ui, list: NameList<'_>, index: usize, value: &str) -> std::sync::Arc<egui::Galley> {
    let kind = list.types.and_then(|(types, _)| types.get(index)).map_or("?", |kind| &**kind);
    let font = egui::TextStyle::Monospace.resolve(ui.style());
    let format = |color| egui::TextFormat { font_id: font.clone(), color, ..Default::default() };
    let weak = ui.visuals().weak_text_color();

    let mut job = egui::text::LayoutJob::default();
    job.append(": ", 0.0, format(weak));
    job.append(kind, 0.0, format(type_color(ui)));
    if !value.is_empty() {
        job.append(" = ", 0.0, format(weak));
        job.append(value, 0.0, format(ui.visuals().text_color()));
    }
    ui.fonts_mut(|fonts| fonts.layout_job(job))
}

/// The row of this mod's entry being edited: `[MyVar] : [REAL ▼] = [0.2] ✔ ✖` (an event:
/// `[MyEvent] ✔ ✖`). `Enter` applies, `Esc` or `✖` cancels.
fn edit_fields(ui: &mut egui::Ui, i18n: &I18nMap, editing: &mut Option<Editing>) -> Option<Action> {
    let state = editing.as_mut()?;
    let mut fields = vec![ui.add(TextEdit::singleline(&mut state.name).desired_width(110.0))];
    if let Some(kind) = &mut state.kind {
        ui.weak(":");
        let mut at = VariableType::ALL.iter().position(|other| other == kind).unwrap_or_default();
        let types = ComboBox::from_id_salt("nemesis_var_edit_type").width(70.0);
        if types.show_index(ui, &mut at, VariableType::ALL.len(), |at| VariableType::ALL[at].name()).changed() {
            *kind = VariableType::ALL[at];
        }
        ui.weak("=");
        let value = ui.add(TextEdit::singleline(&mut state.value).desired_width(60.0));
        fields.push(value.on_hover_markdown(i18n.t(I18nKey::VarsNewVariableValueHover)));
    }
    if std::mem::take(&mut state.focus) {
        fields[0].request_focus();
    }

    let apply = ui.add(Button::new("").icon(Icon::Check).small()).on_hover_text(i18n.t(I18nKey::ApplyButton));
    let cancel = ui.add(Button::new("").icon(Icon::X).small()).on_hover_text(i18n.t(I18nKey::CancelButton));
    if cancel.clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        *editing = None;
        return None;
    }
    let entered = fields.iter().any(egui::Response::lost_focus) && ui.input(|i| i.key_pressed(egui::Key::Enter));
    (apply.clicked() || entered).then_some(Action::Edit)
}

/// The color of a variable's type, to tell it from the name at a glance.
fn type_color(ui: &egui::Ui) -> egui::Color32 {
    if ui.visuals().dark_mode { egui::Color32::from_rgb(86, 182, 194) } else { egui::Color32::from_rgb(0, 110, 130) }
}
