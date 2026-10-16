// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod actions;
mod app;
mod fonts;
mod log;
mod state;
mod theme;
mod theme_preset;
mod ui;

use std::{cell::RefCell, rc::Rc};

use d_merge_gui_shared::{log::LOG_FILENAME, settings::Settings};
use freya::{
    prelude::*,
    winit::{
        dpi::LogicalPosition,
        window::{Icon, WindowId},
    },
};
use ui::log_window::{LogGeometry, update_geometry};

#[cfg(feature = "mimalloc")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

pub(crate) const APP_TITLE: &str = concat!("D Merge v", env!("CARGO_PKG_VERSION"));

/// Latest settings, kept in sync with the UI state so that they can be saved when the window closes.
///
/// The close hook runs outside of the component tree, so it cannot read the reactive state directly.
#[derive(Debug, Clone)]
pub(crate) struct SettingsMirror(pub Rc<RefCell<Settings>>);

/// Values passed from `main` to the root component.
///
/// NOTE: A window holds only one root context(`with_root_context` overwrites the previous one),
/// so they are bundled into this struct.
#[derive(Debug, Clone)]
pub(crate) struct RootContext {
    pub mirror: SettingsMirror,
    pub runtime: tokio::runtime::Handle,
    pub log_geometry: LogGeometry,
}

/// Application entry point.
fn main() {
    let (settings, err) = match Settings::load() {
        Ok(s) => (s, None),
        Err(e) => (Settings::default(), Some(e)),
    };

    let _ = tracing_rotation::global::init_with_level(
        settings.log.dir_path.as_str(),
        LOG_FILENAME,
        5,
        settings.log.level,
    );

    std::panic::set_hook(Box::new(|info| {
        tracing::error!(?info);
    }));

    if let Some(err) = err {
        tracing::error!("[Settings loader] {err}\nFallback to default");
    } else {
        tracing::info!("[Settings loader] Loaded settings.json");
    };

    // NOTE: `nemesis_merge::behavior_gen` and the mod list fetcher need tokio.
    // Freya has its own async runtime, so enter the tokio runtime instead of using `#[tokio::main]`.
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(err) => {
            tracing::error!("Failed to create tokio runtime: {err}");
            return;
        }
    };
    let _guard = runtime.enter();

    let mirror = SettingsMirror(Rc::new(RefCell::new(settings.clone())));
    let log_geometry = LogGeometry(Rc::new(RefCell::new(settings.log.window.clone())));
    let window = create_window(&settings, mirror.clone(), log_geometry.clone());
    drop(settings);

    let root_context = RootContext { mirror, runtime: runtime.handle().clone(), log_geometry };
    launch(LaunchConfig::new().with_window(window.with_root_context(root_context)));
}

/// Create the main window: transparent background, last geometry, and save on close.
fn create_window(
    settings: &Settings,
    mirror: SettingsMirror,
    log_geometry: LogGeometry,
) -> WindowConfig {
    let geometry = settings.ui.window.clone();

    let mut window = WindowConfig::new(app::app)
        .with_title(APP_TITLE)
        .with_app_id("D Merge")
        .with_size(f64::from(geometry.width), f64::from(geometry.height))
        .with_min_size(640.0, 480.0)
        // Background transparency. The visible color is decided by the UI (`theme::Appearance`).
        .with_transparency(true)
        .with_background(Color::TRANSPARENT)
        .with_window_attributes(move |attributes, _| {
            attributes
                .with_position(LogicalPosition::new(geometry.pos_x, geometry.pos_y))
                .with_maximized(geometry.maximized)
        })
        .with_window_handle(theme::register_window_handle)
        .with_on_close(move |context, window_id| {
            save_on_close(&context, window_id, &mirror, &log_geometry);
            // Closing the main window ends the app, even while the log window is open.
            context.active_event_loop.exit();
            CloseDecision::Close
        });

    let (rgba, [width, height]) = d_merge_gui_shared::d_merge_icon();
    match Icon::from_rgba(rgba, width, height) {
        Ok(icon) => window = window.with_icon(icon),
        Err(err) => tracing::error!("Failed to load icon: {err}"),
    }

    window
}

/// Write the geometry of the windows to the settings, then save them.
fn save_on_close(
    context: &RendererContext,
    main_window_id: WindowId,
    mirror: &SettingsMirror,
    log_geometry: &LogGeometry,
) {
    for (window_id, app_window) in context.windows.iter() {
        // NOTE: The other window is the log viewer, if it is still open.
        if *window_id != main_window_id {
            log_geometry.update_from(app_window.window());
        }
    }

    let mut settings = mirror.0.borrow_mut();
    if let Some(app_window) = context.windows.get(&main_window_id) {
        update_geometry(app_window.window(), &mut settings.ui.window);
    }
    settings.log.window = log_geometry.0.borrow().clone();

    settings.save();
}
