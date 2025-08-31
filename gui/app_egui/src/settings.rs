use serde::{Deserialize, Serialize};
use snafu::ResultExt as _;
use std::{
    borrow::Cow,
    fs,
    path::{Path, PathBuf},
};

use crate::{
    app::ModManagerApp,
    i18n::I18nKey,
    mod_item::{ModItem, SortColumn},
};

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    mode: crate::app::DataMode,
    target_runtime: skyrim_data_dir::Runtime,
    /// The directory containing the HKX templates you want to patch.
    ///
    /// Typically this is a directory like `assets/templates`. The actual patch target directory
    /// should be a subdirectory such as `assets/templates/meshes`.
    pub template_dir: String,
    pub output_dir: String,

    /// Delete <output dir>/meshes immediately before running the patch.
    auto_remove_meshes: bool,
    /// Output d merge patches & merged json files.(To <Output dir>/.d_merge/patches/.debug)
    enable_debug_output: bool,
    log_level: crate::app::LogLevel,
    pub filter_text: String,
    pub font_path: Option<PathBuf>,
    pub i18n: std::collections::HashMap<I18nKey, Cow<'static, str>>,
    pub sort_asc: bool,
    pub sort_column: SortColumn,
    pub transparent: bool,
    pub window_height: f32,
    pub window_maximized: bool,
    pub window_pos_x: f32,
    pub window_pos_y: f32,
    pub window_width: f32,

    pub vfs_skyrim_data_dir: String,
    pub vfs_mod_list: Vec<ModItem>,

    pub skyrim_data_dir: String,
    pub mod_list: Vec<ModItem>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            auto_remove_meshes: false, // If an incorrect output directory(e.g. skyrim data dir) is specified, it is dangerous, so false.
            enable_debug_output: false,
            filter_text: String::new(),
            font_path: None,
            i18n: I18nKey::default_map(),
            log_level: crate::app::LogLevel::Debug,
            mode: crate::app::DataMode::Vfs,
            output_dir: "./d_merge_output".into(),
            sort_asc: true,
            sort_column: SortColumn::Priority,
            target_runtime: skyrim_data_dir::Runtime::Se,
            template_dir: "./assets/templates".into(),
            transparent: false, // For white there, visibility becomes poor, so the default is off.
            window_height: 900.0,
            window_maximized: false,
            window_pos_x: 0.0,
            window_pos_y: 0.0,
            window_width: 900.0,

            vfs_skyrim_data_dir: String::new(),
            vfs_mod_list: Vec::new(),
            skyrim_data_dir: String::new(),
            mod_list: Vec::new(),
        }
    }
}

impl From<ModManagerApp> for AppSettings {
    fn from(app: ModManagerApp) -> Self {
        let i18n = if app.i18n.is_empty() {
            I18nKey::default_map()
        } else {
            app.i18n
        };

        Self {
            vfs_skyrim_data_dir: app.vfs_skyrim_data_dir,
            vfs_mod_list: app.vfs_mod_list,

            skyrim_data_dir: app.skyrim_data_dir,
            mod_list: app.mod_list,

            auto_remove_meshes: app.auto_remove_meshes,
            enable_debug_output: app.enable_debug_output,
            filter_text: app.filter_text,
            font_path: app.font_path,
            i18n,
            log_level: app.log_level,
            mode: app.mode,
            output_dir: app.output_dir,
            sort_asc: app.sort_asc,
            sort_column: app.sort_column,
            target_runtime: app.target_runtime,
            template_dir: app.template_dir,
            transparent: app.transparent,
            window_height: app.last_window_size.y,
            window_maximized: app.last_window_maximized,
            window_pos_x: app.last_window_pos.x,
            window_pos_y: app.last_window_pos.y,
            window_width: app.last_window_size.x,
        }
    }
}
impl From<AppSettings> for ModManagerApp {
    fn from(settings: AppSettings) -> Self {
        Self {
            mode: settings.mode,
            target_runtime: settings.target_runtime,
            vfs_skyrim_data_dir: settings.vfs_skyrim_data_dir,
            vfs_mod_list: settings.vfs_mod_list,

            skyrim_data_dir: settings.skyrim_data_dir,
            mod_list: settings.mod_list,

            template_dir: settings.template_dir,
            output_dir: settings.output_dir,
            enable_debug_output: settings.enable_debug_output,
            auto_remove_meshes: settings.auto_remove_meshes,
            filter_text: settings.filter_text,
            sort_column: settings.sort_column,
            sort_asc: settings.sort_asc,
            i18n: settings.i18n,
            log_level: settings.log_level,
            transparent: settings.transparent,
            last_window_size: egui::vec2(settings.window_width, settings.window_height),
            last_window_pos: egui::pos2(settings.window_pos_x, settings.window_pos_y),
            last_window_maximized: settings.window_maximized,
            font_path: settings.font_path,
            ..Default::default()
        }
    }
}

impl AppSettings {
    /// By placing settings in a fixed location within the Skyrim Data directory, you can handle switching between profiles in MO2.
    const FILE: &'static str = "./.d_merge/d_merge_settings.json";

    /// Load settings from JSON file
    pub fn load() -> Result<Self, SettingsError> {
        if Path::new(Self::FILE).exists() {
            let text = fs::read_to_string(Self::FILE).with_context(|_| IoSnafu {
                path: Path::new(Self::FILE),
            })?;
            serde_json::from_str(&text).with_context(|_| JsonSnafu)
        } else {
            Ok(Self::default())
        }
    }

    /// Save settings to JSON file
    pub fn save(&self) {
        if let Ok(text) = serde_json::to_string_pretty(self) {
            if let Err(err) = fs::write(Self::FILE, text) {
                tracing::error!("Failed to save settings: {err}");
            };
            tracing::info!("Settings saved to {}", Self::FILE);
        }
    }
}

#[derive(Debug, snafu::Snafu)]
pub enum SettingsError {
    #[snafu(display("Failed to read file `{}`: {source}", path.display()))]
    Io {
        source: std::io::Error,
        path: std::path::PathBuf,
    },

    #[snafu(display("Failed to parse JSON: {source}"))]
    Json { source: serde_json::Error },
}
