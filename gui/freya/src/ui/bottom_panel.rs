//! Bottom panel: log controls, patch button, help toggle, and the notification bar.

use d_merge_gui_shared::{fs::open_existing_dir_or_ancestor, i18n::I18nKey, log::LogLevel};
use freya::prelude::*;
use freya_extra::dropdown_menu::DropdownMenu;

use crate::{
    actions,
    state::{AppState, Shared},
    ui::{icon, icon_button, icons, log_window::toggle_log_window, panel, row},
};

const BUTTON_WIDTH: f32 = 120.0;

#[derive(PartialEq)]
pub(crate) struct BottomPanel;

impl Component for BottomPanel {
    fn render(&self) -> impl IntoElement {
        let app = use_consume::<AppState>();
        let shared = use_consume::<Shared>();

        let is_fetching = *app.fetching.read();
        let is_patching = *app.patching.read();
        let log_level = app.settings.read().log.level;

        // NOTE: The menu opens upward, because this panel is at the bottom of the window.
        let level_select = LogLevel::all()
            .into_iter()
            .fold(DropdownMenu::new(log_level), |menu, level| menu.item(level, level.as_str()))
            .position(Position::new_absolute().bottom(40.0))
            .on_change(move |next: LogLevel| {
                let mut settings = app.settings;
                settings.write().log.level = next;
                actions::change_log_level(next);
            });

        let patch_label = if is_fetching {
            app.t(I18nKey::PatchFetchingButton)
        } else {
            app.t(I18nKey::PatchButton)
        };
        // egui: `ButtonVariant::Default`(filled). Disabled while fetching, to prevent patching with a stale mod list.
        let inverse = get_theme_or_default().read().colors.text_inverse;
        let patch_button = Button::new()
            .enabled(!is_fetching && !is_patching)
            .filled()
            .theme_layout(ButtonLayoutThemePartial {
                width: Some(Preference::Specific(Size::px(BUTTON_WIDTH))),
                height: Some(Preference::Specific(Size::px(40.0))),
                padding: Some(Preference::Specific(Gaps::new(0., 8., 0., 8.))),
                ..Default::default()
            })
            .on_press({
                let shared = shared.clone();
                move |_| actions::patch(&app, &shared)
            })
            .child(
                row()
                    .expanded()
                    .spacing(6.0)
                    .main_align(Alignment::Center)
                    .child(icon(icons::layers(), 14.0).color(inverse))
                    .child(label().text(patch_label).color(inverse)),
            );

        // egui:
        // | Log Level |                                                         |
        // | [Info v]  | [Log Dir] [Log] [Clear Notify] [Patch]           [Help] |
        panel().child(
            row()
                .width(Size::fill())
                .content(Content::flex())
                .child(
                    rect()
                        .spacing(4.0)
                        .child(label().text(app.t(I18nKey::LogLevelLabel)))
                        .child(level_select),
                )
                .child(icon_button(
                    Some(icons::logs()),
                    app.t(I18nKey::LogDir),
                    (BUTTON_WIDTH, 40.0),
                    move |_| {
                        let dir = app.settings.peek().log.dir_path.clone();
                        if let Err(err) = open_existing_dir_or_ancestor(dir) {
                            actions::notify_error(&app, err);
                        }
                    },
                ))
                .child(icon_button(
                    Some(icons::logs()),
                    app.t(I18nKey::LogButton),
                    (BUTTON_WIDTH, 40.0),
                    move |_| toggle_log_window(&app, &shared),
                ))
                .child(icon_button(
                    Some(icons::logs()),
                    app.t(I18nKey::NotificationClearButton),
                    (BUTTON_WIDTH, 40.0),
                    move |_| actions::clear_notification(&app),
                ))
                .child(patch_button)
                // Push the help button to the right edge.
                .child(rect().width(Size::flex(1.0)))
                .child(icon_button(
                    Some(icons::help()),
                    app.t(I18nKey::HelpButton),
                    (100.0, 40.0),
                    move |_| {
                        let mut show_help = app.show_help;
                        show_help.toggle();
                    },
                )),
        )
    }
}

/// Single-line notification at the very bottom.
#[derive(PartialEq)]
pub(crate) struct NotificationBar;

impl Component for NotificationBar {
    fn render(&self) -> impl IntoElement {
        let app = use_consume::<AppState>();
        let notice = app.notify.read().clone();

        panel().child(
            label()
                .text(notice.text)
                .max_lines(2)
                .text_overflow(TextOverflow::Ellipsis)
                .map(notice.color, |label, color| label.color(color)),
        )
    }
}
