use std::{
    borrow::Cow,
    fs,
    path::{Path, PathBuf},
};

use semver::Version;
use serde::{Deserialize, Serialize};
use snafu::ResultExt as _;

use crate::{
    app::DataMode,
    mod_item::{ModItem, SortColumn},
};

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct AppSettings {
    // ---------------------------------------------------------------------
    // Metadata / migration
    // ---------------------------------------------------------------------
    //
    /// Settings schema version, written as `env!("CARGO_PKG_VERSION")` on save.
    ///
    /// Used to drive forward migrations in settings loading.
    /// Absent in JSON (legacy) -> treated as `"0.0.0"`.
    pub app_version: Version,

    // ---------------------------------------------------------------------
    // Global behavior
    // ---------------------------------------------------------------------
    //
    /// Execution mode: `vfs` (MO2 VFS) or `manual` (explicit mod directory).
    pub mode: crate::app::DataMode,

    /// Target Skyrim runtime for behavior generation (`le`, `se`, or `vr`).
    pub target_runtime: skyrim_data_dir::Runtime,

    /// When true, enables all mods and runs the patch automatically whenever the mod list is refreshed.
    pub auto_run: bool,

    /// Delete `<output_dir>/meshes` immediately before running the patch.
    ///
    /// Skipped automatically when `output_dir` equals the Skyrim data directory,
    /// to prevent accidental destruction of installed mods.
    pub auto_remove_meshes: bool,

    /// Output d merge patches & merged json files.(To <Output dir>/.d_merge/patches/.debug)
    pub enable_debug_output: bool,

    /// If true, generates a FNIS.esp (dummy ESP) file with the correct version and author information.
    pub generate_fnis_esp: bool,

    /// Minimum log severity level written to the rotating log file.
    pub log_level: crate::app::LogLevel,

    // ---------------------------------------------------------------------
    // UI / appearance
    // ---------------------------------------------------------------------
    //
    /// The user's theme preference(`dark`, `light`, or `system`)
    pub theme: Theme,

    /// Whether the main window background is transparent.
    pub transparent: bool,

    /// Path to a custom `.ttf`/`.otf` font file used for the UI.
    ///
    /// `None` falls back to the built-in font.
    pub font_path: Option<PathBuf>,

    /// Path to the i18n JSON file used for UI language translation.
    pub i18n_path: Cow<'static, str>,

    // ---------------------------------------------------------------------
    // Mod list UI state
    // ---------------------------------------------------------------------
    //
    /// The current text in the mod-list search box, persisted so the filter survives restarts.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub filter_text: String,

    /// Which mod-list column is used as the primary filter key.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter_column: Option<SortColumn>,

    /// Which mod-list column is used as the primary sort key.
    pub sort_column: SortColumn,

    /// Whether the active sort column is sorted ascending (`true`) or descending (`false`).
    pub sort_asc: bool,

    // ---------------------------------------------------------------------
    // Window state
    // ---------------------------------------------------------------------
    //
    /// Horizontal position (x) of the window's top-left corner in logical pixels.
    pub window_pos_x: f32,

    /// Vertical position (y) of the window's top-left corner in logical pixels.
    pub window_pos_y: f32,

    /// Inner width of the window in logical pixels.
    pub window_width: f32,

    /// Inner height of the window in logical pixels (excludes title bar and OS decorations).
    pub window_height: f32,

    /// Whether the window was maximized when the application last closed.
    pub window_maximized: bool,

    // ---------------------------------------------------------------------
    // Shared paths
    // ---------------------------------------------------------------------
    //
    /// The directory containing the HKX templates you want to patch.
    ///
    /// Typically this is a directory like `assets/templates`. The actual patch target directory
    /// should be a subdirectory such as `assets/templates/meshes`.
    pub template_dir: String,

    // ---------------------------------------------------------------------
    // VFS mode
    // ---------------------------------------------------------------------
    //
    /// Skyrim data directory resolved via the MO2 virtual file system.
    ///
    /// On Windows this is auto-detected from the Steam registry; on other platforms
    /// the user must set it manually.
    pub vfs_skyrim_data_dir: String,

    /// Output directory for generated behavior files and logs.
    pub vfs_output_dir: String,

    // ---------------------------------------------------------------------
    // Manual mode
    // ---------------------------------------------------------------------
    //
    /// Skyrim data directory for manual (non-VFS) mode.
    ///
    /// Must point to the directory that directly contains `meshes/`, `scripts/`, etc.
    pub skyrim_data_dir: String,

    /// Output directory for generated behavior files (manual mode).
    pub output_dir: String,

    // ---------------------------------------------------------------------
    // Heavy data (put last)
    // ---------------------------------------------------------------------
    //
    /// Mod list used in VFS mode.
    ///
    /// Each entry's ID is the bare Nemesis mod ID (e.g. `aaaa`), so this list
    /// is portable across machines.
    pub vfs_mod_list: Vec<ModItem>,

    /// Mod list used in manual mode.
    ///
    /// Each entry's ID is the absolute path up to the Nemesis mod ID directory,
    /// so entries may not be portable across machines with different drive layouts.
    pub mod_list: Vec<ModItem>,
}

/// Theme color
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    /// System default: follows the OS theme setting. This is the default if `theme` is `None`.
    System,

    /// Dark background.
    Dark,

    /// Light background.
    Light,
}
impl Theme {
    pub(crate) const fn as_str(&self) -> &'static str {
        match self {
            Self::System => "💻 System",
            Self::Dark => "🌙 Dark",
            Self::Light => "☀ Light",
        }
    }
}
impl From<Theme> for egui::ThemePreference {
    fn from(theme: Theme) -> Self {
        match theme {
            Theme::System => Self::System,
            Theme::Dark => Self::Dark,
            Theme::Light => Self::Light,
        }
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            app_version: Self::default_version(),
            auto_remove_meshes: false, // If an incorrect output directory(e.g. skyrim data dir) is specified, it is dangerous, so false.
            auto_run: false,
            enable_debug_output: false,
            filter_column: None,
            filter_text: String::new(),
            font_path: None,
            generate_fnis_esp: false,
            i18n_path: crate::i18n::I18nMap::FILE.into(),
            log_level: crate::app::LogLevel::Debug,
            mode: crate::app::DataMode::Vfs,
            sort_asc: true,
            sort_column: SortColumn::Priority,
            target_runtime: skyrim_data_dir::Runtime::Se,
            template_dir: "./assets/templates".into(),
            theme: Theme::System,
            transparent: false, // For light theme, visibility becomes poor, so the default is off.
            window_height: 900.0,
            window_maximized: false,
            window_pos_x: 0.0,
            window_pos_y: 0.0,
            window_width: 900.0,

            vfs_skyrim_data_dir: String::new(),
            vfs_mod_list: Vec::new(),
            vfs_output_dir: String::new(), // NOTE: For old compatibility, default to empty. Migrated to output_dir on load if still empty.
            skyrim_data_dir: String::new(),
            output_dir: "./d_merge_output".into(),
            mod_list: Vec::new(),
        }
    }
}

impl AppSettings {
    /// By placing settings in a fixed location within the Skyrim Data directory, you can handle switching between profiles in MO2.
    const FILE: &'static str = "./.d_merge/d_merge_settings.json";

    /// Returns the default version for legacy settings files. `0.0.0`
    const fn default_version() -> Version {
        Version::new(0, 0, 0)
    }

    fn migrate(&mut self) {
        // <=1.8.0: vfs_output_dir didn't exist -> seed from output_dir
        if self.vfs_output_dir.is_empty() {
            self.vfs_output_dir = self.output_dir.clone();
        }

        // Always stamp the current version so next run skips all migrations.
        self.app_version =
            Version::parse(env!("CARGO_PKG_VERSION")).unwrap_or(Self::default_version());
    }

    /// Load settings from JSON file
    pub(crate) fn load() -> Result<Self, SettingsError> {
        let path: &Path = Path::new(Self::FILE);

        if path.exists() {
            let text = fs::read_to_string(path).with_context(|_| IoSnafu { path })?;
            let mut settings: Self = sonic_rs::from_str(&text).with_context(|_| JsonSnafu)?;
            settings.migrate();
            Ok(settings)
        } else {
            Ok(Self::default())
        }
    }

    /// Save settings to JSON file
    pub(crate) fn save(&self) {
        match sonic_rs::to_string_pretty(self) {
            Ok(text) => {
                if let Err(err) = fs::write(Self::FILE, text) {
                    tracing::error!("Failed to save settings: {err}");
                };
                tracing::info!("Settings saved to {}", Self::FILE);
            }
            Err(err) => {
                tracing::error!("Failed to parse settings as JSON: {err}");
            }
        }
    }

    /// Returns the output directory for the current mode.
    ///
    /// - vfs  -> `vfs_output_dir`
    /// - manual -> `output_dir`
    pub(crate) fn current_output_dir(&self) -> &str {
        match self.mode {
            DataMode::Vfs => &self.vfs_output_dir,
            DataMode::Manual => &self.output_dir,
        }
    }

    /// Returns a mutable reference to the output directory for the current mode.
    ///
    /// - vfs  -> `vfs_output_dir`
    /// - manual -> `output_dir`
    pub(crate) const fn current_output_dir_mut(&mut self) -> &mut String {
        match self.mode {
            DataMode::Vfs => &mut self.vfs_output_dir,
            DataMode::Manual => &mut self.output_dir,
        }
    }

    /// - vfs -> self.vfs_skyrim_data_dir
    /// - others -> self.skyrim_data_dir
    pub(crate) const fn current_skyrim_data_dir(&self) -> &str {
        match self.mode {
            DataMode::Vfs => self.vfs_skyrim_data_dir.as_str(),
            DataMode::Manual => self.skyrim_data_dir.as_str(),
        }
    }

    /// - vfs -> vfs_mod_list
    /// - others -> mod_list
    pub(crate) const fn mod_list(&self) -> &Vec<ModItem> {
        match self.mode {
            DataMode::Vfs => &self.vfs_mod_list,
            DataMode::Manual => &self.mod_list,
        }
    }

    /// - vfs -> vfs_mod_list
    /// - others -> mod_list
    pub(crate) const fn mod_list_mut(&mut self) -> &mut Vec<ModItem> {
        match self.mode {
            DataMode::Vfs => &mut self.vfs_mod_list,
            DataMode::Manual => &mut self.mod_list,
        }
    }

    pub(crate) fn create_issue_link(&self) -> String {
        use std::borrow::Cow;

        let skyrim_runtime = match self.target_runtime {
            skyrim_data_dir::Runtime::Le => gh_issue_link::SkyrimRuntime::Le,
            skyrim_data_dir::Runtime::Se => gh_issue_link::SkyrimRuntime::Se,
            skyrim_data_dir::Runtime::Vr => gh_issue_link::SkyrimRuntime::Vr,
        };

        let skyrim_data_dir: Option<Cow<'_, Path>> = if self.vfs_skyrim_data_dir.trim().is_empty() {
            skyrim_data_dir::get_skyrim_data_dir(self.target_runtime).ok().map(Cow::Owned)
        } else {
            Some(Path::new(&self.vfs_skyrim_data_dir).into())
        };

        let skyrim_version = skyrim_data_dir.and_then(|skyrim_data_dir| {
            let exe = match skyrim_runtime {
                gh_issue_link::SkyrimRuntime::Le => "TESV.exe",
                gh_issue_link::SkyrimRuntime::Se => "SkyrimSE.exe",
                gh_issue_link::SkyrimRuntime::Vr => "SkyrimVR.exe",
            };
            let exe_path = skyrim_data_dir.parent()?.join(exe);

            gh_issue_link::version::get_file_version(exe_path).map(|ver| ver.to_string()).ok()
        });
        gh_issue_link::new_gh_issue_link(
            env!("CARGO_PKG_VERSION"),
            skyrim_runtime,
            skyrim_version.as_deref(),
        )
    }
}

#[derive(Debug, snafu::Snafu)]
pub(crate) enum SettingsError {
    #[snafu(display("Failed to read file `{}`: {source}", path.display()))]
    Io { source: std::io::Error, path: std::path::PathBuf },

    #[snafu(display("Failed to parse JSON: {source}"))]
    Json { source: sonic_rs::Error },
}
