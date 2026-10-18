//! Help / about window and app settings(font, log, translation, background image).

use std::path::Path;

use d_merge_gui_shared::{i18n::I18nKey, settings::ui::FontMode};
use freya::prelude::*;
use freya_extra::{dropdown_menu::DropdownMenu, floating_window::FloatingWindow};

use crate::{
    actions,
    state::AppState,
    ui::{PathRow, check_tile, heading, path_row, row, with_tooltip},
};

const LABEL_WIDTH: f32 = 160.0;

/// Text that opens `url` in the browser when pressed.
pub(crate) fn hyperlink(text: impl Into<String>, url: impl Into<String>) -> Element {
    let text: String = text.into();
    let url: String = url.into();

    with_tooltip(
        rect()
            .cursor(CursorIcon::Pointer)
            .on_press({
                let url = url.clone();
                move |_| Platform::get().open_url(url.clone())
            })
            .child(label().text(text).color(Color::from_rgb(100, 170, 255))),
        url,
    )
}

#[derive(PartialEq)]
pub(crate) struct HelpWindow;

impl Component for HelpWindow {
    fn render(&self) -> impl IntoElement {
        let app = use_consume::<AppState>();
        let is_open = *app.show_help.read();
        let window_size = Platform::get().current_window().root_size;
        let (width, height) = {
            let size = window_size.read();
            (size.width, size.height)
        };

        let body: Vec<Element> = if is_open {
            vec![
                rect()
                    .width(Size::fill())
                    .main_align(Alignment::Center)
                    .cross_align(Alignment::Center)
                    .child(heading(crate::APP_TITLE))
                    .into_element(),
                help_info(&app),
                bug_report(&app),
                FontSection.into_element(),
                log_section(&app),
                translation_section(&app),
                background_section(&app),
            ]
        } else {
            vec![]
        };

        let window_width = (width * 0.6).max(600.0);
        FloatingWindow::new(app.t(I18nKey::HelpButton))
            .open(is_open)
            .on_close(move |()| {
                let mut show_help = app.show_help;
                show_help.set(false);
            })
            .initial_position((width - window_width).max(0.0) / 2.0, height * 0.1)
            .width(window_width)
            .body_height((height * 0.7).max(300.0))
            .child(
                ScrollView::new()
                    .width(Size::fill())
                    .height(Size::fill())
                    .spacing(12.0)
                    .children(body),
            )
    }
}

fn help_info(app: &AppState) -> Element {
    let app = *app;
    let rows: [(I18nKey, &str, Option<&str>); 5] = [
        (I18nKey::AuthorLabel, env!("CARGO_PKG_AUTHORS"), None),
        (
            I18nKey::LicenseLabel,
            env!("CARGO_PKG_LICENSE"),
            Some(concat!(
                env!("CARGO_PKG_REPOSITORY"),
                "/blob/",
                env!("CARGO_PKG_VERSION"),
                "/LICENSE"
            )),
        ),
        (
            I18nKey::SourceCodeLabel,
            "GitHub",
            Some(concat!(env!("CARGO_PKG_REPOSITORY"), "/tree/", env!("CARGO_PKG_VERSION"))),
        ),
        (
            I18nKey::ChangeLogLabel,
            "CHANGELOG.md",
            Some(concat!(env!("CARGO_PKG_REPOSITORY"), "/blob/main/CHANGELOG.md")),
        ),
        (
            I18nKey::ModTestStatusLabel,
            "test_status.md",
            Some(concat!(
                env!("CARGO_PKG_REPOSITORY"),
                "/blob/",
                env!("CARGO_PKG_VERSION"),
                "/docs/test_status.md"
            )),
        ),
    ];

    rows.into_iter()
        .fold(rect().spacing(6.0), |column, (key, value, url)| {
            let value = match url {
                Some(url) => hyperlink(value, url),
                None => label().text(value).into_element(),
            };
            column.child(
                row().child(label().text(app.t(key)).width(Size::px(LABEL_WIDTH))).child(value),
            )
        })
        .into_element()
}

fn bug_report(app: &AppState) -> Element {
    let app = *app;
    const ISSUE_URL: &str = concat!(env!("CARGO_PKG_REPOSITORY"), "/issues");

    rect()
        .spacing(6.0)
        .child(label().text(app.t(I18nKey::BugReportTitleLabel)))
        .child(
            row()
                .child(with_tooltip(
                    Button::new()
                        .on_press(move |_| {
                            let url = app.settings.peek().create_issue_link();
                            Platform::get().open_url(url);
                        })
                        .child(app.t(I18nKey::IssueReportButton)),
                    app.t(I18nKey::IssueReportHover),
                ))
                .child(hyperlink(app.t(I18nKey::BugReportSeeIssues), ISSUE_URL)),
        )
        .into_element()
}

/// Reload / Clear buttons of a path row.
fn reload_and_clear(
    app: &AppState,
    reload_hover: I18nKey,
    on_reload: impl FnMut(Event<PressEventData>) + 'static,
    mut value: Writable<String>,
) -> [Element; 2] {
    let app = *app;
    [
        with_tooltip(
            Button::new().on_press(on_reload).child(app.t(I18nKey::ReloadButton)),
            app.t(reload_hover),
        ),
        Button::new()
            .on_press(move |_| value.write().clear())
            .child(app.t(I18nKey::ClearButton))
            .into_element(),
    ]
}

/// Opens a picker starting at the parent directory of the current value, and writes the selection.
fn select_button(
    app: &AppState,
    mut value: Writable<String>,
    pick: impl Fn(&Path) -> Option<std::path::PathBuf> + 'static,
    mut on_selected: impl FnMut() + 'static,
) -> Element {
    let app = *app;
    Button::new()
        .on_press(move |_| {
            let current = value.peek().clone();
            let dir = actions::parent_dir_or(&current, ".");
            if let Some(path) = pick(&dir) {
                *value.write() = path.display().to_string();
                on_selected();
            }
        })
        .child(app.t(I18nKey::SelectButton))
        .into_element()
}

#[derive(PartialEq)]
struct FontSection;

impl Component for FontSection {
    fn render(&self) -> impl IntoElement {
        let app = use_consume::<AppState>();
        let search = use_state(String::new);
        let font = app.settings.read().ui.font.clone();

        let mode_select = DropdownMenu::new(font.mode)
            .item(FontMode::Default, app.t(I18nKey::FontModeDefault))
            .item(FontMode::System, app.t(I18nKey::FontModeSystem))
            .item(FontMode::File, app.t(I18nKey::FontModeFile))
            .on_change(move |next: FontMode| {
                let mut settings = app.settings;
                settings.write().ui.font.mode = next;
                // NOTE: The font file is applied by the reload button or by selecting a file.
                if matches!(next, FontMode::Default | FontMode::System) {
                    actions::reload_font(&app);
                }
            });

        let mut section = rect().spacing(6.0).child(
            row()
                .child(with_tooltip(
                    label().text(app.t(I18nKey::FontModeLabel)).width(Size::px(LABEL_WIDTH)),
                    app.t(I18nKey::FontModeHover),
                ))
                .child(mode_select),
        );

        match font.mode {
            FontMode::Default => {}

            FontMode::System => {
                let query = search.read().trim().to_lowercase();
                let candidates = crate::fonts::font_families()
                    .iter()
                    .filter(|family| query.is_empty() || family.to_lowercase().contains(&query))
                    .take(12)
                    .map(|family| {
                        let family = family.clone();
                        Button::new()
                            .compact()
                            .on_press({
                                let family = family.clone();
                                move |_| {
                                    let mut settings = app.settings;
                                    settings.write().ui.font.name = family.clone();
                                    actions::reload_font(&app);
                                }
                            })
                            .child(label().text(family.clone()).font_family(family))
                            .into_element()
                    });

                section = section
                    .child(
                        row()
                            .child(
                                label()
                                    .text(app.t(I18nKey::FontFamily))
                                    .width(Size::px(LABEL_WIDTH)),
                            )
                            .child(label().text(font.name).font_weight(FontWeight::BOLD))
                            .child(
                                Input::new(search)
                                    .placeholder(format!("{}...", app.t(I18nKey::SearchLabel)))
                                    .width(Size::px(240.0)),
                            ),
                    )
                    .child(rect().horizontal().spacing(4.0).children(candidates));
            }

            FontMode::File => {
                let value = Writable::from_state(app.settings)
                    .map(|s| &s.ui.font.path, |s| &mut s.ui.font.path);

                let pick = |dir: &Path| {
                    // NOTE: Fonts are usually here when nothing has been selected yet.
                    let dir =
                        if dir == Path::new(".") { Path::new("C:/Windows/Fonts") } else { dir };
                    rfd::FileDialog::new()
                        .set_directory(dir)
                        .add_filter("font", &["ttc", "ttf", "otf"])
                        .pick_file()
                };

                let mut trailing = vec![select_button(&app, value.clone(), pick, move || {
                    actions::reload_font(&app);
                })];
                trailing.extend(reload_and_clear(
                    &app,
                    I18nKey::FontReloadHover,
                    move |_| actions::reload_font(&app),
                    value.clone(),
                ));

                section = section.child(path_row(
                    PathRow {
                        label: app.t(I18nKey::FontFileLabel),
                        value,
                        placeholder: None,
                        label_width: LABEL_WIDTH,
                    },
                    move |err| actions::notify_error(&app, err),
                    trailing,
                ));
            }
        }

        section
    }
}

fn log_section(app: &AppState) -> Element {
    let app = *app;
    let value =
        Writable::from_state(app.settings).map(|s| &s.log.dir_path, |s| &mut s.log.dir_path);

    let pick = |dir: &Path| {
        let dir = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
        rfd::FileDialog::new().set_directory(dir).pick_folder()
    };

    // NOTE: For a directory, start the picker at the directory itself rather than its parent.
    let select = Button::new()
        .on_press({
            let mut value = value.clone();
            move |_| {
                let current = value.peek().clone();
                if let Some(path) = pick(Path::new(&current)) {
                    *value.write() = path.display().to_string();
                    actions::reload_log(&app);
                }
            }
        })
        .child(app.t(I18nKey::SelectButton))
        .into_element();

    let mut trailing = vec![select];
    trailing.extend(reload_and_clear(
        &app,
        I18nKey::LogReloadHover,
        move |_| actions::reload_log(&app),
        value.clone(),
    ));

    path_row(
        PathRow {
            label: app.t(I18nKey::LogDirPathLabel),
            value,
            placeholder: None,
            label_width: LABEL_WIDTH,
        },
        move |err| actions::notify_error(&app, err),
        trailing,
    )
    .into_element()
}

fn translation_section(app: &AppState) -> Element {
    let app = *app;
    let value =
        Writable::from_state(app.settings).map(|s| &s.ui.i18n_path, |s| &mut s.ui.i18n_path);

    let pick = |dir: &Path| {
        rfd::FileDialog::new().set_directory(dir).add_filter("translation", &["json"]).pick_file()
    };

    let mut trailing =
        vec![select_button(&app, value.clone(), pick, move || actions::reload_i18n(&app))];
    trailing.extend(reload_and_clear(
        &app,
        I18nKey::I18nReloadJsonHover,
        move |_| actions::reload_i18n(&app),
        value.clone(),
    ));

    rect()
        .spacing(6.0)
        .child(path_row(
            PathRow {
                label: app.t(I18nKey::I18nPathLabel),
                value,
                placeholder: None,
                label_width: LABEL_WIDTH,
            },
            move |err| actions::notify_error(&app, err),
            trailing,
        ))
        .child(
            row()
                .child(label().text("English:").width(Size::px(LABEL_WIDTH)))
                .child(with_tooltip(
                    Button::new()
                        .on_press(move |_| actions::write_new_i18n(&app))
                        .child(app.t(I18nKey::I18nWriteNewJsonButton)),
                    app.t(I18nKey::I18nWriteNewJsonHover),
                ))
                .child(with_tooltip(
                    Button::new()
                        .on_press(move |_| actions::load_default_i18n(&app))
                        .child("Default"),
                    "Temporarily force a switch to English mode (for debugging)",
                )),
        )
        .into_element()
}

fn background_section(app: &AppState) -> Element {
    let app = *app;
    let enabled = app.settings.read().ui.background_image.enabled;
    let value = Writable::from_state(app.settings)
        .map(|s| &s.ui.background_image.path, |s| &mut s.ui.background_image.path);

    let pick = |dir: &Path| {
        rfd::FileDialog::new()
            .set_directory(dir)
            .add_filter("image", &["png", "jpg", "jpeg"])
            .pick_file()
    };

    let mut trailing =
        vec![select_button(&app, value.clone(), pick, move || actions::reload_background(&app))];
    trailing.extend(reload_and_clear(
        &app,
        I18nKey::BackgroundImageReloadHover,
        move |_| actions::reload_background(&app),
        value.clone(),
    ));

    rect()
        .spacing(6.0)
        .child(path_row(
            PathRow {
                label: app.t(I18nKey::BackgroundImageLabel),
                value,
                placeholder: None,
                label_width: LABEL_WIDTH,
            },
            move |err| actions::notify_error(&app, err),
            trailing,
        ))
        .child(with_tooltip(
            check_tile(enabled, app.t(I18nKey::BackgroundImageEnabled), move |()| {
                let mut settings = app.settings;
                settings.write().ui.background_image.enabled ^= true;
            }),
            app.t(I18nKey::BackgroundImageEnabledHover),
        ))
        .into_element()
}
