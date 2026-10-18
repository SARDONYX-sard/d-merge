//! Root of the application.

use d_merge_gui_shared::{i18n::I18nMap, settings::DataMode};
use freya::prelude::*;

use crate::{
    RootContext, actions,
    state::{AppState, Shared},
    theme::{Appearance, to_freya_theme},
    ui::{
        bottom_panel::{BottomPanel, NotificationBar},
        help::HelpWindow,
        mod_list::{ModList, TableContextMenu},
        paths::PathsPanel,
        theme_editor::ThemeEditorWindow,
        top_options::TopOptions,
    },
};

/// Root component of the main window.
pub(crate) fn app() -> impl IntoElement {
    let RootContext { mirror, runtime, log_geometry } = use_consume::<RootContext>();

    let app = use_provide_context(|| {
        let settings = mirror.0.borrow().clone();
        let i18n =
            I18nMap::load_with_fallback(settings.ui.i18n_path.as_str()).unwrap_or_else(|e| {
                tracing::error!("Failed to load i18n map: {e}\nFallback to default");
                I18nMap::new()
            });
        AppState::new(settings, i18n)
    });
    let shared = use_provide_context(|| Shared::new(runtime.clone(), log_geometry.clone()));
    use_init_theme(|| {
        let settings = app.settings.peek();
        to_freya_theme(settings.ui.theme, &settings.ui.custom_theme)
    });

    // One-shot setup.
    use_hook({
        let shared = shared.clone();
        move || {
            crate::log::start_log_watcher(&app.settings.peek().log.dir_path, shared.log_lines);
            actions::reload_font(&app);
        }
    });

    // Keep the settings saved by the close hook up to date.
    use_side_effect(move || {
        *mirror.0.borrow_mut() = app.settings.read().clone();
    });

    // Refresh the mod list when the mode or the Skyrim data directory changes. (Also on startup.)
    let (mode, skyrim_data_dir) = {
        let settings = app.settings.read();
        (settings.behavior.mode, settings.current_skyrim_data_dir().to_owned())
    };
    use_side_effect_with_deps(&(mode, skyrim_data_dir), {
        let shared = shared.clone();
        move |(mode, skyrim_data_dir)| {
            if *mode == DataMode::Vfs && skyrim_data_dir.trim().is_empty() {
                // NOTE: Updating the dir re-runs this effect, then the mod list is fetched.
                #[cfg(target_os = "windows")]
                actions::update_vfs_skyrim_data_dir_by_reg(&app);
                return;
            }

            actions::refresh_table(&app);
            actions::update_mod_list(&app, &shared);
        }
    });

    let appearance = *app.appearance.read();
    let font_family = app.font_family.read().clone();

    let root = rect()
        .expanded()
        .theme_color()
        // egui: 14px body text
        .font_size(14.0)
        .on_global_key_down({
            let shared = shared.clone();
            move |event: Event<KeyboardEventData>| on_key(&app, &shared, &event, true)
        })
        .on_global_key_up(move |event: Event<KeyboardEventData>| {
            on_key(&app, &shared, &event, false);
        })
        .map(font_family, |root, family| root.font_family(family));

    let root = match appearance {
        Appearance::Acrylic | Appearance::Transparent => root.background(Color::TRANSPARENT),
        Appearance::Color => root.theme_background(),
    };

    root.child(BackgroundImage)
        .child(
            // NOTE: Panels are stacked without gaps like egui panels. (The background shows through
            //       translucent theme colors only.)
            rect()
                .expanded()
                .content(Content::flex())
                .child(TopOptions)
                .child(PathsPanel)
                .child(rect().width(Size::fill()).height(Size::flex(1.0)).child(ModList))
                .child(BottomPanel)
                .child(NotificationBar),
        )
        .child(TableContextMenu)
        .child(ThemeEditorWindow)
        .child(HelpWindow)
}

/// - Tracks the Shift key for range selection.
/// - `Ctrl + R`: reload the mod list.
fn on_key(app: &AppState, shared: &Shared, event: &Event<KeyboardEventData>, is_down: bool) {
    let data = event.data();

    if data.key == Key::Named(NamedKey::Shift) && *app.shift_held.peek() != is_down {
        let mut shift_held = app.shift_held;
        shift_held.set(is_down);
    }

    if is_down
        && data.modifiers.ctrl()
        && matches!(&data.key, Key::Character(c) if c.eq_ignore_ascii_case("r"))
        && !*app.fetching.peek()
    {
        actions::update_mod_list(app, shared);
    }
}

/// Background image covering the whole window.
#[derive(PartialEq)]
struct BackgroundImage;

impl Component for BackgroundImage {
    fn render(&self) -> impl IntoElement {
        let app = use_consume::<AppState>();
        let generation = *app.background_generation.read();
        let (enabled, path) = {
            let settings = app.settings.read();
            let image = &settings.ui.background_image;
            (image.enabled, image.path.clone())
        };

        if !enabled || path.trim().is_empty() {
            return rect().into_element();
        }

        rect()
            .position(Position::new_absolute().top(0.0).left(0.0))
            .width(Size::fill())
            .height(Size::fill())
            .layer(Layer::Relative(-1))
            .child(
                ImageViewer::new(std::path::PathBuf::from(path))
                    .key(generation)
                    .width(Size::fill())
                    .height(Size::fill())
                    .aspect_ratio(AspectRatio::Max)
                    .image_cover(ImageCover::Center),
            )
            .into_element()
    }
}
