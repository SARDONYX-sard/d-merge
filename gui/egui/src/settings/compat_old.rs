use std::{borrow::Cow, path::PathBuf};

use crate::{
    log::LogLevel,
    mod_item::{ModItem, SortColumn},
    settings::{
        BehaviorSettings, DataMode, LogSettings, ModListSettings, ModListUiSettings, Settings,
        UiSettings, ui::WindowGeometry,
    },
};

/// Legacy settings schema for loading pre-versioned JSON without a `app_version` field.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub(crate) struct OldSettings {
    mode: DataMode,
    target_runtime: skyrim_data_dir::Runtime,
    /// The directory containing the HKX templates you want to patch.
    ///
    /// Typically this is a directory like `assets/templates`. The actual patch target directory
    /// should be a subdirectory such as `assets/templates/meshes`.
    pub template_dir: Cow<'static, str>,
    pub output_dir: String,

    /// Delete <output dir>/meshes immediately before running the patch.
    auto_remove_meshes: bool,
    /// Output d merge patches & merged json files.(To <Output dir>/.d_merge/patches/.debug)
    enable_debug_output: bool,
    /// If true, generates a FNIS.esp(dummy ESP) file with the correct version and author information.
    pub generate_fnis_esp: bool,

    pub log_level: LogLevel,
    pub auto_run: bool,
    pub transparent: bool,
    pub filter_text: String,
    pub font_path: Option<PathBuf>,
    pub sort_asc: bool,
    pub sort_column: SortColumn,
    pub window_pos_x: f32,
    pub window_pos_y: f32,
    pub window_height: f32,
    pub window_width: f32,
    pub window_maximized: bool,

    pub vfs_skyrim_data_dir: String,
    pub vfs_mod_list: Vec<ModItem>,

    pub skyrim_data_dir: String,
    pub mod_list: Vec<ModItem>,
}

impl Default for OldSettings {
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

// ---------------------------------------------------------------------------
// Migration
// ---------------------------------------------------------------------------
impl From<OldSettings> for Settings {
    fn from(old: OldSettings) -> Self {
        Self {
            app_version: semver::Version::new(1, 0, 0),
            behavior: BehaviorSettings {
                auto_run: old.auto_run,
                target_runtime: old.target_runtime,
                auto_remove_meshes: old.auto_remove_meshes,
                enable_debug_output: old.enable_debug_output,
                generate_fnis_esp: old.generate_fnis_esp,
                template_dir: old.template_dir,
                mode: old.mode,
            },
            ui: UiSettings {
                theme: super::ui::Theme::Dark,
                transparent: old.transparent,
                font_path: old.font_path.map(|p| p.to_string_lossy().into_owned()),
                i18n_path: Default::default(),
                mod_list: ModListUiSettings {
                    filter_text: old.filter_text,
                    filter_column: None,
                    sort_asc: old.sort_asc,
                    sort_column: old.sort_column,
                },
                window: WindowGeometry {
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
