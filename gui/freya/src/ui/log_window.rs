//! Log viewer: a separate OS window.
//!
//! The main window opens/closes it with [`toggle_log_window`]. Its geometry is kept in
//! [`LogGeometry`] and saved with the settings when the main window closes.

use std::{cell::RefCell, rc::Rc, sync::Arc, time::Duration};

use d_merge_gui_shared::{i18n::I18nKey, settings::ui::WindowGeometry};
use freya::{
    prelude::*,
    winit::{
        dpi::{LogicalPosition, LogicalSize},
        window::Window,
    },
};

use crate::{
    log::LogQueueLock,
    state::{AppState, Shared},
    ui::row,
};

/// Latest geometry of the log window. (Shared between the windows and the close hooks)
#[derive(Debug, Clone)]
pub(crate) struct LogGeometry(pub Rc<RefCell<WindowGeometry>>);

impl LogGeometry {
    pub(crate) fn update_from(&self, window: &Window) {
        update_geometry(window, &mut self.0.borrow_mut());
    }
}

/// Reads the geometry from the native window.
///
/// NOTE: Position and size are only updated when not maximized, to avoid saving the transient geometry.
pub(crate) fn update_geometry(window: &Window, geometry: &mut WindowGeometry) {
    let scale = window.scale_factor();

    geometry.maximized = window.is_maximized();
    if geometry.maximized {
        return;
    }

    if let Ok(position) = window.outer_position() {
        let position: LogicalPosition<f32> = position.to_logical(scale);
        geometry.pos_x = position.x;
        geometry.pos_y = position.y;
    }
    let size: LogicalSize<f32> = window.inner_size().to_logical(scale);
    geometry.width = size.width;
    geometry.height = size.height;
}

/// Opens the log window, or closes it if it is open.
pub(crate) fn toggle_log_window(app: &AppState, shared: &Shared) {
    let mut log_window = app.log_window;

    let open_id = *log_window.peek();
    if let Some(window_id) = open_id {
        // NOTE: The user may have closed it with the title bar button.
        if Platform::get().try_window(window_id).is_some() {
            Platform::get().close_window(window_id);
            log_window.set(None);
            return;
        }
    }

    let config = log_window_config(app, shared);
    spawn_in_window(async move {
        let window_id = Platform::get().launch_window(config).await;
        log_window.set(Some(window_id));
    });
}

fn log_window_config(app: &AppState, shared: &Shared) -> WindowConfig {
    let geometry = shared.log_geometry.0.borrow().clone();
    let colors = get_theme_or_default().read().colors.clone();

    let viewer = LogViewer {
        log_lines: Arc::clone(&shared.log_lines),
        clear_label: app.t_peek(I18nKey::ClearButton),
        colors,
    };

    let log_geometry = shared.log_geometry.clone();
    WindowConfig::new_app(viewer)
        .with_title("Log viewer")
        .with_size(f64::from(geometry.width.max(300.0)), f64::from(geometry.height.max(200.0)))
        .with_transparency(true)
        .with_background(Color::TRANSPARENT)
        .with_window_attributes(move |attributes, _| {
            attributes
                .with_position(LogicalPosition::new(geometry.pos_x, geometry.pos_y))
                .with_maximized(geometry.maximized)
        })
        .with_on_close(move |context, window_id| {
            if let Some(app_window) = context.windows.get(&window_id) {
                log_geometry.update_from(app_window.window());
            }
            CloseDecision::Close
        })
}

/// Root of the log window.
struct LogViewer {
    log_lines: LogQueueLock,
    clear_label: String,
    /// Theme colors of the main window when opened.
    colors: ColorsSheet,
}

impl App for LogViewer {
    fn render(&self) -> impl IntoElement {
        use_init_theme(|| {
            let mut theme = dark_theme();
            theme.colors = self.colors.clone();
            theme
        });
        let mut scroll_controller = use_scroll_controller(ScrollConfig::default);

        // The log file is tailed by another thread: refresh the view periodically.
        let mut line_count = use_state(|| 0_usize);
        use_hook({
            let log_lines = Arc::clone(&self.log_lines);
            move || {
                spawn(async move {
                    loop {
                        let count = log_lines.read().len();
                        if count != *line_count.peek() {
                            line_count.set(count);
                            // Stick to the bottom.
                            scroll_controller.scroll_to(ScrollPosition::End, Direction::Vertical);
                        }
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                });
            }
        });
        let _ = *line_count.read(); // Subscribe

        let lines: Vec<Element> =
            self.log_lines.read().iter().map(|line| crate::log::log_line(line)).collect();

        let clear_lines = Arc::clone(&self.log_lines);
        let copy_lines = Arc::clone(&self.log_lines);

        rect()
            .expanded()
            .theme_background()
            .theme_color()
            .padding(8.0)
            .spacing(6.0)
            .content(Content::flex())
            .child(
                row()
                    .child(
                        Button::new()
                            .on_press(move |_| {
                                clear_lines.write().clear();
                                line_count.set(0);
                            })
                            .child(self.clear_label.clone()),
                    )
                    .child(
                        Button::new()
                            .on_press(move |_| {
                                let text = copy_lines
                                    .read()
                                    .iter()
                                    .map(String::as_str)
                                    .collect::<Vec<_>>()
                                    .join("\n");
                                if let Err(err) = Clipboard::set(text) {
                                    tracing::error!("Failed to copy log: {err:?}");
                                }
                            })
                            .child("Copy"),
                    ),
            )
            .child(
                ScrollView::new_controlled(scroll_controller)
                    .width(Size::fill())
                    .height(Size::flex(1.0))
                    .spacing(2.0)
                    .children(lines),
            )
    }
}
