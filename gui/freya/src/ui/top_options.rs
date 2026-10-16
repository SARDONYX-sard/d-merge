//! Top panel: execution mode, target runtime, parser mode, theme, appearance and behavior options.

use d_merge_gui_shared::{
    i18n::I18nKey,
    settings::{DataMode, behavior::ParserMode, ui::theme::Theme as ThemeSetting},
};
use freya::prelude::*;
use freya_extra::dropdown_menu::DropdownMenu;
use skyrim_data_dir::Runtime;

use crate::{
    actions,
    state::AppState,
    theme::{Appearance, to_freya_theme},
    ui::{check_tile, panel, radio_tile, row, separator, with_tooltip, wrap_row},
};

#[derive(PartialEq)]
pub(crate) struct TopOptions;

impl Component for TopOptions {
    fn render(&self) -> impl IntoElement {
        let app = use_consume::<AppState>();

        // egui: | mode | runtime, parser, options |  space  | theme |
        panel().child(
            wrap_row()
                .child(ExecutionMode)
                .child(separator())
                .child(TargetRuntime)
                .child(ParserModeSelect)
                .children(behavior_options(&app))
                .child(separator())
                .child(ThemeSelect)
                .child(AppearanceSelect),
        )
    }
}

#[derive(PartialEq)]
struct ExecutionMode;

impl Component for ExecutionMode {
    fn render(&self) -> impl IntoElement {
        let app = use_consume::<AppState>();
        let mode = app.settings.read().behavior.mode;
        let is_fetching = *app.fetching.read();

        let radio = |value: DataMode, label: I18nKey, hover: I18nKey| {
            let on_select = move |()| {
                // NOTE: Switching while fetching would apply the fetched list to the other mode.
                if is_fetching || mode == value {
                    return;
                }
                let mut settings = app.settings;
                settings.write().behavior.mode = value;
            };
            with_tooltip(radio_tile(mode == value, app.t(label), on_select), app.t(hover))
        };

        row()
            .child(label().text(app.t(I18nKey::ExecutionModeLabel)))
            .child(radio(DataMode::Vfs, I18nKey::VfsMode, I18nKey::VfsModeHover))
            .child(radio(DataMode::Manual, I18nKey::ManualMode, I18nKey::ManualModeHover))
    }
}

#[derive(PartialEq)]
struct TargetRuntime;

impl Component for TargetRuntime {
    fn render(&self) -> impl IntoElement {
        let app = use_consume::<AppState>();
        let runtime = app.settings.read().behavior.target_runtime;

        let select = [Runtime::Le, Runtime::Se, Runtime::Vr]
            .into_iter()
            .fold(DropdownMenu::new(runtime), |menu, runtime| menu.item(runtime, runtime.as_str()))
            .on_change(move |next: Runtime| {
                let mut settings = app.settings;
                settings.write().behavior.target_runtime = next;

                // In VFS mode, the data directory depends on the runtime.
                #[cfg(target_os = "windows")]
                if settings.peek().behavior.mode == DataMode::Vfs {
                    actions::update_vfs_skyrim_data_dir_by_reg(&app);
                }
            });

        row()
            .child(with_tooltip(
                label().text(app.t(I18nKey::RuntimeTargetLabel)),
                app.t(I18nKey::RuntimeTargetHover),
            ))
            .child(select)
    }
}

#[derive(PartialEq)]
struct ParserModeSelect;

impl Component for ParserModeSelect {
    fn render(&self) -> impl IntoElement {
        let app = use_consume::<AppState>();
        let parser_mode = app.settings.read().behavior.parser_mode;

        let select = DropdownMenu::new(parser_mode)
            .item(ParserMode::Strict, app.t(I18nKey::ParserModeStrict))
            .item(ParserMode::Lenient, app.t(I18nKey::ParserModeLenient))
            .on_change(move |next| {
                let mut settings = app.settings;
                settings.write().behavior.parser_mode = next;
            });

        row()
            .child(with_tooltip(
                label().text(app.t(I18nKey::ParserModeLabel)),
                app.t(I18nKey::ParserModeHover),
            ))
            .child(select)
    }
}

#[derive(PartialEq)]
struct ThemeSelect;

impl Component for ThemeSelect {
    fn render(&self) -> impl IntoElement {
        let app = use_consume::<AppState>();
        let mut theme = use_theme();
        let current = app.settings.read().ui.theme;

        let select = DropdownMenu::new(current)
            .item(ThemeSetting::System, app.t(I18nKey::ThemeSelectSystem))
            .item(ThemeSetting::Dark, app.t(I18nKey::ThemeSelectDark))
            .item(ThemeSetting::Light, app.t(I18nKey::ThemeSelectLight))
            .item(ThemeSetting::Custom, app.t(I18nKey::ThemeSelectCustom))
            .on_change(move |next| {
                let mut settings = app.settings;
                settings.write().ui.theme = next;
                let custom = settings.peek().ui.custom_theme.clone();
                theme.set(to_freya_theme(next, &custom));
            });

        row()
            .child(with_tooltip(
                label().text(app.t(I18nKey::ThemeLabel)),
                app.t(I18nKey::ThemeHover),
            ))
            .child(select)
            // Live color editing of the custom theme.
            .maybe_child((current == ThemeSetting::Custom).then(|| {
                let is_open = *app.show_theme_editor.read();
                let button = Button::new()
                    .on_press(move |_| {
                        let mut show = app.show_theme_editor;
                        show.toggle();
                    })
                    .child(app.t(I18nKey::ThemeEditorButton));
                let button = if is_open { button.filled() } else { button };
                with_tooltip(button, app.t(I18nKey::ThemeEditorButtonHover))
            }))
    }
}

#[derive(PartialEq)]
struct AppearanceSelect;

impl Component for AppearanceSelect {
    fn render(&self) -> impl IntoElement {
        let app = use_consume::<AppState>();
        let current = *app.appearance.read();

        let select = Appearance::ALL
            .into_iter()
            // Acrylic is only supported on Windows.
            .filter(|appearance| cfg!(target_os = "windows") || *appearance != Appearance::Acrylic)
            .fold(DropdownMenu::new(current), |menu, appearance| {
                menu.item(appearance, appearance.as_str())
            })
            .on_change(move |next: Appearance| match next.apply_backdrop() {
                Ok(()) => {
                    let mut appearance = app.appearance;
                    appearance.set(next);
                }
                Err(err) => actions::notify_error(&app, err),
            });

        with_tooltip(select, "Window background")
    }
}

/// debug output, auto-remove meshes, generate FNIS ESP, arm fix, auto-run
fn behavior_options(app: &AppState) -> Vec<Element> {
    let app = *app;
    let behavior = app.settings.read().behavior.clone();

    type Toggle = fn(&mut d_merge_gui_shared::settings::Settings);
    let options: [(bool, I18nKey, I18nKey, Toggle); 5] = [
        (behavior.enable_debug_output, I18nKey::DebugOutput, I18nKey::DebugOutputHover, |s| {
            s.behavior.enable_debug_output ^= true;
        }),
        (
            behavior.auto_remove_meshes,
            I18nKey::AutoRemoveMeshes,
            I18nKey::AutoRemoveMeshesHover,
            |s| {
                s.behavior.auto_remove_meshes ^= true;
            },
        ),
        (
            behavior.generate_fnis_esp,
            I18nKey::GenerateFnisEspLabel,
            I18nKey::GenerateFnisEspHover,
            |s| s.behavior.generate_fnis_esp ^= true,
        ),
        (behavior.skeleton_arm_fix, I18nKey::ArmFixLabel, I18nKey::ArmFixHover, |s| {
            s.behavior.skeleton_arm_fix ^= true;
        }),
        (behavior.auto_run, I18nKey::AutoRun, I18nKey::AutoRunHover, |s| {
            s.behavior.auto_run ^= true;
        }),
    ];

    options
        .into_iter()
        .map(|(checked, label, hover, toggle)| {
            let on_toggle = move |()| {
                let mut settings = app.settings;
                toggle(&mut settings.write());
            };
            with_tooltip(check_tile(checked, app.t(label), on_toggle), app.t(hover))
        })
        .collect()
}
