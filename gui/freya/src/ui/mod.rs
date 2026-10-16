//! UI components.
//!
//! The look and the sizes follow the egui GUI(egui-shadcn): flat panels separated by lines,
//! 14px text, outline buttons with Lucide icons.
//!
//! Layout(top to bottom):
//! ```txt
//! | TopOptions       | mode, runtime, parser, behavior options, theme, appearance
//! | PathsPanel       | Skyrim data dir, output dir
//! | ModList          | toolbar + drag & drop table
//! | BottomPanel      | log level, log buttons, patch, help
//! | NotificationBar  |
//! ```
pub(crate) mod bottom_panel;
pub(crate) mod color_picker;
pub(crate) mod help;
pub(crate) mod log_window;
pub(crate) mod mod_list;
pub(crate) mod paths;
pub(crate) mod theme_editor;
pub(crate) mod top_options;

use freya::{icons::lucide, prelude::*};

/// Height of the buttons in a toolbar row.
pub(crate) const BUTTON_HEIGHT: f32 = 32.0;

/// Flat panel like `egui::TopBottomPanel`: the theme background, divided by a line.
pub(crate) fn panel() -> Rect {
    let colors = get_theme_or_default().read().colors.clone();

    rect()
        .width(Size::fill())
        .padding((6.0, 8.0))
        .spacing(4.0)
        .background(colors.background)
        .border(Border::new().fill(colors.border).width(BorderWidth {
            top: 0.0,
            right: 0.0,
            bottom: 1.0,
            left: 0.0,
        }))
}

/// Horizontal row with vertically centered children.
pub(crate) fn row() -> Rect {
    rect().horizontal().cross_align(Alignment::Center).spacing(8.0)
}

/// Full-width row that wraps its children to the next line when the window is narrow.
pub(crate) fn wrap_row() -> Rect {
    row().width(Size::fill()).content(Content::Wrap { wrap_spacing: Some(4.0) })
}

/// Vertical line like `ui.separator()` in a horizontal layout.
pub(crate) fn separator() -> Rect {
    let colors = get_theme_or_default().read().colors.clone();
    rect().width(Size::px(1.0)).height(Size::px(22.0)).background(colors.border)
}

/// Shows `text` while hovering `child`, like egui's `on_hover_text`. (500ms delay, same as egui)
///
/// NOTE: `Tooltip::new_text` is limited to one line, but the hover texts have several paragraphs.
pub(crate) fn with_tooltip(child: impl Into<Element>, text: impl Into<String>) -> Element {
    let text: String = text.into();
    if text.is_empty() {
        return child.into();
    }

    let tooltip = Tooltip::new()
        .child(rect().max_width(Size::px(500.0)).child(label().text(text).font_size(14.0)));
    TooltipContainer::new(tooltip).child(child).into_element()
}

/// Lucide icon tinted with the text color.
pub(crate) fn icon(icon_svg: (&'static str, Bytes), size: f32) -> SvgViewer {
    let color = get_theme_or_default().read().colors.text_primary;
    SvgViewer::new(icon_svg).color(color).width(Size::px(size)).height(Size::px(size))
}

/// Outline button with an optional leading icon, like `button_with_icon` of the egui GUI.
pub(crate) fn icon_button(
    icon_svg: Option<(&'static str, Bytes)>,
    label: impl Into<String>,
    size: (f32, f32),
    on_press: impl FnMut(Event<PressEventData>) + 'static,
) -> Button {
    let label: String = label.into();
    let content = row()
        .spacing(6.0)
        .maybe_child(icon_svg.map(|svg| icon(svg, 14.0).into_element()))
        .child(label);

    Button::new()
        .on_press(on_press)
        .theme_layout(ButtonLayoutThemePartial {
            width: Some(Preference::Specific(Size::px(size.0))),
            height: Some(Preference::Specific(Size::px(size.1))),
            padding: Some(Preference::Specific(Gaps::new(0., 8., 0., 8.))),
            ..Default::default()
        })
        .child(
            rect()
                .expanded()
                .main_align(Alignment::Center)
                .cross_align(Alignment::Center)
                .child(content),
        )
}

/// Fixed-size outline button without an icon.
pub(crate) fn sized_button(
    label: impl Into<String>,
    width: f32,
    on_press: impl FnMut(Event<PressEventData>) + 'static,
) -> Button {
    icon_button(None, label, (width, BUTTON_HEIGHT), on_press)
}

/// Icon-only button, like `icon_only` of the egui GUI.
pub(crate) fn icon_only(
    icon_svg: (&'static str, Bytes),
    enabled: bool,
    on_press: impl FnMut(Event<PressEventData>) + 'static,
) -> Button {
    Button::new()
        .enabled(enabled)
        .on_press(on_press)
        .theme_layout(ButtonLayoutThemePartial {
            width: Some(Preference::Specific(Size::px(36.0))),
            height: Some(Preference::Specific(Size::px(36.0))),
            padding: Some(Preference::Specific(Gaps::new_all(0.))),
            ..Default::default()
        })
        .child(
            rect()
                .expanded()
                .main_align(Alignment::Center)
                .cross_align(Alignment::Center)
                .child(icon(icon_svg, 18.0)),
        )
}

/// Checkbox with a label. The whole row toggles the value.
pub(crate) fn check_tile(
    checked: bool,
    label: impl Into<String>,
    on_toggle: impl FnMut(()) + 'static,
) -> Rect {
    let label: String = label.into();
    let mut on_toggle = on_toggle;
    row()
        .spacing(6.0)
        .cursor(CursorIcon::Pointer)
        .on_press(move |_| on_toggle(()))
        .child(Checkbox::new().selected(checked).size(16.0))
        .maybe_child((!label.is_empty()).then(|| label.into_element()))
}

/// Radio button with a label.
pub(crate) fn radio_tile(
    selected: bool,
    label: impl Into<String>,
    on_select: impl FnMut(()) + 'static,
) -> Rect {
    let label: String = label.into();
    let mut on_select = on_select;
    row()
        .spacing(6.0)
        .cursor(CursorIcon::Pointer)
        .on_press(move |_| on_select(()))
        .child(RadioItem::new().selected(selected))
        .child(label)
}

/// Section title. (egui-shadcn `Typography::h3`)
pub(crate) fn heading(text: impl Into<String>) -> Label {
    let text: String = text.into();
    label().text(text).font_size(24.0).font_weight(FontWeight::SEMI_BOLD)
}

/// Lucide icons used by the GUI.
pub(crate) mod icons {
    use freya::{icons::lucide, prelude::Bytes};

    type Icon = (&'static str, Bytes);

    pub(crate) fn folder_search() -> Icon {
        ("folder-search", lucide::folder_search())
    }
    pub(crate) fn scan_search() -> Icon {
        ("scan-search", lucide::scan_search())
    }
    pub(crate) fn search() -> Icon {
        ("search", lucide::search())
    }
    pub(crate) fn refresh() -> Icon {
        ("refresh-ccw", lucide::refresh_ccw())
    }
    pub(crate) fn refresh_off() -> Icon {
        ("refresh-cw-off", lucide::refresh_cw_off())
    }
    pub(crate) fn normalize() -> Icon {
        ("text-align-justify", lucide::text_align_justify())
    }
    pub(crate) fn lock() -> Icon {
        ("lock", lucide::lock())
    }
    pub(crate) fn logs() -> Icon {
        ("logs", lucide::logs())
    }
    pub(crate) fn layers() -> Icon {
        ("layers", lucide::layers())
    }
    pub(crate) fn help() -> Icon {
        ("wrench", lucide::wrench())
    }
}

/// Props of [`path_row`].
pub(crate) struct PathRow {
    /// Button label. Pressing it opens the directory in the file manager.
    pub label: String,
    /// Bound text.
    pub value: Writable<String>,
    pub placeholder: Option<String>,
    /// Width of the label button.
    pub label_width: f32,
}

/// ```txt
/// |<---label--->|<------ stretch ------>| <trailing buttons> |
/// | Skyrim      | path..................| Auto | Select      |
/// ```
pub(crate) fn path_row(
    props: PathRow,
    on_open_error: impl FnMut(String) + 'static,
    trailing: impl IntoIterator<Item = Element>,
) -> Rect {
    let PathRow { label, value, placeholder, label_width } = props;
    let mut on_open_error = on_open_error;

    let open_button = sized_button(label, label_width, {
        let value = value.clone();
        move |_| {
            let path = value.peek().clone();
            if let Err(err) = d_merge_gui_shared::fs::open_existing_dir_or_ancestor(path) {
                on_open_error(err);
            }
        }
    });

    let input = Input::new(value)
        .width(Size::flex(1.0))
        .map(placeholder, |input, placeholder| input.placeholder(placeholder));

    row()
        .width(Size::fill())
        .content(Content::flex())
        .child(open_button)
        .child(input)
        .children(trailing)
}

// NOTE: Keep `lucide` referenced from the crate root of this module for the `icons` sub module.
#[allow(unused_imports)]
use lucide as _;
