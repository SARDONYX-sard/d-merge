use std::{borrow::Cow, path::PathBuf};

use crate::{
    log::LogLevel,
    settings::{
        BehaviorSettings, DataMode, LogSettings, ModListSettings, PagesSettings, PandoraSupportSettings, Settings,
        ViewState,
        pages::{
            convert::ConvertSettings,
            hkanno::HkannoSettings,
            patch::{PatchSettings, behavior::ParserMode, mod_item::ModItem, mod_list::SortColumn},
            patch_editor::PatchEditorSettings,
        },
        ui::{
            self, BackgroundImageSettings, FontMode,
            theme::{CustomTheme, Theme},
        },
    },
};

/// Legacy settings schema for loading pre-versioned JSON without a `app_version` field.
pub(crate) mod v1 {
    use super::*;

    #[derive(Debug, serde::Serialize, serde::Deserialize)]
    #[serde(default)]
    pub(crate) struct Settings {
        pub(super) mode: DataMode,
        pub(super) target_runtime: skyrim_data_dir::Runtime,
        pub(super) template_dir: Cow<'static, str>,
        pub(super) output_dir: String,

        pub(super) auto_remove_meshes: bool,
        pub(super) enable_debug_output: bool,
        pub(super) generate_fnis_esp: bool,

        pub(super) log_level: LogLevel,
        pub(super) auto_run: bool,
        pub(super) transparent: bool,
        pub(super) filter_text: String,
        pub(super) font_path: Option<PathBuf>,
        pub(super) sort_asc: bool,
        pub(super) sort_column: SortColumn,

        pub(super) window_pos_x: f32,
        pub(super) window_pos_y: f32,
        pub(super) window_height: f32,
        pub(super) window_width: f32,
        pub(super) window_maximized: bool,

        pub(super) vfs_skyrim_data_dir: String,
        pub(super) vfs_mod_list: Vec<ModItem>,

        pub(super) skyrim_data_dir: String,
        pub(super) mod_list: Vec<ModItem>,
    }

    impl Default for Settings {
        fn default() -> Self {
            Self {
                auto_remove_meshes: false, // If an incorrect output directory(e.g. skyrim data dir) is specified, it is dangerous, so false.
                auto_run: false,
                enable_debug_output: false,
                generate_fnis_esp: false,
                filter_text: String::new(),
                font_path: None,
                log_level: LogLevel::Debug,
                mode: DataMode::Vfs,
                output_dir: "./d_merge_output".into(),
                sort_asc: true,
                sort_column: SortColumn::Priority,
                target_runtime: skyrim_data_dir::Runtime::Se,
                template_dir: "./assets/templates".into(),
                transparent: false, // For white theme, visibility becomes poor, so the default is off.
                window_width: 900.0,
                window_height: 900.0,
                window_pos_x: 0.0,
                window_pos_y: 0.0,
                window_maximized: false,

                vfs_skyrim_data_dir: String::new(),
                vfs_mod_list: Vec::new(),
                skyrim_data_dir: String::new(),
                mod_list: Vec::new(),
            }
        }
    }
}

pub(crate) mod v2 {
    use super::*;

    #[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
    #[serde(default)]
    pub(crate) struct UiSettings {
        pub theme: ui::theme::Theme,
        pub custom_theme: CustomTheme,
        pub background_image: BackgroundImageSettings,
        pub font: ui::FontSettings,
        pub i18n_path: String,
        pub mod_list: ViewState,
        pub window: ui::WindowGeometry,
    }

    /// Frozen layout of 2.x (`app_version` present, flat structure).
    #[derive(serde::Deserialize, Default)]
    #[serde(default)]
    #[allow(missing_debug_implementations)]
    pub(crate) struct Settings {
        pub(super) behavior: BehaviorSettings,
        pub(super) ui: UiSettings,
        pub(super) log: LogSettings,
        pub(super) vfs: ModListSettings,
        pub(super) manual: ModListSettings,
    }

    impl From<v1::Settings> for v2::Settings {
        fn from(old: v1::Settings) -> Self {
            Self {
                behavior: BehaviorSettings {
                    mode: old.mode,
                    target_runtime: old.target_runtime,
                    auto_run: old.auto_run,
                    auto_remove_meshes: old.auto_remove_meshes,
                    enable_debug_output: old.enable_debug_output,
                    generate_fnis_esp: old.generate_fnis_esp,
                    template_dir: old.template_dir,

                    // added v2
                    parser_mode: ParserMode::Strict,
                    skeleton_arm_fix: false,
                    apply_user_patch: true,
                },
                ui: UiSettings {
                    theme: Theme::Dark,
                    custom_theme: CustomTheme::default(),
                    background_image: BackgroundImageSettings::default(),
                    font: old
                        .font_path
                        .map(|path| ui::FontSettings {
                            mode: FontMode::File,
                            name: String::new(),
                            path: path.to_string_lossy().to_string(),
                        })
                        .unwrap_or_default(),
                    i18n_path: Default::default(),
                    mod_list: ViewState {
                        filter_text: old.filter_text,
                        filter_column: None,
                        sort_asc: old.sort_asc,
                        sort_column: old.sort_column,
                    },
                    window: ui::WindowGeometry {
                        pos_x: old.window_pos_x,
                        pos_y: old.window_pos_y,
                        height: old.window_height,
                        width: old.window_width,
                        maximized: old.window_maximized,
                    },
                },
                log: LogSettings {
                    dir_path: format!("{}/.d_merge/logs", old.output_dir),
                    level: LogLevel::Debug,
                    ..Default::default()
                },
                vfs: ModListSettings {
                    skyrim_data_dir: old.vfs_skyrim_data_dir,
                    mod_list: old.vfs_mod_list,
                    output_dir: old.output_dir.clone(),
                },
                manual: ModListSettings {
                    skyrim_data_dir: old.skyrim_data_dir,
                    mod_list: old.mod_list,
                    output_dir: old.output_dir,
                },
            }
        }
    }
}

impl From<v2::Settings> for Settings {
    fn from(old: v2::Settings) -> Self {
        Self {
            app_version: super::LAYOUT_VERSION,
            ui: ui::UiSettings {
                theme: old.ui.theme,
                custom_theme: old.ui.custom_theme,
                background_image: old.ui.background_image,
                font: old.ui.font,
                i18n_path: old.ui.i18n_path,
                page: ui::Page::default(),
                lsp: super::LspOptions::default(),
                window: old.ui.window,
            },
            log: old.log,
            pages: PagesSettings {
                patch: PatchSettings {
                    behavior: old.behavior,
                    view_state: old.ui.mod_list,
                    pandora: PandoraSupportSettings::default(),
                    vfs: old.vfs,
                    manual: old.manual,
                },
                convert: ConvertSettings::default(),
                hkanno: HkannoSettings::default(),
                patch_editor: PatchEditorSettings::default(),
            },
        }
    }
}
