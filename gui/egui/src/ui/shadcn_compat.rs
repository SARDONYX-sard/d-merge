//! This module provides wrappers around egui and shadcn ui.
//!
//! - demo: https://pjankiewicz.github.io/egui-shadcn/
//! - example: https://github.com/pjankiewicz/egui-shadcn/blob/main/examples/component_dashboard.rs

/// Shows a dropdown for selecting a value from a list.
pub(crate) fn enum_select<T, S>(
    ui: &mut egui::Ui,
    selected: &mut T,
    items: &[(T, S)],
    size: Option<impl Into<egui::Vec2>>,
) -> egui::Response
where
    T: Clone + PartialEq,
    S: Clone + std::fmt::Display + PartialEq,
{
    use egui_shadcn::Select;

    let labels: Vec<String> = items.iter().map(|(_, label)| label.to_string()).collect();

    let before = selected.clone();

    let mut current =
        items.iter().find(|(value, _)| value == selected).map(|(_, label)| label.to_string());

    let mut response = match size {
        Some(size) => ui.add_sized(size, Select::new(&mut current, &labels)),
        None => ui.add(Select::new(&mut current, &labels)),
    };

    if let Some(label) = current
        && let Some((value, _)) = items.iter().find(|(_, l)| l.to_string() == label)
    {
        *selected = value.clone();
    }

    if *selected != before {
        // egui_shadcn::Select does not correctly report changes.
        // Mark the response manually so callers can rely on changed().
        response.mark_changed();
    }

    response
}

pub(crate) fn enum_options<T>(
    ui: &mut egui::Ui,
    current: &mut Option<T>,
    items: &[(Option<T>, String)],
    size: Option<impl Into<egui::Vec2>>,
) -> egui::Response
where
    T: Clone + PartialEq,
{
    use egui_shadcn::Select;

    let labels: Vec<String> = items.iter().map(|(_, label)| label.clone()).collect();

    let mut selected =
        items.iter().find(|(value, _)| value == current).map(|(_, label)| label.clone());

    let response = match size {
        Some(size) => ui.add_sized(size, Select::new(&mut selected, &labels)),
        None => ui.add(Select::new(&mut selected, &labels)),
    };

    if let Some(label) = selected
        && let Some((value, _)) = items.iter().find(|(_, l)| *l == label)
    {
        *current = value.clone();
    }

    response
}

pub(crate) fn searchable_string_select(
    ui: &mut egui::Ui,
    selected: &mut String,
    items: &[String],
    hint: impl Into<String>,
) -> egui::Response {
    let before = selected.clone();

    let mut index = items.iter().position(|item| item == selected);

    let mut response = super::combo_box::Combobox::new("font_family", items.to_vec())
        .search_hint(hint)
        .width(ui.available_width())
        .show(ui, &mut index);

    if let Some(index) = index
        && let Some(value) = items.get(index)
    {
        *selected = value.clone();
    }

    if *selected != before {
        // egui_shadcn::Combobox does not correctly report changes.
        response.mark_changed();
    }

    response
}

/// Shows a radio button representing a specific value.
///
/// If the radio button is selected, `current` is updated to `value`.
///
/// This is similar to `egui::Ui::radio_value`.
pub(crate) fn radio_value<T>(
    ui: &mut egui::Ui,
    current: &mut T,
    value: T,
    text: impl Into<egui::WidgetText>,
) -> egui::Response
where
    T: PartialEq + Clone,
{
    let mut checked = *current == value;

    let response = ui.add(egui_shadcn::Radio::new(&mut checked).label(text));

    if response.clicked() {
        *current = value;
    }

    response
}

pub(crate) fn button(text: impl Into<egui::WidgetText>) -> impl egui::Widget {
    egui_shadcn::Button::new(text).variant(egui_shadcn::ButtonVariant::Outline)
}

pub(crate) fn lock_button(text: impl Into<egui::WidgetText>) -> impl egui::Widget {
    egui_shadcn::Button::new(text)
        .full_width()
        .icon(egui_shadcn::LucideIcon::Lock)
        .variant(egui_shadcn::ButtonVariant::Destructive)
}

pub(crate) fn button_with_icon(
    text: impl Into<egui::WidgetText>,
    icon: egui_shadcn::LucideIcon,
) -> impl egui::Widget {
    egui_shadcn::Button::new(text)
        .full_width()
        .icon(icon)
        .variant(egui_shadcn::ButtonVariant::Outline)
}

pub(crate) fn patch_button(text: impl Into<egui::WidgetText>) -> impl egui::Widget {
    egui_shadcn::Button::new(text)
        .full_width()
        .icon(egui_shadcn::LucideIcon::Layers)
        .variant(egui_shadcn::ButtonVariant::Default)
}

pub(crate) fn text(
    ui: &mut egui::Ui,
    value: &mut String,
    hint: Option<&str>,
    width: f32,
) -> egui::Response {
    let mut input = egui_shadcn::Input::new(value).desired_width(width);

    if let Some(hint) = hint {
        input = input.placeholder(hint);
    }

    input.show(ui)
}

pub(crate) fn heading(value: impl Into<String>) -> impl egui::Widget {
    egui_shadcn::Typography::h3(value)
}

pub(crate) fn icon(ui: &mut egui::Ui, icon: &egui_shadcn::LucideIcon) {
    let size = 18.0;
    let rect = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover()).0;
    egui_shadcn::paint_icon(
        ui.painter(),
        rect,
        icon,
        egui_shadcn::ShadcnThemeExt::shadcn_theme(ui.ctx()).foreground,
    );
}
