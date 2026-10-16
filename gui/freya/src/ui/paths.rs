//! Directory pickers: Skyrim data directory and output directory.
//!
//! ```txt
//! |<---label--->|<------ stretch ------>|<-90->|<-90->|
//! | Skyrim      | path..................| Auto | Select |
//! | Output      | path..................|      | Select |
//! ```

use d_merge_gui_shared::{fs::find_existing_dir_or_ancestor, i18n::I18nKey, settings::DataMode};
use freya::prelude::*;

use crate::{
    actions,
    state::AppState,
    ui::{BUTTON_HEIGHT, PathRow, icon_button, icons, panel, path_row, with_tooltip},
};

const LABEL_WIDTH: f32 = 140.0;
const BUTTON_WIDTH: f32 = 90.0;

#[derive(PartialEq)]
pub(crate) struct PathsPanel;

impl Component for PathsPanel {
    fn render(&self) -> impl IntoElement {
        let app = use_consume::<AppState>();
        let mode = app.settings.read().behavior.mode;

        let skyrim_data_dir = Writable::from_state(app.settings).map(
            |settings| &settings.current_mod_list_settings().skyrim_data_dir,
            |settings| settings.current_skyrim_data_dir_mut(),
        );
        let output_dir = Writable::from_state(app.settings).map(
            |settings| &settings.current_mod_list_settings().output_dir,
            |settings| settings.current_output_dir_mut(),
        );

        // -- Skyrim data dir
        let mut skyrim_buttons = vec![];
        if cfg!(target_os = "windows") && mode == DataMode::Vfs {
            skyrim_buttons.push(with_tooltip(
                icon_button(
                    Some(icons::scan_search()),
                    app.t(I18nKey::AutoDetectButton),
                    (BUTTON_WIDTH, BUTTON_HEIGHT),
                    move |_| {
                        actions::update_vfs_skyrim_data_dir_by_reg(&app);
                    },
                ),
                app.t(I18nKey::AutoDetectHover),
            ));
        }
        if skyrim_buttons.is_empty() {
            // egui: keep the inputs aligned with an empty space.
            skyrim_buttons.push(rect().width(Size::px(BUTTON_WIDTH)).into_element());
        }
        skyrim_buttons.push(
            icon_button(
                Some(icons::folder_search()),
                app.t(I18nKey::SelectButton),
                (BUTTON_WIDTH, BUTTON_HEIGHT),
                move |_| {
                    let current = app.settings.peek().current_skyrim_data_dir().to_owned();
                    let dialog = match find_existing_dir_or_ancestor(current) {
                        Ok(abs_path) => rfd::FileDialog::new().set_directory(abs_path),
                        Err(_) => rfd::FileDialog::new(),
                    };

                    // NOTE: The mod list is refreshed by the effect watching the data directory.
                    if let Some(dir) = dialog.pick_folder() {
                        let mut settings = app.settings;
                        *settings.write().current_skyrim_data_dir_mut() = dir.display().to_string();
                    }
                },
            )
            .into_element(),
        );

        let skyrim_row = path_row(
            PathRow {
                label: app.t(I18nKey::SkyrimDataDirLabel),
                value: skyrim_data_dir,
                placeholder: (mode == DataMode::Manual)
                    .then(|| "D:\\GAME\\ModOrganizer Skyrim SE\\mods\\*".to_string()),
                label_width: LABEL_WIDTH,
            },
            move |err| actions::notify_error(&app, err),
            skyrim_buttons,
        );

        // -- Output dir
        let output_select = icon_button(
            Some(icons::folder_search()),
            app.t(I18nKey::SelectButton),
            (BUTTON_WIDTH, BUTTON_HEIGHT),
            move |_| {
                let current = app.settings.peek().current_output_dir().to_owned();
                let dialog = if current.is_empty() {
                    rfd::FileDialog::new()
                } else {
                    match find_existing_dir_or_ancestor(&current) {
                        Ok(abs_path) => rfd::FileDialog::new().set_directory(abs_path),
                        Err(err) => {
                            actions::notify_error(
                                &app,
                                format!("Couldn't find output dir or ancestor: {err}"),
                            );
                            return;
                        }
                    }
                };

                if let Some(dir) = dialog.pick_folder() {
                    let mut settings = app.settings;
                    *settings.write().current_output_dir_mut() = dir.display().to_string();
                }
            },
        );

        let output_row = path_row(
            PathRow {
                label: app.t(I18nKey::OutputDirLabel),
                value: output_dir,
                placeholder: None,
                label_width: LABEL_WIDTH,
            },
            move |err| actions::notify_error(&app, err),
            [rect().width(Size::px(BUTTON_WIDTH)).into_element(), output_select.into_element()],
        );

        // NOTE: Re-create the inputs when the mode changes, because they are bound to another setting.
        panel().key(mode == DataMode::Vfs).child(skyrim_row).child(output_row)
    }
}
