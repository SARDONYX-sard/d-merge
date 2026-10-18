//! Custom theme editor: edit every color of the theme live, and save/load presets.

use d_merge_gui_shared::i18n::I18nKey;
use freya::prelude::*;
use freya_extra::{
    color_picker::ColorPicker, dropdown_menu::DropdownMenu, floating_window::FloatingWindow, themes,
};

use crate::{
    actions,
    state::AppState,
    theme::colors,
    theme_preset::{self, COLOR_FIELDS, CUSTOM_THEME_NAME, ThemePreset},
    ui::{check_tile, row},
};

/// Start point of the editor: a built-in theme or a saved preset.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Base {
    Dark,
    Light,
    DarkGlass,
    LiquidGlass,
    Preset(String),
}

impl Base {
    fn to_theme(&self, themes_dir: &str) -> Result<(Theme, bool), String> {
        Ok(match self {
            Self::Dark => (dark_theme(), true),
            Self::Light => (light_theme(), false),
            Self::DarkGlass => (themes::dark_glass::theme(), true),
            Self::LiquidGlass => (themes::liquid_glass::theme(), true),
            Self::Preset(name) => {
                let preset = theme_preset::load_preset(themes_dir, name)?;
                (preset.to_theme(), preset.dark)
            }
        })
    }
}

/// `Theme` is not `Clone`: build a new one from the base and the colors.
fn custom_theme(colors: ColorsSheet, dark: bool) -> Theme {
    let mut theme = if dark { dark_theme() } else { light_theme() };
    theme.name = CUSTOM_THEME_NAME;
    theme.colors = colors;
    crate::theme::with_egui_metrics(theme)
}

#[derive(PartialEq)]
pub(crate) struct ThemeEditorWindow;

impl Component for ThemeEditorWindow {
    fn render(&self) -> impl IntoElement {
        let app = use_consume::<AppState>();
        let mut theme = use_theme();

        let (themes_dir, selected) = {
            let settings = app.settings.read();
            let custom = &settings.ui.custom_theme;
            (custom.themes_dir.clone(), custom.freya_selected_theme.clone())
        };

        let mut base = use_state(|| selected.clone().map_or(Base::LiquidGlass, Base::Preset));
        let mut is_dark = use_state(|| true);
        let name = use_state(|| selected.clone().unwrap_or_else(|| "my_theme".to_string()));
        // Re-read the preset list after saving.
        let mut presets_generation = use_state(|| 0_u64);

        let is_open = *app.show_theme_editor.read();
        if !is_open {
            return FloatingWindow::new("").open(false).into_element();
        }

        let _ = *presets_generation.read();
        let presets = theme_preset::list_presets(&themes_dir);

        // -- Base select
        let builtins = [
            (Base::Dark, "Dark"),
            (Base::Light, "Light"),
            (Base::DarkGlass, "DarkGlass"),
            (Base::LiquidGlass, "LiquidGlass"),
        ];
        let base_select = builtins
            .into_iter()
            .map(|(base, label)| (base, label.to_string()))
            .chain(presets.iter().map(|name| (Base::Preset(name.clone()), format!("📄 {name}"))))
            .fold(DropdownMenu::new(base.read().clone()), |menu, (base, label)| {
                menu.item(base, label)
            })
            .on_change({
                let themes_dir = themes_dir.clone();
                move |next: Base| match next.to_theme(&themes_dir) {
                    Ok((mut next_theme, dark)) => {
                        next_theme.name = CUSTOM_THEME_NAME;
                        theme.set(next_theme);
                        is_dark.set(dark);

                        if let Base::Preset(preset_name) = &next {
                            let mut settings = app.settings;
                            settings.write().ui.custom_theme.freya_selected_theme =
                                Some(preset_name.clone());
                            let mut name = name;
                            name.set(preset_name.clone());
                        }
                        base.set(next);
                    }
                    Err(err) => actions::notify_error(&app, err),
                }
            });

        // -- Save
        let save_button = Button::new()
            .filled()
            .on_press({
                let themes_dir = themes_dir.clone();
                move |_| {
                    let preset_name = name.read().trim().to_string();
                    if preset_name.is_empty() {
                        actions::notify_error(&app, "Theme name is empty.");
                        return;
                    }

                    let preset = ThemePreset::from_theme(&theme.read(), *is_dark.read());
                    match theme_preset::save_preset(&themes_dir, &preset_name, &preset) {
                        Ok(path) => {
                            let mut settings = app.settings;
                            settings.write().ui.custom_theme.freya_selected_theme =
                                Some(preset_name.clone());
                            base.set(Base::Preset(preset_name));
                            *presets_generation.write() += 1;
                            actions::notify(
                                &app,
                                format!("Saved {}", path.display()),
                                colors::GREEN,
                            );
                        }
                        Err(err) => actions::notify_error(&app, err),
                    }
                }
            })
            .child("Save");

        let open_dir_button = Button::new()
            .on_press({
                move |_| {
                    let dir = theme_preset::presets_dir(&themes_dir);
                    if let Err(err) = d_merge_gui_shared::fs::open_existing_dir_or_ancestor(dir) {
                        actions::notify_error(&app, err);
                    }
                }
            })
            .child("📁");

        // -- Colors
        let color_list = VirtualScrollView::new(move |item, _| {
            let field = &COLOR_FIELDS[item.index];
            let current = (field.get)(&theme.read().colors);

            rect()
                .horizontal()
                .cross_align(Alignment::Center)
                .width(Size::fill())
                .height(Size::px(item.size))
                .padding(6.0)
                .spacing(8.0)
                .child(label().text(field.name).font_size(13.0).width(Size::px(170.0)))
                .child(
                    ColorPicker::new(move |color| {
                        let mut colors = theme.read().colors.clone();
                        (field.set)(&mut colors, color);
                        theme.set(custom_theme(colors, *is_dark.peek()));
                    })
                    .value(current)
                    // NOTE: Rows of the virtual list are recycled: key by field, so a picker never keeps
                    // the state of another field.
                    .key(field.name),
                )
                .into()
        })
        .length(COLOR_FIELDS.len())
        .item_size(64.0)
        .width(Size::fill())
        // NOTE: An explicit height: `fill` collapses to 0 inside the window body sized by its content.
        .height(Size::px(420.0));

        FloatingWindow::new(app.t(I18nKey::ThemeEditorButton))
            .open(true)
            .on_close(move |()| {
                let mut show = app.show_theme_editor;
                show.set(false);
            })
            .initial_position(60.0, 120.0)
            .width(460.0)
            .child(row().child(label().text("Base")).child(base_select))
            .child(
                row()
                    .child(Input::new(name).placeholder("Theme name").width(Size::px(200.0)))
                    .child(save_button)
                    .child(open_dir_button),
            )
            .child(check_tile(*is_dark.read(), "Dark base", move |()| {
                is_dark.toggle();
                let colors = theme.read().colors.clone();
                theme.set(custom_theme(colors, *is_dark.peek()));
            }))
            .child(color_list)
            .into_element()
    }
}
